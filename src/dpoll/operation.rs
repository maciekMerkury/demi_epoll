use libc::{EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD, c_int, epoll_event};

use crate::{
    buffer::{Buffer, Index},
    shared::Shared,
    socket::Socket,
    wrappers::demi,
};

use super::Event;

#[allow(private_interfaces)]
#[derive(Debug)]
pub enum Operation {
    Epoll(EpollOperation),
    Dpoll(DpollOperation),
}

#[derive(Debug)]
pub(super) struct EpollOperation {
    pub op: c_int,
    pub fd: c_int,
    pub event: *mut epoll_event,
}

impl Operation {
    pub unsafe fn from_raw(
        socs: &Buffer<true, Shared<Socket>>,
        op: c_int,
        fd: c_int,
        event: *mut epoll_event,
    ) -> Self {
        let idx: Index = fd.into();
        if !idx.is_dpoll() {
            return Self::Epoll(EpollOperation { op, fd, event });
        }

        let event = unsafe { event.as_ref() };
        let soc = socs.get(idx).unwrap().clone();
        return Self::Dpoll(DpollOperation::new(soc, op, event));
    }
}

#[derive(Debug)]
pub(super) enum DpollOperation {
    Add {
        soc: Shared<Socket>,
        evs: Event,
        data: u64,
    },
    Del {
        qd: demi::DemiQd,
    },
    Mod {
        qd: demi::DemiQd,
        evs: Event,
    },
}

impl DpollOperation {
    pub fn new(soc: Shared<Socket>, op: c_int, event: Option<&epoll_event>) -> Self {
        let evs = event.map(|ev| ev.events.try_into().unwrap());
        return match op {
            EPOLL_CTL_ADD => {
                let event = event.unwrap();
                Self::Add {
                    soc,
                    evs: evs.unwrap(),
                    data: event.u64,
                }
            }
            EPOLL_CTL_DEL => Self::Del {
                qd: soc.borrow().soc.qd,
            },
            EPOLL_CTL_MOD => Self::Mod {
                qd: soc.borrow().soc.qd,
                evs: evs.unwrap(),
            },
            _ => panic!("invalid op: {}", op),
        };
    }
}
