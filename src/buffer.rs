use bitfields::bitfield;
use log::trace;
use std::{default::Default, mem};

pub struct Buffer<const S: bool, T> {
    items: Vec<Entry<T>>,
    next_free: Option<usize>,
}

impl<const S: bool, T> Buffer<S, T> {
    pub const fn new() -> Self {
        return Self {
            items: Vec::new(),
            next_free: None,
        };
    }

    #[allow(dead_code)]
    pub fn new_with_cap(cap: usize) -> Self {
        return Self {
            items: Vec::with_capacity(cap),
            next_free: None,
        };
    }

    pub fn allocate(&mut self, item: T) -> Index {
        let idx = if let Some(i) = self.next_free {
            self.next_free = match self.items[i].field {
                Field::Free(n) => n,
                Field::Item(_) => panic!("an item is on the free list"),
            };

            Index::from_parts(i, self.items[i].generation, S)
        } else {
            self.items.push(Entry::default());
            Index::from_parts(self.items.len() - 1, Generation::ZERO, S)
        };

        self.get_entry_mut(idx).unwrap().field = Field::Item(item);
        return idx;
    }

    pub fn take(&mut self, idx: Index) -> T {
        assert!(idx.is_dpoll());
        let next_free = self.next_free;
        self.next_free = Some(idx.index() as usize);
        let entry = self.get_entry_mut(idx).unwrap();

        assert!(idx.generation() == entry.generation);

        let item = match mem::replace(&mut entry.field, Field::Free(next_free)) {
            Field::Item(it) => it,
            Field::Free(_) => panic!("trying to take an already existing item"),
        };

        return item;
    }

    pub fn free(&mut self, idx: Index) {
        assert!(idx.is_dpoll());
        let next_free = self.next_free;
        let entry = self.get_entry_mut(idx).unwrap();

        if entry.generation != idx.generation() || matches!(entry.field, Field::Free(_)) {
            panic!("trying to double free or free an old item: {idx:?}");
        }

        *entry = Entry {
            generation: entry.generation.next(),
            field: Field::Free(next_free),
        };
        self.next_free = Some(idx.index() as usize);
    }

    pub fn get(&self, idx: Index) -> Option<&T> {
        if !idx.is_dpoll() {
            trace!("{idx:?} is not dpoll");
            return None;
        }
        return match &self.get_entry(idx)?.field {
            Field::Item(it) => Some(it),
            Field::Free(_) => None,
        };
    }

    pub fn get_mut(&mut self, idx: Index) -> Option<&mut T> {
        if !idx.is_dpoll() {
            return None;
        }
        return match &mut self.get_entry_mut(idx)?.field {
            Field::Item(it) => Some(it),
            Field::Free(_) => None,
        };
    }

    fn get_entry(&self, idx: Index) -> Option<&Entry<T>> {
        let entry = &self.items[idx.index() as usize];
        if entry.generation != idx.generation() {
            return None;
        }
        return Some(entry);
    }

    fn get_entry_mut(&mut self, idx: Index) -> Option<&mut Entry<T>> {
        let entry = &mut self.items[idx.index() as usize];
        if entry.generation != idx.generation() {
            return None;
        }
        return Some(entry);
    }
}

#[derive(Debug)]
struct Entry<T> {
    generation: Generation,
    field: Field<T>,
}

impl<T> Default for Entry<T> {
    fn default() -> Self {
        return Self {
            field: Field::default(),
            generation: Generation::default(),
        };
    }
}

#[derive(Debug)]
enum Field<T> {
    Item(T),
    Free(Option<usize>),
}

impl<T> Default for Field<T> {
    fn default() -> Self {
        return Self::Free(None);
    }
}

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Generation(u8);

impl Generation {
    const ZERO: Generation = Generation(0);
    #[inline]
    const fn next(self) -> Self {
        return Self(self.0.wrapping_add(1));
    }

    #[inline]
    const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    #[inline]
    const fn into_bits(self) -> u8 {
        self.0
    }
}

#[bitfield(u32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Index {
    #[bits(21)]
    index: u32,

    #[bits(8)]
    generation: Generation,

    is_socket: bool,

    #[bits(1, default = true, access = ro)]
    is_dpoll: bool,

    #[bits(default = false)]
    _sign: bool,
}

impl Index {
    fn from_parts(index: usize, gene: Generation, is_socket: bool) -> Self {
        return IndexBuilder::new()
            .with_index(index.try_into().unwrap())
            .with_generation(gene)
            .with_is_socket(is_socket)
            .build();
    }
}

impl std::convert::From<i32> for Index {
    fn from(value: i32) -> Self {
        return Self::from_bits(value.try_into().expect("a fd cannot be negative"));
    }
}

impl std::convert::Into<i32> for Index {
    fn into(self) -> i32 {
        return self.into_bits() as i32;
    }
}
