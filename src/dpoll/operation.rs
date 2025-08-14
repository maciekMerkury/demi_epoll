use libc::{EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD, c_int, epoll_event};
use log::trace;

use crate::{buffer::Index, wrappers::demi};

use super::{DpollErrors, Event};

#[derive(Debug)]
pub enum Operation {
    Add {
        qd: Option<demi::DemiQd>,
        idx: Index,
        evs: Event,
        data: u64,
    },
    Del(Option<demi::DemiQd>, Index),
    Mod(Option<demi::DemiQd>, Index, Event),
}

impl Operation {
    pub fn new(
        idx: Index,
        qd: Option<demi::DemiQd>,
        op: c_int,
        event: Option<&epoll_event>,
    ) -> Result<Self, DpollErrors> {
        trace!("event: {:?}", event);
        match op {
            EPOLL_CTL_ADD => Ok(Self::Add {
                idx,
                qd,
                evs: event.unwrap().events.try_into().unwrap(),
                data: event.unwrap().u64,
            }),
            EPOLL_CTL_DEL => Ok(Self::Del(qd, idx)),
            EPOLL_CTL_MOD => Ok(Self::Mod(
                qd,
                idx,
                event.unwrap().events.try_into().unwrap(),
            )),
            _ => return Err(DpollErrors::InvalidOp(op)),
        }
    }

    pub fn idx(&self) -> Index {
        return match self {
            Operation::Add { idx, .. } | Operation::Del(_, idx) | Operation::Mod(_, idx, _) => *idx,
        };
    }

    pub fn to_raw(self) -> (i32, i32, Option<epoll_event>) {
        let op;
        let fd;
        let event;

        match self {
            Operation::Add { idx, evs, data, .. } => {
                op = EPOLL_CTL_ADD;
                fd = idx.into();
                event = Some(epoll_event {
                    events: evs.bits(),
                    u64: data,
                });
            }
            Operation::Del(_, idx) => {
                op = EPOLL_CTL_DEL;
                fd = idx.into();
                event = None;
            }
            Operation::Mod(_, idx, evs) => {
                op = EPOLL_CTL_MOD;
                fd = idx.into();
                event = Some(epoll_event {
                    events: evs.bits(),
                    u64: 0,
                });
            }
        }

        return (op, fd, event);
    }
}
