#![feature(ptr_as_uninit, linked_list_cursors)]

#[allow(unused)]
pub mod bindings;

mod buffer;
mod dpoll;
mod operation;
mod shared;
mod socket;
mod wrappers;
