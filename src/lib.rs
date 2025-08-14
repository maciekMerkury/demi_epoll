#![feature(ptr_as_uninit, linked_list_cursors)]
#![allow(dead_code, unused)]
#[allow(unused)]
pub mod bindings;

mod buffer;
mod dpoll;
mod operation;
mod socket;
mod wrappers;
