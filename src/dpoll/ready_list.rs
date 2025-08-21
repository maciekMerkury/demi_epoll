use std::collections::LinkedList;

use crate::{shared::Shared, socket::Socket};

use super::item::Item;

#[derive(Debug)]
pub struct ReadyList {
    list: LinkedList<(Shared<Item>, u64)>,
}

impl ReadyList {
    pub fn new() -> Self {
        return Self {
            list: LinkedList::new(),
        };
    }

    pub fn push(&mut self, item: Shared<Item>) {
        let data = {
            let mut item = item.borrow_mut();
            if item.on_readylist {
                return;
            }
            item.on_readylist = true;
            item.data
        };
        self.list.push_back((item, data));
    }

    pub fn remove(&mut self, item: &Shared<Item>) {
        let needle = {
            let mut item = item.borrow_mut();
            if !item.on_readylist {
                return;
            }
            item.on_readylist = false;
            item.get_qd()
        };
        let mut cursor = self.list.cursor_back_mut();

        while let Some(current) = cursor.current() {
            let current = current.0.borrow().get_qd();
            if current == needle {
                cursor.remove_current();
                break;
            }
            cursor.move_prev();
        }
    }

    pub fn append(&mut self, mut other: Self) {
        self.list.append(&mut other.list);
    }

    pub fn drain<F>(&mut self, max: usize, mut func: F) -> usize
    where
        F: FnMut(usize, &Socket, u64),
    {
        if self.list.is_empty() {
            return 0;
        }
        let mut idx = 0;

        while let Some(curr) = self.list.pop_front()
            && idx < max
        {
            let mut item = curr.0.borrow_mut();
            item.on_readylist = false;
            func(idx, &item.soc.borrow(), curr.1);
            idx += 1;
        }

        return idx;
    }

    pub fn is_empty(&self) -> bool {
        return self.list.is_empty();
    }

    pub fn into_iter(self) -> std::collections::linked_list::IntoIter<(Shared<Item>, u64)> {
        return self.list.into_iter();
    }
}
