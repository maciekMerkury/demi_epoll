#![feature(ptr_as_uninit, linked_list_cursors)]
pub mod bindings;

mod buffer;
mod dpoll;
mod operation;
mod socket;
mod wrappers;
