mod header;
mod leaf;
mod node16;
mod node256;
mod node4;
mod node48;

pub use header::{NodeData, NodeHeader, NodeKind};
pub use leaf::LeafNode;
pub use node16::Node16;
pub use node256::Node256;
pub use node4::Node4;
pub use node48::Node48;

use crate::key::{Key, KeyStorage};

use super::{MutablePtr, NodePtr, OwnedNodePtr, OwnedTypedNodePtr};

/// # Safety
/// Implementor must ensure that the associated KIND value is distinct from any other type
/// impelementing NodeType.
pub unsafe trait NodeType {
    const KIND: NodeKind;
    type Key: Key + ?Sized;
    type Value;
}

impl<O: MutablePtr, K: Key + ?Sized, V> NodePtr<O, K, V> {
    pub fn insert_grow(&mut self, key: u8, v: OwnedNodePtr<K, V>) -> Option<OwnedNodePtr<K, V>> {
        match self.header().kind() {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => self.insert_grow_4(key, v),
            NodeKind::Node16 => self.insert_grow_16(key, v),
            NodeKind::Node48 => self.insert_grow_48(key, v),
            NodeKind::Node256 => {
                unsafe { self.cast_mut_unchecked::<Node256<K, V>>() }.insert(key, v)
            }
        }
    }

    pub fn remove(&mut self, key: u8) -> Option<OwnedNodePtr<K, V>> {
        match self.header().kind() {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => unsafe {
                let res = self.cast_mut_unchecked::<Node4<K, V>>().remove(key);
                // TODO: Fold
                res
            },
            NodeKind::Node16 => unsafe {
                let mut cast = self.cast_mut_unchecked::<Node16<K, V>>();
                let res = cast.remove(key);
                if cast.should_shrink() {
                    self.shrink_16()
                }
                res
            },
            NodeKind::Node48 => unsafe {
                let mut cast = self.cast_mut_unchecked::<Node48<K, V>>();
                let res = cast.remove(key);
                if cast.should_shrink() {
                    self.shrink_48()
                }
                res
            },
            NodeKind::Node256 => unsafe {
                let mut cast = self.cast_mut_unchecked::<Node256<K, V>>();
                let res = cast.remove(key);
                if cast.should_shrink() {
                    self.shrink_256()
                }
                res
            },
        }
    }

    pub fn new_branch(
        &mut self,
        key: &K,
        value: V,
        at: usize,
        range_start: usize,
        mismatch_index: usize,
    ) {
        let split_node = OwnedTypedNodePtr::new(Node4::<K, V>::new(key, range_start..at));
        let leaf_node =
            OwnedTypedNodePtr::new(LeafNode::<K, V>::new(key, (at + 1)..key.len(), value));

        let new_key = key.at(at);

        let prefix_mismatch_offset = mismatch_index - range_start;
        let old_key = self.header().prefix()[prefix_mismatch_offset];

        // +1 because also drop the mismatching key.
        self.header_mut().storage.drop_prefix(at + 1);

        unsafe {
            let old = self.as_unknown();
            *self = split_node.erase_type().as_unknown().assume_ownership();

            let mut this = self.cast_mut_unchecked::<Node4<K, V>>();

            this.header.data_mut().len = 2;

            if new_key < old_key {
                this.keys[0] = new_key;
                this.ptr[0] = leaf_node.erase_type().as_unknown();
                this.keys[1] = old_key;
                this.ptr[1] = old;
            } else {
                this.keys[0] = old_key;
                this.ptr[0] = old;
                this.keys[1] = new_key;
                this.ptr[1] = leaf_node.erase_type().as_unknown();
            }
        }
    }
}
