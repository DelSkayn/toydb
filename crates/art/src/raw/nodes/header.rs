use super::NodeType;
use crate::key::{Key, KeyStorage};
use core::fmt;
use std::{marker::PhantomData, ops::Range};

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

pub struct NodeHeader<K: Key + ?Sized, V> {
    //pub(crate) parent: Option<NodePtr<Unknown, K, V>>,
    pub(crate) storage: K::Storage,
    _marker: PhantomData<V>,
}

impl<K: Key + ?Sized, V> fmt::Debug for NodeHeader<K, V>
where
    K::Storage: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NodeHeader").field(&self.storage).finish()
    }
}

impl<K: Key + ?Sized, V> NodeHeader<K, V> {
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
        NodeHeader {
            //parent: None,
            storage,
            _marker: PhantomData,
        }
    }

    pub fn kind(&self) -> NodeKind {
        self.storage.data().kind
    }

    pub fn is<N: NodeType>(&self) -> bool {
        self.kind() == N::KIND
    }

    pub unsafe fn change_type<N: NodeType>(&mut self) {
        self.storage.data_mut().kind = N::KIND
    }

    pub fn data(&self) -> &NodeData {
        self.storage.data()
    }

    pub fn data_mut(&mut self) -> &mut NodeData {
        self.storage.data_mut()
    }

    pub fn prefix(&self) -> &[u8] {
        self.storage.prefix()
    }
}
