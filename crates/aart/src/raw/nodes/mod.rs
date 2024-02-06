use crate::{
    key::{KeyBytes, KeyStorage},
    prim::sync::atomic::AtomicUsize,
};
use bytemuck::{Pod, Zeroable};
use core::panic;
use std::marker::PhantomData;

mod leaf;
mod node16;
mod node256;
mod node4;
mod node48;
mod ptr;
pub use leaf::NodeLeaf;
pub use node16::Node16;
pub use node4::Node4;
pub use node48::Node48;
pub use ptr::*;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct NodeHeaderData {
    pub kind: u8,
    pub len: u8,
    pub free: u8,
}

impl NodeHeaderData {
    pub fn leaf() -> Self {
        NodeHeaderData {
            len: 0,
            kind: NodeKind::Leaf as u8,
            free: 0,
        }
    }

    pub fn new(len: u8, kind: NodeKind, free: u8) -> Self {
        NodeHeaderData {
            len,
            kind: kind as u8,
            free,
        }
    }

    pub fn kind(self) -> NodeKind {
        NodeKind::from_u8(self.kind)
    }

    pub fn free(self) -> u8 {
        self.free
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NodeKind {
    Leaf = 0,
    Node4,
    Node16,
    Node48,
    Node256,
}

impl NodeKind {
    pub fn from_u8(v: u8) -> NodeKind {
        match v {
            0 => NodeKind::Leaf,
            1 => NodeKind::Node4,
            2 => NodeKind::Node16,
            3 => NodeKind::Node48,
            4 => NodeKind::Node256,
            x => panic!("invalid node kind {x}"),
        }
    }
}

pub struct NodeHeader<K: KeyBytes + ?Sized, V> {
    pub(crate) ref_count: AtomicUsize,
    storage: K::Storage,
    _marker: PhantomData<V>,
}

impl<K: KeyBytes + ?Sized, V> NodeHeader<K, V> {
    pub fn new(key: &K, until: usize, data: NodeHeaderData) -> Self {
        let ref_count = AtomicUsize::new(1);
        let storage = K::Storage::store(key, until, bytemuck::cast(data));
        NodeHeader {
            ref_count,
            storage,
            _marker: PhantomData,
        }
    }

    pub fn new_from(existing: &Self, data: NodeHeaderData) -> Self {
        Self {
            ref_count: AtomicUsize::new(1),
            storage: K::Storage::new_from(&existing.storage, bytemuck::cast(data)),
            _marker: PhantomData,
        }
    }

    pub fn copy_drop_prefix(&self, until: usize) -> Self {
        Self {
            ref_count: AtomicUsize::new(1),
            storage: self.storage.copy_drop_prefix(until),
            _marker: PhantomData,
        }
    }

    pub fn data(&self) -> NodeHeaderData {
        bytemuck::cast::<_, NodeHeaderData>(self.storage.data())
    }

    pub fn kind(&self) -> NodeKind {
        self.data().kind()
    }

    pub fn prefix(&self) -> &[u8] {
        self.storage.prefix()
    }
}

/// A trait implemented by ART node types.
///
/// # Safety
/// This trait should not be implemented outside this crate.
pub unsafe trait Node {
    const KIND: NodeKind;
    type Key: KeyBytes + ?Sized;
    type Value;

    fn header(&self) -> &NodeHeader<Self::Key, Self::Value> {
        unsafe { &*(self as *const Self).cast() }
    }
}
