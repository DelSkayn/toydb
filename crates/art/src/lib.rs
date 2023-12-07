#![allow(dead_code)]

use std::fmt;

use key::Key;
use nodes::{LeafNode, Node4, NodePtr};

use crate::{key::KeyStorage, nodes::BoxedNode};

mod header;
mod key;
mod nodes;

#[cfg(test)]
mod test;

pub struct Art<K: Key + ?Sized, V> {
    root: Option<NodePtr<K, V>>,
}

impl<K: Key + ?Sized, V> fmt::Debug for Art<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Art").field("root", &self.root).finish()
    }
}

impl<K: Key + ?Sized, V> Default for Art<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Key + ?Sized, V> Art<K, V> {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        if let Some(root) = self.root.as_ref() {
            return Self::find_value(root, 0, key);
        }
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if let Some(root) = self.root.as_mut() {
            return Self::find_value_mut(root, 0, key);
        }
        None
    }

    pub fn insert(&mut self, key: &K, value: V) -> Option<V> {
        if let Some(x) = self.root.as_mut() {
            Self::insert_node(x, 0, key, value)
        } else {
            let range = 0..key.len();
            let leaf_node = BoxedNode::new(LeafNode::new(key, range, value));
            self.root = Some(leaf_node.into());
            None
        }
    }

    fn find_value<'a>(node: &'a NodePtr<K, V>, mut matched: usize, key: &K) -> Option<&'a V> {
        let prefix = node.header().storage().key();
        if Self::match_prefix(key, matched, prefix).is_some() {
            return None;
        }

        matched += prefix.len();
        if matched >= key.len() {
            if let Some(x) = node.as_leaf() {
                return Some(&x.value);
            }
            // we matched all but have no key left, so it is a prefix.
            panic!("key was a prefix of an existing key");
        }
        let decision = key.at(matched);

        if let Some(x) = node.get(decision) {
            matched += 1;
            return Self::find_value(x, matched, key);
        }
        None
    }

    fn find_value_mut<'a>(
        node: &'a mut NodePtr<K, V>,
        mut matched: usize,
        key: &K,
    ) -> Option<&'a mut V> {
        let prefix = node.header().storage().key();
        if Self::match_prefix(key, matched, prefix).is_some() {
            return None;
        }

        matched += prefix.len();
        if matched >= key.len() {
            if let Some(x) = node.as_leaf_mut() {
                return Some(&mut x.value);
            }
            // we matched all but have no key left, so it is a prefix.
            panic!("key was a prefix of an existing key");
        }
        let decision = key.at(matched);

        if let Some(x) = node.get_mut(decision) {
            matched += 1;
            return Self::find_value_mut(x, matched, key);
        }
        None
    }

    fn insert_node(node: &mut NodePtr<K, V>, mut matched: usize, key: &K, value: V) -> Option<V> {
        let prefix = node.header().storage().key();
        if let Some(x) = Self::match_prefix(key, matched, prefix) {
            // prefix diverged, split node in prefix.
            Self::split_at_prefix(node, key, value, matched, matched + x);
            return None;
        }
        // matched the entire prefix.
        matched += prefix.len();
        if matched >= key.len() {
            if let Some(x) = node.as_leaf_mut() {
                let res = std::mem::replace(&mut x.value, value);
                return Some(res);
            }
            // we matched all but have no key left, so it is a prefix.
            panic!("key was a prefix of an existing key");
        }

        let decision = key.at(matched);
        matched += 1;

        if let Some(x) = node.get_mut(decision) {
            return Self::insert_node(x, matched, key, value);
        }

        let new_node = BoxedNode::new(LeafNode::new(key, matched..key.len(), value));
        node.insert(decision, new_node.into());
        None
    }

    fn match_prefix(key: &K, from: usize, to: &[u8]) -> Option<usize> {
        for (idx, p) in to.iter().copied().enumerate() {
            if idx + from >= key.len() {
                panic!("key was a prefix of an existing key");
            }
            let k = key.at(from + idx);
            if p != k {
                return Some(idx);
            }
        }
        None
    }

    fn split_at_prefix(
        node: &mut NodePtr<K, V>,
        key: &K,
        value: V,
        range_start: usize,
        mismatch_index: usize,
    ) {
        let split_node = BoxedNode::new(Node4::new(key, range_start..mismatch_index));
        let leaf_node = BoxedNode::new(LeafNode::new(key, (mismatch_index + 1)..key.len(), value));

        let mut split_ptr = split_node.as_raw();

        let value_key = key.at(mismatch_index);
        let prefix_mismatch_offset = mismatch_index - range_start;
        let old_key = node.header().storage().key()[prefix_mismatch_offset];

        // +1 because also drop the mismatching key.
        node.header_mut()
            .storage_mut()
            .drop_start(prefix_mismatch_offset + 1);

        dbg!(node.header().storage().key());

        let old_node = std::mem::replace(node, split_node.into());

        unsafe {
            split_ptr.as_mut().insert(value_key, leaf_node.into());
            split_ptr.as_mut().insert(old_key, old_node);
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        todo!()
    }

    pub fn display(&self) {
        if let Some(x) = self.root.as_ref() {
            x.display(1)
        }
    }
}
