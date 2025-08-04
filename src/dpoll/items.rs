use std::{cell::RefCell, collections::BTreeSet};

use super::item::Item;
use std::collections::btree_set::Iter;

#[derive(Debug)]
pub struct Items {
    inner: BTreeSet<RefCell<Item>>,
}

impl Items {
    pub fn new() -> Self {
        return Self {
            inner: BTreeSet::new(),
        };
    }

    pub fn insert(&mut self, it: Item) {
        self.inner.insert(RefCell::new(it));
    }

    pub fn take(&mut self, needle: Item) -> Option<Item> {
        return self
            .inner
            .take(&RefCell::new(needle))
            .map(|cell| cell.into_inner());
    }

    pub fn get(&mut self, needle: Item) -> Option<&RefCell<Item>> {
        return self.inner.get(&RefCell::new(needle));
    }

    pub fn len(&self) -> usize {
        return self.inner.len();
    }

    pub fn iter(&self) -> Iter<'_, RefCell<Item>> {
        return self.inner.iter();
    }
}
