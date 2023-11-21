use crate::{
    header::{NodeHeader, NodeKind},
    key::{Key, KeyStorage},
};
use std::{fmt, marker::PhantomData, ptr::NonNull};

mod boxed_node;
mod leaf;
mod node16;
mod node256;
mod node4;
mod node48;

pub use boxed_node::BoxedNode;
pub use leaf::LeafNode;
pub use node16::Node16;
pub use node256::Node256;
pub use node4::Node4;
pub use node48::Node48;

pub struct NodePtr<K: Key + ?Sized, V>(NonNull<NodeHeader<K>>, PhantomData<V>);

impl<K: Key + ?Sized, V> fmt::Debug for NodePtr<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NodePtr").field(&self.0).finish()
    }
}

impl<K: Key + ?Sized, V> From<BoxedNode<LeafNode<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<LeafNode<K, V>>) -> Self {
        Self(value.into_raw().cast(), PhantomData)
    }
}

impl<K: Key + ?Sized, V> From<BoxedNode<Node4<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<Node4<K, V>>) -> Self {
        Self(value.into_raw().cast(), PhantomData)
    }
}

impl<K: Key + ?Sized, V> From<BoxedNode<Node16<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<Node16<K, V>>) -> Self {
        Self(value.into_raw().cast(), PhantomData)
    }
}

impl<K: Key + ?Sized, V> From<BoxedNode<Node48<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<Node48<K, V>>) -> Self {
        Self(value.into_raw().cast(), PhantomData)
    }
}

impl<K: Key + ?Sized, V> From<BoxedNode<Node256<K, V>>> for NodePtr<K, V> {
    fn from(value: BoxedNode<Node256<K, V>>) -> Self {
        Self(value.into_raw().cast(), PhantomData)
    }
}

impl<K: Key + ?Sized, V> NodePtr<K, V> {
    pub unsafe fn from_ptr(ptr: NonNull<NodeHeader<K>>) -> Self {
        NodePtr(ptr, PhantomData)
    }

    pub fn header(&self) -> &NodeHeader<K> {
        unsafe { self.0.as_ref() }
    }

    pub fn header_mut(&mut self) -> &mut NodeHeader<K> {
        unsafe { self.0.as_mut() }
    }

    pub fn as_leaf(&self) -> Option<&LeafNode<K, V>> {
        unsafe { (self.0.as_ref().data().kind == NodeKind::Leaf).then_some(self.0.cast().as_ref()) }
    }

    pub fn as_leaf_mut(&mut self) -> Option<&mut LeafNode<K, V>> {
        unsafe { (self.0.as_ref().data().kind == NodeKind::Leaf).then_some(self.0.cast().as_mut()) }
    }

    unsafe fn into_boxed<N>(self) -> BoxedNode<N> {
        let res = BoxedNode::<N>::from_raw(self.0.cast());
        std::mem::forget(self);
        res
    }

    pub fn into_raw(self) -> NonNull<NodeHeader<K>> {
        let res = self.0;
        std::mem::forget(self);
        res
    }

    pub unsafe fn from_raw(ptr: NonNull<NodeHeader<K>>) -> Self {
        Self(ptr, PhantomData)
    }

    pub fn get(&self, key: u8) -> Option<&NodePtr<K, V>> {
        match self.header().data().kind {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => unsafe { self.0.cast::<Node4<K, V>>().as_ref().get(key) },
            NodeKind::Node16 => unsafe { self.0.cast::<Node16<K, V>>().as_ref().get(key) },
            NodeKind::Node48 => unsafe { self.0.cast::<Node48<K, V>>().as_ref().get(key) },
            NodeKind::Node256 => unsafe { self.0.cast::<Node256<K, V>>().as_ref().get(key) },
        }
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

    pub fn insert(&mut self, key: u8, value: NodePtr<K, V>) -> Option<NodePtr<K, V>> {
        match self.header().data().kind {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => unsafe { Node4::insert_grow(self, key, value) },
            NodeKind::Node16 => unsafe { Node16::insert_grow(self, key, value) },
            NodeKind::Node48 => unsafe { Node48::insert_grow(self, key, value) },
            NodeKind::Node256 => unsafe {
                self.0.cast::<Node256<K, V>>().as_mut().insert(key, value)
            },
        }
    }

    pub fn display(&self, depth: usize) {
        for _ in 0..(depth * 4) {
            print!(" ");
        }
        match self.header().data().kind {
            NodeKind::Leaf => print!("leaf"),
            NodeKind::Node4 => print!("node4"),
            NodeKind::Node16 => print!("node16"),
            NodeKind::Node48 => print!("node48"),
            NodeKind::Node256 => print!("node256"),
        }
        println!(" {:?}", self.header().storage().key());
        match self.header().data().kind {
            NodeKind::Leaf => {}
            NodeKind::Node4 => unsafe {
                let node = self.0.cast::<Node4<K, V>>();
                for i in 0..node.as_ref().header.data().len {
                    node.as_ref().ptr[i as usize]
                        .assume_init_ref()
                        .display(depth + 1);
                }
            },
            NodeKind::Node16 => unsafe {
                let node = self.0.cast::<Node16<K, V>>();
                for i in 0..node.as_ref().header.data().len {
                    node.as_ref().ptr[i as usize]
                        .assume_init_ref()
                        .display(depth + 1);
                }
            },
            _ => todo!(),
        }
    }
}

impl<K: Key + ?Sized, V> Drop for NodePtr<K, V> {
    fn drop(&mut self) {
        unsafe {
            match self.header().data().kind {
                NodeKind::Leaf => {
                    std::mem::drop(BoxedNode::from_raw(self.0.cast::<LeafNode<K, V>>()))
                }
                NodeKind::Node4 => {
                    std::mem::drop(BoxedNode::from_raw(self.0.cast::<Node4<K, V>>()))
                }
                NodeKind::Node16 => {
                    std::mem::drop(BoxedNode::from_raw(self.0.cast::<Node16<K, V>>()))
                }
                NodeKind::Node48 => {
                    std::mem::drop(BoxedNode::from_raw(self.0.cast::<Node48<K, V>>()))
                }
                NodeKind::Node256 => {
                    std::mem::drop(BoxedNode::from_raw(self.0.cast::<Node256<K, V>>()))
                }
            }
        }
    }
}
