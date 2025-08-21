mod epoll;
mod item;
mod items;
mod operation;
mod ready_list;

use crate::wrappers::{
    demi,
    errno::{PosixError, PosixResult},
};
use bitflags::bitflags;
use libc::{EPOLLIN, EPOLLOUT, epoll_event};
use log::trace;
use std::{convert, mem::MaybeUninit, time::Duration};
use thiserror::Error;

use epoll::Epoll;
use item::Item;
use items::Items;
pub use operation::Operation;
use ready_list::ReadyList;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Event: u32 {
        const IN = EPOLLIN as u32;
        const OUT = EPOLLOUT as u32;
    }
}

impl convert::TryFrom<u32> for Event {
    type Error = DpollErrors;

    fn try_from(evs: u32) -> Result<Self, Self::Error> {
        match Self::from_bits(evs) {
            Some(evs) => return Ok(evs),
            None => return Err(DpollErrors::InvalidEvent(evs)),
        }
    }
}

#[derive(Debug, Error)]
pub enum DpollErrors {
    #[error("invalid error value: {:b}", 0)]
    InvalidEvent(u32),
}

#[derive(Debug)]
pub struct Dpoll {
    items: Items,

    ready_list: ReadyList,
    qtoks: Vec<demi::QToken>,
    epoll: Epoll,
}

impl Dpoll {
    pub fn create(flags: i32) -> PosixResult<Self> {
        return Ok(Self {
            items: Items::new(),
            qtoks: Vec::with_capacity(1024),
            epoll: Epoll::create(flags)?,
            ready_list: ReadyList::new(),
        });
    }

    pub fn ctl(&mut self, op: Operation) -> PosixResult<()> {
        let op = match op {
            Operation::Epoll(op) => return self.epoll.ctl(op),
            Operation::Dpoll(op) => op,
        };

        match op {
            operation::DpollOperation::Add { soc, evs, data } => {
                self.items.insert(Item::new(soc, evs, data));
            }
            operation::DpollOperation::Del { qd } => {
                let it = self.items.take(qd).unwrap();

                if it.borrow().on_readylist {
                    self.ready_list.remove(&it);
                }
            }
            operation::DpollOperation::Mod { qd, evs } => {
                self.items.get(qd).unwrap().borrow_mut().evs = evs
            }
        }

        return Ok(());
    }

    fn wait(&mut self, timeout: Option<Duration>) -> PosixResult<()> {
        trace!("waiting on {:?}", self.qtoks);
        if self.qtoks.is_empty() {
            trace!("there are no qtoks, not going to wait");
            return Ok(());
        }
        let (_, res) = demi::wait_any(self.qtoks.as_slice(), timeout)?;
        trace!("got {res:?}");
        let res = res.unwrap();
        let item = self.items.get(res.qd).unwrap();
        item.borrow()
            .soc
            .borrow_mut()
            .process_event(res.value.unwrap());
        self.ready_list.push(item);

        return Ok(());
    }

    fn get_and_schedule_events(&mut self) {
        trace!("starting to schedule events");
        self.qtoks.clear();
        self.qtoks.reserve(self.items.len() * 2);

        let mut list = ReadyList::new();
        let mut delete_list = ReadyList::new();

        for item in self.items.iter() {
            let it = item.borrow();
            let mut soc = it.soc.borrow_mut();
            if !soc.open {
                trace!("socket {:?} is not open, adding it to delete_list", soc);
                delete_list.push(item.clone());
                continue;
            }

            let evs = it.evs;
            let ready = soc.available_events(evs);
            let evs_to_schedule = evs.difference(ready);
            soc.schedule_events(evs_to_schedule, &mut self.qtoks);
            if !ready.is_empty() && !it.on_readylist {
                list.push(item.clone());
            }
        }

        for it in delete_list.into_iter().map(|(item, _)| item) {
            let item = it.borrow_mut();

            if item.on_readylist {
                self.ready_list.remove(&it);
            }

            self.items.remove(&item);
        }

        trace!("list: {:?}", list);
        self.ready_list.append(list);
    }

    fn drain_ready_list(&mut self, evs: &mut [MaybeUninit<epoll_event>]) -> usize {
        return self.ready_list.drain(evs.len(), |i, soc, data| {
            let events = soc.available_events(Event::all());
            evs[i] = MaybeUninit::new(epoll_event {
                events: events.bits(),
                u64: data,
            });
        });
    }

    pub fn pwait(
        &mut self,
        events: &mut [MaybeUninit<epoll_event>],
        mut timeout: Option<Duration>,
    ) -> PosixResult<usize> {
        self.get_and_schedule_events();

        if !self.ready_list.is_empty() {
            trace!("ready_list is not empty, only going to poll");
            timeout = Some(Duration::ZERO);
        }

        trace!("going to wait");
        match self.wait(timeout) {
            Ok(()) => {}
            Err(PosixError::TIMEDOUT) => timeout = Some(Duration::ZERO),
            Err(e) => {
                trace!("self.wait failed with {e:?}");
                return Err(e);
            }
        }

        trace!("draining list");
        let mut evs_len = self.drain_ready_list(events);

        if evs_len > 0 {
            timeout = Some(Duration::ZERO);
        }

        trace!(
            "{epoll:?} going to wait on epoll for {timeout:?}",
            epoll = self.epoll
        );

        evs_len += match self.epoll.wait(&mut events[evs_len..], timeout) {
            Ok(len) => len,
            Err(e) => {
                trace!("epoll.wait failed with {e:?}");
                return Err(e);
            }
        };

        if evs_len == 0 {
            trace!("epoll: {self:?} timed out");
            return Err(PosixError::TIMEDOUT);
        }

        return Ok(evs_len);
    }
}
