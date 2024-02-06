use super::{NodeHeader, NodeKind, NodeType};
use crate::{
    key::{Key, KeyStorage},
    raw::ptr::{MutValuePtr, OwnedTypedNodePtr, TypedNodePtr, Unknown, ValidPtr},
};
use core::fmt;
use std::{ops::Range, ptr::addr_of_mut};

#[repr(C)]
pub struct LeafNode<K: Key + ?Sized, V> {
    pub header: NodeHeader<K, V>,
    pub value: V,
}

unsafe impl<K: Key + ?Sized, V> NodeType for LeafNode<K, V> {
    const KIND: NodeKind = NodeKind::Leaf;

    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> LeafNode<K, V> {
    pub fn new(key: &K, range: Range<usize>, value: V) -> Self {
        let mut header = NodeHeader::new::<Self>(key, range);
        LeafNode { header, value }
    }
}

impl<K: Key + ?Sized, V> OwnedTypedNodePtr<LeafNode<K, V>> {
    pub fn into_value(self) -> V {
        let raw = self.into_unknown();
        let ptr = raw.as_ptr();
        unsafe {
            let value_ptr = addr_of_mut!((*ptr).value);
            let header_ptr = addr_of_mut!((*ptr).header);

            // drop the header.
            std::ptr::drop_in_place(header_ptr);
            // move out the value
            let value = value_ptr.read();
            // all fields dropped, or moved, so deallocated without dropping.
            TypedNodePtr::<Unknown, LeafNode<K, V>>::dealloc(raw);
            value
        }
    }
}

impl<O: ValidPtr, K: Key + ?Sized, V> TypedNodePtr<O, LeafNode<K, V>> {
    pub fn as_value(&self) -> &V {
        &self.as_ref().value
    }
}

impl<O: MutValuePtr, K: Key + ?Sized, V> TypedNodePtr<O, LeafNode<K, V>> {
    pub fn as_value_mut(&mut self) -> &mut V {
        unsafe {
            let value_ptr = addr_of_mut!((*self.as_unknown().as_ptr()).value);
            &mut (*value_ptr)
        }
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> LeafNode<K, V> {
    pub fn display(&self, fmt: &mut fmt::Formatter, _depth: usize) -> fmt::Result {
        writeln!(
            fmt,
            "LEAF: len={:?} prefix={:?} | {:?}",
            self.header.data().len,
            self.header.storage.prefix(),
            self.value
        )
    }
}
