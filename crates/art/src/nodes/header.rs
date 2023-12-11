use core::fmt;
use std::ops::Range;

use crate::key::{Key, KeyStorage};

use super::NodeType;

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

impl<K: Key + ?Sized> fmt::Debug for NodeHeader<K>
where
    K::Storage: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NodeHeader").field(&self.0).finish()
    }
}

impl<K: Key + ?Sized> NodeHeader<K> {
    pub fn new<N: NodeType<Key = K>>(key: &K, range: Range<usize>) -> Self {
        let storage = <K::Storage as KeyStorage<K>>::store(
            key,
            range,
            NodeData {
                len: 0,
                kind: N::KIND,
                free: 0,
            },
        );
        NodeHeader(storage)
    }

    pub fn is<N: NodeType>(&self) -> bool {
        self.0.data().kind == N::KIND
    }

    pub unsafe fn change_type<N: NodeType>(&mut self) {
        self.0.data_mut().kind = N::KIND
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
