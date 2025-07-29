use std::{cell::RefCell, collections::LinkedList};

use crate::buffer::Index;

use super::item::Item;

pub struct ReadyList {
    list: LinkedList<(Index, u64)>
}

impl ReadyList {
    pub fn new() -> Self {
        return Self {
            list: LinkedList::new(),
        }
    }

    pub fn push(&mut self, item: &mut Item) {
        if item.on_readylist {
            return;
        }
        item.on_readylist = true;
        self.list.push_back((item.idx, item.data));
    }

    pub fn remove(&mut self, item: &mut Item) {
        item.on_readylist = false;
        let idx = item.idx;
        let mut cursor = self.list.cursor_back_mut();
        while let Some(current) = cursor.current() {
            if current.0 == idx {
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
        where F: FnMut(usize, Index, u64),
    {
        let mut idx = 0;

        while let Some(curr) = self.list.pop_front() && idx < max {
            func(idx, curr.0, curr.1);
            idx += 1;
        }

        return idx;
    }

    pub fn is_empty(&self) -> bool {
        return self.list.is_empty();
    }
}

