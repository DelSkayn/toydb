use crate::{
    header::{NodeHeader, NodeKind},
    key::Key,
};
use std::{
    alloc::Layout,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

mod leaf;
mod node16;
mod node256;
mod node4;
mod node48;

pub use leaf::LeafNode;
pub use node16::Node16;
pub use node256::Node256;
pub use node4::Node4;
pub use node48::Node48;

#[repr(transparent)]
pub struct BoxedNode<N>(NonNull<N>);

impl<N> BoxedNode<N> {
    unsafe fn alloc() -> NonNull<N> {
        let ptr = std::alloc::alloc(Layout::new::<N>());
        NonNull::new(ptr).unwrap().cast::<N>()
    }

    unsafe fn dealloc(ptr: NonNull<N>) {
        std::alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<N>());
    }

    fn new(node: N) -> Self {
        unsafe {
            let this = Self::alloc();
            this.as_ptr().write(node);
            BoxedNode(this)
        }
    }
}

impl<N> Drop for BoxedNode<N> {
    fn drop(&mut self) {
        unsafe {
            std::ptr::drop_in_place(self.0.as_ptr());
            std::alloc::dealloc(self.0.as_ptr().cast(), Layout::new::<N>())
        }
    }
}

impl<N> Deref for BoxedNode<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<N> DerefMut for BoxedNode<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

pub struct NodePtr<K: Key, V>(NonNull<NodeHeader<K>>, PhantomData<V>);

impl<K: Key, V> From<BoxedNode<LeafNode<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<LeafNode<K, V>>) -> Self {
        Self(value.0.cast(), PhantomData)
    }
}

impl<K: Key, V> From<BoxedNode<Node4<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<Node4<K, V>>) -> Self {
        Self(value.0.cast(), PhantomData)
    }
}

impl<K: Key, V> From<BoxedNode<Node16<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<Node16<K, V>>) -> Self {
        Self(value.0.cast(), PhantomData)
    }
}

impl<K: Key, V> From<BoxedNode<Node48<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<Node48<K, V>>) -> Self {
        Self(value.0.cast(), PhantomData)
    }
}

impl<K: Key, V> From<BoxedNode<Node256<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<Node256<K, V>>) -> Self {
        Self(value.0.cast(), PhantomData)
    }
}

impl<K: Key, V> NodePtr<K, V> {
    pub unsafe fn from_ptr(ptr: NonNull<NodeHeader<K>>) -> Self {
        NodePtr(ptr, PhantomData)
    }

    pub fn header(&self) -> &NodeHeader<K> {
        unsafe { self.0.as_ref() }
    }

    pub fn header_mut(&mut self) -> &NodeHeader<K> {
        unsafe { self.0.as_mut() }
    }

    pub fn as_leaf_mut(&mut self) -> Option<&mut LeafNode<K, V>> {
        unsafe { (self.0.as_ref().data().kind == NodeKind::Leaf).then_some(self.0.cast().as_mut()) }
    }

    unsafe fn into_boxed<N>(self) -> BoxedNode<N> {
        BoxedNode::<N>(self.0.cast())
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut NodePtr<K, V>> {
        match self.header().data().kind {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => unsafe { self.0.cast::<Node4<K, V>>().as_mut().get_mut(key) },
            NodeKind::Node16 => unsafe { self.0.cast::<Node16<K, V>>().as_mut().get_mut(key) },
            NodeKind::Node48 => unsafe { self.0.cast::<Node48<K, V>>().as_mut().get_mut(key) },
            NodeKind::Node256 => unsafe { self.0.cast::<Node256<K, V>>().as_mut().get_mut(key) },
        }
    }

    pub fn insert(self, key: u8, value: NodePtr<K, V>) -> (Self, Option<NodePtr<K, V>>) {
        match self.header().data().kind {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => unsafe {
                let mut boxed = self.into_boxed::<Node4<K, V>>();
                if boxed.is_full() {
                    let mut node = Node4::grow(boxed);
                    let res = node.insert(key, value);
                    return (node.into(), res);
                }
                let res = boxed.insert(key, value);
                (boxed.into(), res)
            },
            _ => todo!(),
        }
    }
}

impl<K: Key, V> Drop for NodePtr<K, V> {
    fn drop(&mut self) {
        unsafe {
            match self.0.as_ref().data().kind {
                NodeKind::Leaf => std::mem::drop(BoxedNode(self.0.cast::<LeafNode<K, V>>())),
                NodeKind::Node4 => std::mem::drop(BoxedNode(self.0.cast::<Node4<K, V>>())),
                _ => todo!(),
            }
        }
    }
}
