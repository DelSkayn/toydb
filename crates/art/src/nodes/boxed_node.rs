use super::{
    owned_node::RawOwnedNode, LeafNode, Node16, Node256, Node4, Node48, NodeHeader, NodeKind,
    NodeType, OwnedNode,
};
use crate::key::{Key, KeyStorage};
use core::fmt;
use std::{marker::PhantomData, ptr::NonNull};

#[repr(transparent)]
pub struct RawBoxedNode<K: Key + ?Sized, V>(NonNull<NodeHeader<K>>, PhantomData<V>);
impl<K: Key + ?Sized, V> Copy for RawBoxedNode<K, V> {}
impl<K: Key + ?Sized, V> Clone for RawBoxedNode<K, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: Key + ?Sized, V> RawBoxedNode<K, V> {
    pub const fn dangling() -> Self {
        Self(NonNull::dangling(), PhantomData)
    }

    pub fn from_node_ptr<N: NodeType<Key = K, Value = V>>(ptr: NonNull<N>) -> Self {
        RawBoxedNode(ptr.cast(), PhantomData)
    }

    pub fn from_ptr(ptr: NonNull<NodeHeader<K>>) -> Self {
        RawBoxedNode(ptr, PhantomData)
    }

    pub fn into_ptr(self) -> *mut NodeHeader<K> {
        self.0.as_ptr()
    }

    pub fn into_nonnull(self) -> NonNull<NodeHeader<K>> {
        self.0
    }

    pub unsafe fn header(&self) -> &NodeHeader<K> {
        self.0.as_ref()
    }

    pub unsafe fn header_mut(&mut self) -> &mut NodeHeader<K> {
        self.0.as_mut()
    }

    pub unsafe fn prefix(&self) -> &[u8] {
        self.header().storage().prefix()
    }

    pub unsafe fn is<N: NodeType>(&self) -> bool {
        self.header().data().kind == N::KIND
    }

    pub unsafe fn as_ref<'a, N: NodeType>(self) -> &'a N {
        debug_assert!(self.is::<N>());
        self.0.cast().as_ref()
    }

    pub unsafe fn as_mut<'a, N: NodeType>(self) -> &'a mut N {
        debug_assert!(self.is::<N>());
        self.0.cast().as_mut()
    }

    pub unsafe fn as_boxed_ref(&self) -> &BoxedNode<K, V> {
        std::mem::transmute(self)
    }

    pub unsafe fn as_boxed_mut(&mut self) -> &mut BoxedNode<K, V> {
        std::mem::transmute(self)
    }

    pub unsafe fn into_owned<N: NodeType<Key = K, Value = V>>(self) -> RawOwnedNode<N> {
        debug_assert!(self.is::<N>());
        RawOwnedNode::from_ptr(self.0.cast())
    }

    pub unsafe fn get<'a>(self, key: u8) -> Option<&'a BoxedNode<K, V>> {
        match self.header().data().kind {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => unsafe { self.as_ref::<Node4<K, V>>().get(key) },
            NodeKind::Node16 => unsafe { self.as_ref::<Node16<K, V>>().get(key) },
            NodeKind::Node48 => unsafe { self.as_ref::<Node48<K, V>>().get(key) },
            NodeKind::Node256 => unsafe { self.as_ref::<Node256<K, V>>().get(key) },
        }
    }

    pub unsafe fn get_mut<'a>(self, key: u8) -> Option<&'a mut BoxedNode<K, V>> {
        match self.header().data().kind {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => unsafe { self.as_mut::<Node4<K, V>>().get_mut(key) },
            NodeKind::Node16 => unsafe { self.as_mut::<Node16<K, V>>().get_mut(key) },
            NodeKind::Node48 => unsafe { self.as_mut::<Node48<K, V>>().get_mut(key) },
            NodeKind::Node256 => unsafe { self.as_mut::<Node256<K, V>>().get_mut(key) },
        }
    }

    pub unsafe fn insert(&mut self, key: u8, value: BoxedNode<K, V>) -> Option<BoxedNode<K, V>> {
        match self.header().data().kind {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => Node4::insert_grow(self, key, value),
            NodeKind::Node16 => Node16::insert_grow(self, key, value),
            NodeKind::Node48 => Node48::insert_grow(self, key, value),
            NodeKind::Node256 => self.as_mut::<Node256<K, V>>().insert(key, value),
        }
    }

    pub unsafe fn remove(&mut self, key: u8) -> Option<BoxedNode<K, V>> {
        match self.header().data().kind {
            NodeKind::Leaf => panic!(),
            NodeKind::Node4 => {
                let node = self.as_mut::<Node4<K, V>>();
                let res = node.remove(key);
                if node.should_shrink() {
                    *self = Node4::<K, V>::fold(self.into_owned::<Node4<K, V>>());
                }
                res
            }
            NodeKind::Node16 => {
                let node = self.as_mut::<Node16<K, V>>();
                let res = node.remove(key);
                if node.should_shrink() {
                    *self = Node16::shrink(self.into_owned::<Node16<K, V>>()).into_boxed();
                }
                res
            }
            NodeKind::Node48 => {
                let node = self.as_mut::<Node48<K, V>>();
                let res = node.remove(key);
                if node.should_shrink() {
                    *self = Node48::shrink(self.into_owned::<Node48<K, V>>()).into_boxed();
                }
                res
            }
            NodeKind::Node256 => {
                let node = self.as_mut::<Node256<K, V>>();
                let res = node.remove(key);
                if node.should_shrink() {
                    *self = Node256::shrink(self.into_owned::<Node256<K, V>>()).into_boxed();
                }
                res
            }
        }
    }

