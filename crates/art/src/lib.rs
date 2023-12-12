#![allow(dead_code)]

use key::Key;
use raw::RawArt;

mod key;
mod nodes;
mod raw;

#[cfg(test)]
mod test;

pub struct Art<K: Key + ?Sized, V> {
    tree: RawArt<K, V>,
    len: usize,
}

impl<K: Key + ?Sized, V> Default for Art<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Key + ?Sized, V> Art<K, V> {
    pub fn new() -> Self {
        Self {
            tree: RawArt::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.tree.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.tree.get_mut(key)
    }

    pub fn insert(&mut self, key: &K, value: V) -> Option<V> {
        let res = self.tree.insert(key, value);
        self.len += res.is_none() as usize;
        res
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let res = self.tree.remove(key);
        self.len -= res.is_some() as usize;
        res
    }
}

impl<K: Key + ?Sized, V: std::fmt::Debug> Art<K, V> {
    pub fn print(&self) {
        struct Printer<'a, K: Key + ?Sized, V: std::fmt::Debug>(&'a Art<K, V>);
        impl<K: Key + ?Sized, V: std::fmt::Debug> std::fmt::Display for Printer<'_, K, V> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.tree.display(f)
            }
        }
        println!("{}", Printer(self));
    }
}
