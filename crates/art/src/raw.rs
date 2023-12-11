use core::fmt;

use crate::key::{Key, KeyStorage};
use crate::nodes::{BoxedNode, LeafNode, Node4, OwnedNode, RawBoxedNode};

pub struct RawArt<K: Key + ?Sized, V> {
    root: Option<BoxedNode<K, V>>,
}

impl<K: Key + ?Sized, V> RawArt<K, V> {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        if let Some(root) = self.root.as_ref() {
            return unsafe { Self::find_value(root.as_raw(), key) };
        }
        None
    }

    pub fn get_mut(&self, key: &K) -> Option<&mut V> {
        if let Some(root) = self.root.as_ref() {
            return unsafe { Self::find_value_mut(root.as_raw(), key) };
        }
        None
    }

    pub fn insert(&mut self, key: &K, value: V) -> Option<V> {
        if let Some(x) = self.root.as_mut() {
            unsafe { Self::insert_node(x.as_raw_mut(), key, value) }
        } else {
            let range = 0..key.len();
            let leaf_node = OwnedNode::new(LeafNode::new(key, range, value));
            self.root = Some(leaf_node.into());
            None
        }
    }

    unsafe fn find_value<'a>(mut node: RawBoxedNode<K, V>, key: &K) -> Option<&'a V>
    where
        K: 'a,
    {
        let mut matched: usize = 0;

        loop {
            let prefix = node.prefix();
            if Self::match_prefix(key, matched, prefix).is_some() {
                return None;
            }
            matched += prefix.len();

            if matched == key.len() {
                if let Some(x) = node
                    .is::<LeafNode<K, V>>()
                    .then(|| node.as_ref::<LeafNode<K, V>>())
                {
                    return Some(&x.value);
                }
                // we matched all but have no key left, so it is a prefix.
                panic!("key was a prefix of an existing key");
            }
            let decision = key.at(matched);

            if let Some(x) = node.get(decision) {
                matched += 1;
                node = x.as_raw();
            } else {
                return None;
            }
        }
    }

    unsafe fn find_value_mut<'a>(mut node: RawBoxedNode<K, V>, key: &K) -> Option<&'a mut V>
    where
        K: 'a,
    {
        let mut matched: usize = 0;

        loop {
            let prefix = node.prefix();
            if Self::match_prefix(key, matched, prefix).is_some() {
                return None;
            }
            matched += prefix.len();

            if matched == key.len() {
                if let Some(x) = node
                    .is::<LeafNode<K, V>>()
                    .then(|| node.as_mut::<LeafNode<K, V>>())
                {
                    return Some(&mut x.value);
                }
                // we matched all but have no key left, so it is a prefix.
                panic!("key was a prefix of an existing key");
            }
            let decision = key.at(matched);

            if let Some(x) = node.get(decision) {
                matched += 1;
                node = x.as_raw();
            } else {
                return None;
            }
        }
    }

    unsafe fn insert_node(mut node: &mut RawBoxedNode<K, V>, key: &K, value: V) -> Option<V> {
        let mut matched = 0;
        loop {
            let prefix = node.header().storage().prefix();
            if let Some(x) = Self::match_prefix(key, matched, prefix) {
                // prefix diverged, split node in prefix.
                Self::split_at_prefix(node, key, value, matched, matched + x);
                return None;
            }
            // matched the entire prefix.
            matched += prefix.len();
            if matched >= key.len() {
                if let Some(x) = node
                    .is::<LeafNode<K, V>>()
                    .then(|| node.as_mut::<LeafNode<K, V>>())
                {
                    let res = std::mem::replace(&mut x.value, value);
                    return Some(res);
                }
                // we matched all but have no key left, so it is a prefix.
                panic!("key was a prefix of an existing key");
            }

            let decision = key.at(matched);
            matched += 1;

            if let Some(x) = node.get_mut(decision) {
                node = x.as_raw_mut();
            } else {
                let new_node = OwnedNode::new(LeafNode::new(key, matched..key.len(), value));
                node.insert(decision, new_node.into());
                return None;
            }
        }
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

    unsafe fn split_at_prefix(
        node: &mut RawBoxedNode<K, V>,
        key: &K,
        value: V,
        range_start: usize,
        mismatch_index: usize,
    ) {
        let split_node = OwnedNode::new(Node4::new(key, range_start..mismatch_index));
        let leaf_node = OwnedNode::new(LeafNode::new(key, (mismatch_index + 1)..key.len(), value));

        let value_key = key.at(mismatch_index);
        let prefix_mismatch_offset = mismatch_index - range_start;
        let old_key = node.header().storage().prefix()[prefix_mismatch_offset];

        // +1 because also drop the mismatching key.
        node.header_mut()
            .storage_mut()
            .drop_prefix(prefix_mismatch_offset + 1);

        dbg!(node.header().storage().prefix());

        let mut split_raw = split_node.into_raw();
        let old_node = std::mem::replace(node, split_raw.into_boxed());

        split_raw.as_mut().insert(value_key, leaf_node.into());
        split_raw
            .as_mut()
            .insert(old_key, BoxedNode::from_raw(old_node));
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let Some(root) = self.root.as_mut() else {
            return None;
        };

        if Self::match_prefix(key, 0, root.header().storage().prefix()).is_some() {
            return None;
        }

        if key.len() == root.header().storage().prefix().len() {
            match self.root.take().unwrap().into_owned() {
                Ok(x) => return Some(LeafNode::<K, V>::into_value(x)),
                Err(this) => {
                    self.root = Some(this);
                    return None;
                }
            }
        }

        Self::remove_node(root, key, root.header().storage().prefix().len())
    }

    fn remove_node(node: &mut BoxedNode<K, V>, key: &K, mut matched: usize) -> Option<V> {
        let select = key.at(matched);
        matched += 1;

        let _next_node = node.get_mut(select);

        let prefix = node.header().storage().prefix();
        if Self::match_prefix(key, matched, prefix).is_some() {
            // prefix diverged, split node in prefix.
            return None;
        }
        /*
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
        */
        todo!()
    }

    /*
    pub fn display(&self) {
        if let Some(x) = self.root.as_ref() {
            x.display(1)
        }
    }
    */
}

impl<K: Key + ?Sized, V: fmt::Display> RawArt<K, V> {
    pub fn display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(x) = self.root.as_ref() {
            write!(f, "TREE = ")?;
            x.display(f, 1)?;
        } else {
            writeln!(f, "TREE = EMPTY")?;
        }
        Ok(())
    }
}
