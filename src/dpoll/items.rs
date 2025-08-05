use std::{cell::RefCell, collections::{BTreeMap, btree_map::Values}, sync::{Arc, Mutex}};

use crate::wrappers::demi;

use super::item::Item;

#[derive(Debug)]
pub struct Items {
    inner: BTreeMap<demi::DemiQd, Arc<Mutex<Item>>>
}

impl Items {
    pub fn new() -> Self {
        return Self {
            inner: BTreeMap::new(),
        };
    }

    pub fn insert(&mut self, it: Item) {
        let qd = it.soc.lock().unwrap().soc.qd;
        self.inner.insert(qd, Arc::new(Mutex::new(it)));
    }

    fn wrapped_op<F, T>(func: F, needle: Item) -> T
        where F: FnOnce(&Item) -> T
    {
        let ret = func(&needle);
        std::mem::forget(needle);
        return ret;
    }

    pub fn take(&mut self, needle: Item) -> Option<Arc<Mutex<Item>>> {
        return Self::wrapped_op(|needle| self.inner.remove(&needle.get_qd()), needle);
    }

    pub fn get(&mut self, needle: Item) -> Option<Arc<Mutex<Item>>> {
        return Self::wrapped_op(|needle| self.inner.get(&needle.get_qd()).map(|arc| arc.clone()), needle);
    }

    pub fn len(&self) -> usize {
        return self.inner.len();
    }

    pub fn iter(&self) -> Values<'_, demi::DemiQd, Arc<Mutex<Item>>> {
        return self.inner.values();
    }
}
