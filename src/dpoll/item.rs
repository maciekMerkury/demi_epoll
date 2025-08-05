use std::sync::{Arc, Mutex};

use crate::{buffer::Index, socket::Socket, wrappers::demi};

use super::Event;

#[derive(Debug)]
pub struct Item {
    pub soc: Arc<Mutex<Socket>>,
    pub evs: Event,
    pub idx: Index,
    pub data: u64,
    pub on_readylist: bool,
}

impl Item {
    pub fn dummy(qd: demi::DemiQd) -> Self {
        return Self {
            soc: Arc::new(Mutex::new(Socket::new(demi::SocketQd{ qd }))),
            evs: Event::empty(),
            idx: Index::new(),
            data: 0,
            on_readylist: false,
        }
    }

    pub fn get_qd(&self) -> demi::DemiQd {
        return self.soc.lock().unwrap().soc.qd;
    }
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        return self.get_qd().cmp(&other.get_qd());
    }
}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return self.get_qd().partial_cmp(&other.get_qd());
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        return self.get_qd().eq(&other.get_qd());
    }
}

impl Eq for Item {}
