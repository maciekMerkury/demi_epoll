mod operation;
mod epoll;
mod item;
mod ready_list;
mod items;

use std::{cell::RefCell, collections::{BTreeSet, LinkedList}, convert, mem::MaybeUninit, pin::Pin, rc::Rc, sync::Arc, time::Duration};
use crate::{buffer::{Buffer, Index}, socket::Socket, wrappers::{demi, errno::{PosixError, PosixResult}}};
use bitflags::bitflags;
use libc::{c_int, epoll_event, pthread_sigmask, sigset_t, EPOLLIN, EPOLLOUT, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD, SIG_SETMASK};
use thiserror::Error;

use epoll::Epoll;
use ready_list::ReadyList;
use items::Items;
pub use operation::Operation;
use item::Item;

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
        if !op.idx().is_dpoll() {
            return self.epoll.ctl(op);
        } 
        match op {
            Operation::Add { qd, idx, evs, data } => {
                    self.items.insert(Item {
                        evs,
                        idx,
                        data,
                        on_readylist: false,
                        qd,
                    });
            },
            Operation::Del(qd, index) => {
                let mut it = self.items.take(Item::dummy(qd)).unwrap();
                if it.on_readylist {
                    self.ready_list.remove(&mut it);
                }
            }
            Operation::Mod(qd, index, event) => {
                self.items.get(Item::dummy(qd)).unwrap().borrow_mut().evs = event;
            },
        }

        return Ok(());
    }

    fn wait(&mut self, socs: &mut Buffer<true, Socket>, timeout: Option<Duration>) -> PosixResult<()> {
        let (_, res) = demi::wait_any(self.qtoks.as_slice(), timeout)?;
        let res = res.unwrap();
        let mut item = self.items.get(Item::dummy(res.qd)).unwrap().borrow_mut();
        let soc = socs.get_mut(item.idx).unwrap();
        soc.process_event(res.value.unwrap());
        self.ready_list.push(&mut item);

        return Ok(());
    }

    fn get_and_schedule_events(&mut self, socs: &mut Buffer<true, Socket>) {
        self.qtoks.clear();
        self.qtoks.reserve(self.items.len() * 2);

        let mut added_events = 0;

        let mut list = ReadyList::new();
        for item in self.items.iter() {
            let mut item = item.borrow_mut();
            let soc = socs.get_mut(item.idx).unwrap();
            let ready = soc.available_events(item.evs);
            if !ready.is_empty() && !item.on_readylist {
                list.push(&mut item);
            }
            let evs_to_schedule = item.evs.difference(ready);
            soc.schedule_events(evs_to_schedule, &mut self.qtoks);
        }
        self.ready_list.append(list);
    }

    fn drain_ready_list(&mut self, socs: &mut Buffer<true, Socket>, evs: &mut [MaybeUninit<epoll_event>]) -> usize {
        return self.ready_list.drain(evs.len(), |i, index, data| {
            let events = socs.get(index).unwrap().available_events(Event::all());
            evs[i] = MaybeUninit::new(epoll_event {
                events: events.bits(),
                u64: data,
            });
        });
    }

    pub fn pwait(&mut self, socs: &mut Buffer<true, Socket>, events: &mut [MaybeUninit<epoll_event>], mut timeout: Option<Duration>, sigmask: Option<&sigset_t>) -> PosixResult<usize> {
        let mut old_mask = MaybeUninit::uninit();
        if let Some(mask) = sigmask {
            unsafe {
                assert_eq!(pthread_sigmask(SIG_SETMASK, mask, old_mask.as_mut_ptr()),
                0);
            }
        }

        self.get_and_schedule_events(socs);

        if !self.ready_list.is_empty() {
            timeout = Some(Duration::ZERO);
        }

        match self.wait(socs, timeout) {
            Ok(()) | Err(PosixError::TIMEDOUT) => {},
            Err(e) => return Err(e),
        }

        let mut evs_len = self.drain_ready_list(socs, events);

        // we never have to block here, because if we're here then either
        // a) ready_list is not empty
        // b) demi::wait yielded something
        // c) demi::wait timed out, thus we should not block more
        evs_len += match self.epoll.wait(&mut events[evs_len..], Some(Duration::ZERO)) {
            Ok(len) => len,
            Err(PosixError::TIMEDOUT) => 0,
            Err(e) => return Err(e),
        };

        if evs_len == 0 {
            return Err(PosixError::TIMEDOUT);
        }


        if let Some(mask) = sigmask {
            unsafe {
                assert_eq!(pthread_sigmask(SIG_SETMASK, old_mask.as_ptr(), std::ptr::null_mut()),
                0);
            }
        }

        return Ok(evs_len);
    }
}

fn unwrap_not_timedout<T>(res: PosixResult<T>, zero: T) -> T {
    return match res {
        Ok(r) => r,
        Err(err) => if err == PosixError::TIMEDOUT { zero } else { res.unwrap() },
    };
}


