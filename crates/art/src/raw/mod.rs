use crate::{
    iter::RawIterator,
    key::{BorrowedKey, Key},
};
use core::fmt;

mod nodes;
mod ptr;

pub use nodes::*;
pub use ptr::*;

pub struct RawArt<K: Key + ?Sized, V> {
    root: Option<OwnedNodePtr<K, V>>,
}

impl<K: Key + ?Sized, V> RawArt<K, V> {
    const PREFIX_PANIC: &'static str = "the key was a prefix of an existing key";

    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        todo!()
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        todo!()
    }

    pub fn insert(&mut self, key: &K, value: V) -> Option<V> {
        if let Some(x) = self.root.as_mut() {
            todo!()
        }
        self.root =
            Some(OwnedTypedNodePtr::new(LeafNode::new(key, 0..key.len(), value)).erase_type());
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let root = self.root.as_mut()?;
        if root.is::<LeafNode<K, V>>() {
            todo!()
            //let leaf = self.root.take().unwrap();
            //return Some(leaf.cast_owned::<LeafNode<K, V>>().into_value());
        }

        todo!()
    }

    pub fn iter(&self) -> BorrowIter<K, V> {
        todo!()
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

    fn insert_node(
        mut node: NodePtr<BorrowMut, K, V>,
        key: &K,
        value: V,
    ) -> Option<OwnedNodePtr<K, V>> {
        let mut matched: usize = 0;

        loop {
            let prefix = node.header().prefix();
            if let Some(x) = Self::match_prefix(key, matched, prefix) {
                node.new_branch(key, value, matched, range_start, mismatch_index)
            }
        }
    }
}

pub struct BorrowIter<'a, K: Key + ?Sized, V> {
    raw: RawIterator<'a, Borrow<'a>, K, V>,
}

impl<'a, K: Key + BorrowedKey + ?Sized, V> BorrowIter<'a, K, V> {
    pub fn next(&mut self) -> Option<(&K, &V)> {
        todo!()
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> RawArt<K, V> {
    pub fn display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        /*
        if let Some(x) = self.root.as_ref() {
            write!(f, "TREE = ")?;
            x.display(f, 1)?;
        } else {
            writeln!(f, "TREE = EMPTY")?;
        }
        Ok(())
        */
        todo!()
    }
}
