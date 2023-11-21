use std::ops::Range;

use crate::key::{Key, KeyStorage};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum NodeKind {
    Leaf = 0,
    Node4 = 1,
    Node16 = 2,
    Node48 = 3,
    Node256 = 4,
}

#[derive(Clone, Copy)]
pub struct NodeData {
    pub len: u8,
    pub kind: NodeKind,
    pub free: u8,
}

pub struct NodeHeader<K: Key + ?Sized>(K::Storage);

impl<K: Key + ?Sized> NodeHeader<K> {
    pub fn new(key: &K, range: Range<usize>, kind: NodeKind) -> Self {
        let storage = <K::Storage as KeyStorage<K>>::store(
            key,
            range,
            NodeData {
                len: 0,
                kind,
                free: 0,
            },
        );
        NodeHeader(storage)
    }

    pub fn data(&self) -> &NodeData {
        self.0.data()
    }

    pub fn data_mut(&mut self) -> &mut NodeData {
        self.0.data_mut()
    }

    pub fn storage(&self) -> &K::Storage {
        &self.0
    }

    pub fn storage_mut(&mut self) -> &mut K::Storage {
        &mut self.0
    }
}
