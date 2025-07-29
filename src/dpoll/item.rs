use crate::{buffer::Index, wrappers::demi};

use super::Event;

pub struct Item {
    pub evs: Event,
    pub idx: Index,
    pub data: u64,
    pub on_readylist: bool,
    pub qd: demi::DemiQd,
}

impl Item {
    pub fn dummy(qd: demi::DemiQd) -> Self {
        return Self {
            evs: Event::empty(),
            idx: 0.into(),
            data: 0,
            on_readylist: false,
            qd,
        }
    }
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        return self.qd.cmp(&other.qd);
    }
}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return self.qd.partial_cmp(&other.qd);
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        return self.qd.eq(&other.qd);
    }
}

impl Eq for Item { }

