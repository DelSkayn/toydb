#![allow(dead_code)]

use key::Key;
use raw::RawAart;

pub mod key;
mod prim;
pub mod raw;

pub struct Aart<K: Key + ?Sized, V> {
    inner: RawAart<K::Bytes, V>,
}

impl<K: Key + ?Sized, V> Aart<K, V> {
    pub fn new() -> Self {
        Aart {
            inner: RawAart::new(),
        }
    }

    pub fn insert(&mut self, key: &K, value: V) {
        self.inner.insert(key.as_key_bytes(), value);
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.inner.get(key.as_key_bytes()).map(|x| &*x.value)
    }
}

impl<K: Key + ?Sized, V> Default for Aart<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
