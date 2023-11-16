#![allow(dead_code)]

use key::Key;
use nodes::{LeafNode, NodePtr};

use crate::key::KeyStorage;

#[macro_use]
mod mac;

mod header;
mod key;
mod nodes;

pub struct Art<K: Key, V> {
    root: Option<NodePtr<K, V>>,
}

impl<K: Key, V> Default for Art<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Key, V> Art<K, V> {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        todo!()
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        todo!()
    }

    pub fn insert(&mut self, key: &K, value: V) -> Option<V> {
        if let Some(x) = self.root.as_mut() {
            Self::insert_node(x, key, value)
        } else {
            let range = 0..key.len();
            let leaf_node = LeafNode::new(key, range, value);
            self.root = Some(leaf_node.into());
            None
        }
    }

    fn insert_node(mut node: &mut NodePtr<K, V>, key: &K, value: V) -> Option<V> {
        // how far we have fully matched nodes to the key.
        let mut matched = 0;
        // for each node.
        loop {
            let prefix = node.header().storage().key();
            if let Some(x) = Self::match_prefix(key, matched, prefix) {
                // prefix diverged, split node in prefix.
                Self::split_at_prefix(node, x, key, matched + x);
                return None;
            }
            // matched the entire prefix.
            matched += prefix.len();
            if matched >= key.len() {
                // we matched all but have no key left, so it is a prefix.
                panic!("key was a prefix of an existing key");
            }

            let decision = key.at(matched);
            matched += 1;

            if let Some(x) = node.get_mut(decision) {
                node = x;
                continue;
            }

            let new_node = LeafNode::new(key, matched..key.len(), value);
            *node = node.insert(decision, new_node.into()).0;
            return None;
        }
    }

    fn match_prefix(key: &K, from: usize, to: &[u8]) -> Option<usize> {
        for (idx, p) in to.iter().copied().enumerate() {
            if idx + from > key.len() {
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
        mismatch_index: usize,
        key: &K,
        key_mismatch: usize,
    ) {
        todo!()
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        todo!()
    }
}
