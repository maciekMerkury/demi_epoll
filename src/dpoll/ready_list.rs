use std::{cell::RefCell, collections::LinkedList, sync::{Arc, Mutex}, thread::current};

use crate::{buffer::Index, socket::Socket};

use super::item::Item;

#[derive(Debug)]
pub struct ReadyList {
    list: LinkedList<(Arc<Mutex<Item>>, u64)>,
}

impl ReadyList {
    pub fn new() -> Self {
        return Self {
            list: LinkedList::new(),
        };
    }

    pub fn push(&mut self, item: Arc<Mutex<Item>>) {
        let data = {
            let mut item = item.lock().unwrap();
            item.on_readylist = true;
            item.data
        };
        self.list.push_back((item, data));
    }

    pub fn remove(&mut self, item: Arc<Mutex<Item>>) {
        let needle = {
            let mut item = item.lock().unwrap();
            item.on_readylist = false;
            item.get_qd()
        };
        let mut cursor = self.list.cursor_back_mut();

        while let Some(current) = cursor.current() {
            let current = current.0.lock().unwrap().get_qd();
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
        let mut idx = 0;

        while let Some(curr) = self.list.pop_front()
            && idx < max
        {
            let mut item = curr.0.lock().unwrap();
            item.on_readylist = false;
            func(idx, &item.soc.lock().unwrap(), curr.1);
            idx += 1;
        }

        return idx;
    }

    pub fn is_empty(&self) -> bool {
        return self.list.is_empty();
    }
}
