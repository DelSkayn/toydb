use super::{BoxedNode, Node48, NodePtr};
use crate::{
    header::{NodeHeader, NodeKind},
    key::Key,
};
use std::{mem::MaybeUninit, ops::Range};

#[repr(C)]
pub struct Node256<K: Key, V> {
    pub header: NodeHeader<K>,
    pub ptr: [Option<NodePtr<K, V>>; 256],
}

impl<K: Key, V> Node256<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> BoxedNode<Self> {
        BoxedNode::new(Node256 {
            header: NodeHeader::new(key, range, NodeKind::Node48),
            ptr: unsafe { MaybeUninit::zeroed().assume_init() },
        })
    }

    pub fn is_full(&self) -> bool {
        // slight quirk with len being only u8
        // Can't fit full length so actuall length is self.header.data().len + 1
        self.header.data().len == 255
    }

    pub fn should_shrink(&self) -> bool {
        // slight quirk with len being only u8
        // Can't fit full length so actuall length is self.header.data().len + 1
        self.header.data().len < 47
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut NodePtr<K, V>> {
        self.ptr[key as usize].as_mut()
    }

    pub fn insert(&mut self, key: u8, ptr: NodePtr<K, V>) -> Option<NodePtr<K, V>> {
        let res = self.ptr[key as usize].replace(ptr);
        self.header.data_mut().len += res.is_none() as u8;
        res
    }

    pub fn remove(&mut self, key: u8) -> Option<NodePtr<K, V>> {
        let res = self.ptr[key as usize].take();
        self.header.data_mut().len -= res.is_some() as u8;
        res
    }

    pub fn shrink(this: BoxedNode<Self>) -> BoxedNode<Node48<K, V>> {
        assert!(this.should_shrink());
        todo!()
    }
}