    pub unsafe fn drop_in_place(self) {
        match self.header().data().kind {
            NodeKind::Leaf => {
                let owned = self.into_owned::<LeafNode<K, V>>();
                owned.drop_in_place();
            }
            NodeKind::Node4 => {
                let owned = self.into_owned::<Node4<K, V>>();
                owned.drop_in_place();
            }
            NodeKind::Node16 => {
                let owned = self.into_owned::<Node16<K, V>>();
                owned.drop_in_place();
            }
            NodeKind::Node48 => {
                let owned = self.into_owned::<Node48<K, V>>();
                owned.drop_in_place();
            }
            NodeKind::Node256 => {
                let owned = self.into_owned::<Node256<K, V>>();
                owned.drop_in_place();
            }
        }
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> RawBoxedNode<K, V> {
    pub unsafe fn display(self, fmt: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        match self.header().data().kind {
            NodeKind::Leaf => self.as_ref::<LeafNode<K, V>>().display(fmt, depth),
            NodeKind::Node4 => self.as_ref::<Node4<K, V>>().display(fmt, depth),
            NodeKind::Node16 => self.as_ref::<Node16<K, V>>().display(fmt, depth),
            NodeKind::Node48 => self.as_ref::<Node48<K, V>>().display(fmt, depth),
            NodeKind::Node256 => self.as_ref::<Node256<K, V>>().display(fmt, depth),
        }
    }
}

/// A pointer to a node of any kind.
/// The pointer owns the node.
#[repr(transparent)]
pub struct BoxedNode<K: Key + ?Sized, V>(RawBoxedNode<K, V>);

impl<N: NodeType, K: Key + ?Sized, V> From<OwnedNode<N>> for BoxedNode<K, V> {
    fn from(value: OwnedNode<N>) -> Self {
        Self(RawBoxedNode::from_ptr(value.into_nonnull().cast()))
    }
}

impl<K: Key + ?Sized, V> BoxedNode<K, V> {
    pub unsafe fn from_ptr(ptr: NonNull<NodeHeader<K>>) -> Self {
        BoxedNode(RawBoxedNode::from_ptr(ptr))
    }

    pub unsafe fn from_raw(ptr: RawBoxedNode<K, V>) -> Self {
        BoxedNode(ptr)
    }

    pub unsafe fn from_raw_ref(ptr: &RawBoxedNode<K, V>) -> &Self {
        std::mem::transmute(ptr)
    }

    pub unsafe fn from_raw_mut(ptr: &mut RawBoxedNode<K, V>) -> &mut Self {
        std::mem::transmute(ptr)
    }

    pub fn header(&self) -> &NodeHeader<K> {
        unsafe { self.0.header() }
    }

    pub fn header_mut(&mut self) -> &mut NodeHeader<K> {
        unsafe { self.0.header_mut() }
    }

    pub fn is<N: NodeType>(&self) -> bool {
        unsafe { self.0.is::<N>() }
    }

    pub fn as_ref<N: NodeType>(&self) -> Option<&N> {
        self.is::<N>().then(|| unsafe { self.0.as_ref() })
    }

    pub fn as_mut<N: NodeType>(&mut self) -> Option<&mut N> {
        self.is::<N>().then(|| unsafe { self.0.as_mut() })
    }

    pub fn into_owned<N: NodeType<Key = K, Value = V>>(self) -> Result<OwnedNode<N>, Self> {
        if self.is::<N>() {
            unsafe { Ok(OwnedNode::from_raw(self.0.into_owned())) }
        } else {
            Err(self)
        }
    }

    pub fn into_raw(self) -> RawBoxedNode<K, V> {
        let res = self.0;
        std::mem::forget(self);
        res
    }

    pub fn as_raw(&self) -> RawBoxedNode<K, V> {
        self.0
    }

    pub fn as_raw_mut(&mut self) -> &mut RawBoxedNode<K, V> {
        &mut self.0
    }

    pub fn get(&self, key: u8) -> Option<&BoxedNode<K, V>> {
        unsafe { self.0.get(key) }
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut BoxedNode<K, V>> {
        unsafe { self.0.get_mut(key) }
    }

    pub fn insert(&mut self, key: u8, value: BoxedNode<K, V>) -> Option<BoxedNode<K, V>> {
        unsafe { self.0.insert(key, value) }
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> BoxedNode<K, V> {
    pub fn display(&self, fmt: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        unsafe { self.0.display(fmt, depth) }
    }
}

impl<K: Key + ?Sized, V> Drop for BoxedNode<K, V> {
    fn drop(&mut self) {
        unsafe { self.0.drop_in_place() }
    }
}
