use super::BoxedNode;
use crate::{
    header::{NodeHeader, NodeKind},
    key::Key,
};
use std::ops::Range;

#[repr(C)]
pub struct LeafNode<K: Key + ?Sized, V> {
    header: NodeHeader<K>,
    pub value: V,
}

impl<K: Key + ?Sized, V> LeafNode<K, V> {
    pub fn new(key: &K, range: Range<usize>, value: V) -> Self {
        LeafNode {
            header: NodeHeader::new(key, range, NodeKind::Leaf),
            value,
        }
    }
}
