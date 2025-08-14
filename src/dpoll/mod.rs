mod epoll;
mod item;
mod items;
mod operation;
mod ready_list;

use crate::{
    buffer::{Buffer, Index},
    socket::Socket,
    wrappers::{
        demi,
        errno::{PosixError, PosixResult},
    },
};
use bitflags::bitflags;
use libc::{
    EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD, EPOLLIN, EPOLLOUT, SIG_SETMASK, c_int,
    epoll_event, pthread_sigmask, sigset_t,
};
use log::trace;
use std::{
    cell::RefCell, collections::{BTreeSet, LinkedList}, convert, fs::ReadDir, mem::MaybeUninit, pin::Pin, rc::Rc, sync::{Arc, Mutex}, time::Duration
};
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
    #[error("invalid operation: {:b}", 0)]
    InvalidOp(i32),
}

#[derive(Debug)]
pub struct Dpoll {
    items: Items,

    ready_list: ReadyList,
    qtoks: Vec<demi::QToken>,
    epoll: Epoll,
    counter: usize,
}

impl Dpoll {
    pub fn create(flags: i32) -> PosixResult<Self> {
        return Ok(Self {
            items: Items::new(),
            qtoks: Vec::with_capacity(1024),
            epoll: Epoll::create(flags)?,
            ready_list: ReadyList::new(),
            counter: 0,
        });
    }

    pub fn ctl(&mut self, socs: &Buffer<true, Arc<Mutex<Socket>>>, op: Operation) -> PosixResult<()> {
        if matches!(op, Operation::Add { .. }) {
            self.counter += 1;
        } else if matches!(op, Operation::Del(_, _)) {
            self.counter -= 1;
        }

        if !op.idx().is_dpoll() {
            trace!("non-dpoll ctl op: {op:?}");
            return self.epoll.ctl(op);
        }
        match op {
            Operation::Add { qd, idx, evs, data } => {
                self.items.insert(Item {
                    evs,
                    idx,
                    data,
                    on_readylist: false,
                    soc: socs.get(idx).unwrap().clone(),
                });
            },
            Operation::Del(qd, index) => {
                let it = self.items.take(Item::dummy(qd.unwrap())).unwrap();

                if it.lock().unwrap().on_readylist {
                    self.ready_list.remove(&it);
                }
            },
            Operation::Mod(qd, index, event) => {
                self.items
                    .get(Item::dummy(qd.unwrap()))
                    .unwrap()
                    .lock().unwrap()
                    .evs = event;
            },
        }

        return Ok(());
    }

    fn wait(
        &mut self,
        timeout: Option<Duration>,
    ) -> PosixResult<()> {
        if self.qtoks.is_empty() {
            trace!("there are no qtoks, not going to wait");
            return Ok(());
        }
        let (_, res) = demi::wait_any(self.qtoks.as_slice(), timeout)?;
        let res = res?;
        let mut item = self.items.get(Item::dummy(res.qd)).unwrap();
        item.lock().unwrap().soc.lock().unwrap().process_event(res.value.unwrap());
        self.ready_list.push(item);

        return Ok(());
    }

    fn get_and_schedule_events(&mut self) {
        self.qtoks.clear();
        self.qtoks.reserve(self.items.len() * 2);

        let mut added_events = 0;

        let mut list = ReadyList::new();
        let mut delete_list = ReadyList::new();
        for item in self.items.iter() {
            let lock = item.lock().unwrap();
            let mut soc = lock.soc.lock().unwrap();
            if !soc.open {
                //eprintln!("deleting the cunt");
                delete_list.push(item.clone());
                continue;
            }
            let evs = lock.evs;
            let ready = soc.available_events(evs);

            let evs_to_schedule = evs.difference(ready);
            soc.schedule_events(evs_to_schedule, &mut self.qtoks);

            if !ready.is_empty() && !item.lock().unwrap().on_readylist {
                list.push(item.clone());
            }
        }

        for mutex in delete_list.into_iter().map(|(item, _)| item) {
            let item = mutex.lock().unwrap();

            if item.on_readylist {
                self.ready_list.remove(&mutex);
            }

            self.items.remove(&item);
        }

        self.ready_list.append(list);
    }

    fn drain_ready_list(
        &mut self,
        evs: &mut [MaybeUninit<epoll_event>],
    ) -> usize {
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

        match self.wait(timeout) {
            Ok(()) => {}
            Err(PosixError::TIMEDOUT) => timeout = Some(Duration::ZERO),
            Err(e) => {
                trace!("self.wait failed with {e:?}");
                return Err(e);
            }
        }

        //eprintln!("drain_ready_list");
        let mut evs_len = self.drain_ready_list(events);

        if evs_len > 0 {
            timeout = Some(Duration::ZERO);
        }

        trace!(
            "{epoll:?} going to wait on epoll for {timeout:?}",
            epoll = self.epoll
        );

        //eprintln!("epoll.wait");
        evs_len += match self.epoll.wait(&mut events[evs_len..], timeout) {
            Ok(len) => len,
            Err(PosixError::TIMEDOUT) => 0,
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

