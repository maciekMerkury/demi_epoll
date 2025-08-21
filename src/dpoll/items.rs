use std::collections::{BTreeMap, btree_map::Values};

use crate::{shared::Shared, wrappers::demi};

use super::item::Item;

#[derive(Debug)]
pub struct Items {
    inner: BTreeMap<demi::DemiQd, Shared<Item>>,
}

impl Items {
    pub fn new() -> Self {
        return Self {
            inner: BTreeMap::new(),
        };
    }

    pub fn insert(&mut self, it: Item) {
        let qd = it.get_qd();
        self.inner.insert(qd, Shared::new(it));
    }

    pub fn take(&mut self, qd: demi::DemiQd) -> Option<Shared<Item>> {
        let ret = self.inner.remove(&qd);
        return ret;
    }

    pub fn get(&mut self, qd: demi::DemiQd) -> Option<Shared<Item>> {
        let ret = self.inner.get(&qd).map(|rc| rc.clone());
        return ret;
    }

    pub fn len(&self) -> usize {
        return self.inner.len();
    }

    pub fn iter(&self) -> Values<'_, demi::DemiQd, Shared<Item>> {
        return self.inner.values();
    }

    pub fn remove(&mut self, needle: &Item) {
        _ = self.inner.remove(&needle.get_qd()).unwrap();
    }
}
