use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use crate::buffer::Buffer;

#[derive(Debug)]
pub struct Shared<T> {
    inner: Rc<RefCell<T>>,
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        return Self {
            inner: self.inner.clone(),
        };
    }
}

impl<T> Shared<T> {
    pub fn new(it: T) -> Self {
        return Self {
            inner: Rc::new(RefCell::new(it)),
        };
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        return self.inner.borrow();
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        return self.inner.borrow_mut();
    }
}

pub type ThreadBuffer<const B: bool, T> = RefCell<Buffer<B, Shared<T>>>;

pub const fn new_thread_buffer<const B: bool, T>() -> ThreadBuffer<B, T> {
    return RefCell::new(Buffer::new());
}
