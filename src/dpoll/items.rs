use std::{
    collections::{BTreeMap, btree_map::Values},
    sync::{Arc, Mutex},
};

use crate::wrappers::demi;

use super::item::Item;

#[derive(Debug)]
pub struct Items {
    inner: BTreeMap<demi::DemiQd, Arc<Mutex<Item>>>,
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

    pub fn take(&mut self, needle: Item) -> Option<Arc<Mutex<Item>>> {
        let ret = self.inner.remove(&needle.get_qd());
        return ret;
    }

    pub fn get(&mut self, needle: Item) -> Option<Arc<Mutex<Item>>> {
        let ret = self.inner.get(&needle.get_qd()).map(|arc| arc.clone());
        return ret;
    }

    pub fn len(&self) -> usize {
        return self.inner.len();
    }

    pub fn iter(&self) -> Values<'_, demi::DemiQd, Arc<Mutex<Item>>> {
        return self.inner.values();
    }

    pub fn remove(&mut self, needle: &Item) {
        _ = self.inner.remove(&needle.get_qd()).unwrap();
    }
}
