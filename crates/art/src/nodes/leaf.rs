use super::{owned_node::RawOwnedNode, NodeHeader, NodeKind, NodeType, OwnedNode};
use crate::key::Key;
use std::{ops::Range, ptr::addr_of_mut};

#[repr(C)]
pub struct LeafNode<K: Key + ?Sized, V> {
    pub header: NodeHeader<K>,
    pub value: V,
}

unsafe impl<K: Key + ?Sized, V> NodeType for LeafNode<K, V> {
    const KIND: NodeKind = NodeKind::Leaf;

    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> LeafNode<K, V> {
    pub fn new(key: &K, range: Range<usize>, value: V) -> Self {
        LeafNode {
            header: NodeHeader::new::<Self>(key, range),
            value,
        }
    }

    pub fn into_value(this: OwnedNode<Self>) -> V {
        let this = this.into_raw();
        unsafe {
            std::ptr::drop_in_place(addr_of_mut!((*this.as_ptr()).header));
            let res = addr_of_mut!((*this.as_ptr()).value).read();
            RawOwnedNode::dealloc(this);
            res
        }
    }
}
