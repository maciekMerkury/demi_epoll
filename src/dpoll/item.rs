use crate::{shared::Shared, socket::Socket, wrappers::demi};

use super::Event;

#[derive(Debug)]
pub struct Item {
    pub soc: Shared<Socket>,
    pub evs: Event,
    pub data: u64,
    pub on_readylist: bool,
}

impl Item {
    pub fn new(soc: Shared<Socket>, evs: Event, data: u64) -> Self {
        return Self {
            soc,
            evs,
            data,
            on_readylist: false,
        };
    }

    pub fn get_qd(&self) -> demi::DemiQd {
        return self.soc.borrow().soc.qd;
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
