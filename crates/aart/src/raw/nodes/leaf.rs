use std::sync::Arc;

use crate::key::KeyBytes;

use super::{Node, NodeBox, NodeHeader, NodeHeaderData, NodeKind};

#[repr(C)]
pub struct NodeLeaf<K: KeyBytes + ?Sized, V> {
    pub(crate) header: NodeHeader<K, V>,
    pub(crate) value: Arc<V>,
}

unsafe impl<K: KeyBytes + ?Sized, V> Node for NodeLeaf<K, V> {
    const KIND: super::NodeKind = NodeKind::Leaf;

    type Key = K;

    type Value = V;
}

impl<K: KeyBytes + ?Sized, V> NodeLeaf<K, V> {
    pub fn new(key: &K, until: usize, value: V) -> Self {
        let header = NodeHeader::new(key, until, NodeHeaderData::leaf());
        NodeLeaf {
            header,
            value: Arc::new(value),
        }
    }

    pub fn copy_drop_prefix(&self, drop: usize) -> NodeBox<K, V> {
        NodeBox::new(NodeLeaf {
            header: self.header.copy_drop_prefix(drop),
            value: self.value.clone(),
        })
    }
}
