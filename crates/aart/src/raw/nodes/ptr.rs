use super::{
    node16::Node16, node256::Node256, node48::Node48, Node, NodeHeader, NodeKind, NodeLeaf,
};
use crate::{
    key::KeyBytes,
    prim::alloc::{self, Layout},
    raw::nodes::Node4,
};
use bytemuck::ZeroableInOption;
use std::{marker::PhantomData, ops::Deref, ptr::NonNull, sync::atomic::Ordering};

pub struct NodeRef<'a, K: KeyBytes + ?Sized, V> {
    ptr: NonNull<NodeHeader<K, V>>,
    _marker: PhantomData<&'a NodeBox<K, V>>,
}

impl<K: KeyBytes + ?Sized, V> Clone for NodeRef<'_, K, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K: KeyBytes + ?Sized, V> Copy for NodeRef<'_, K, V> {}

impl<'a, K: KeyBytes + ?Sized, V> NodeRef<'a, K, V> {
    pub fn is<N>(self) -> bool
    where
        N: Node<Key = K, Value = V>,
    {
        self.kind() == N::KIND
    }

    /// # Safety
    /// Caller must ensure that this reference is actually a pointer to the give node type
    pub unsafe fn cast_unchecked<N>(self) -> &'a N
    where
        N: Node<Key = K, Value = V>,
    {
        unsafe { self.ptr.cast().as_ref() }
    }

    pub fn cast<N>(self) -> Option<&'a N>
    where
        N: Node<Key = K, Value = V>,
    {
        self.is::<N>().then(|| unsafe { self.cast_unchecked() })
    }

    pub fn as_ptr(self) -> *mut NodeHeader<K, V> {
        self.ptr.as_ptr()
    }

    pub fn as_nonnull(self) -> NonNull<NodeHeader<K, V>> {
        self.ptr
    }

    pub fn get(self, key: u8) -> Option<NodeRef<'a, K, V>> {
        unsafe {
            match self.data().kind() {
                NodeKind::Leaf => panic!("tried to retrieve a branch from a leaf node"),
                NodeKind::Node4 => self.cast_unchecked::<Node4<_, _>>().get(key),
                NodeKind::Node16 => self.cast_unchecked::<Node16<_, _>>().get(key),
                NodeKind::Node48 => self.cast_unchecked::<Node48<_, _>>().get(key),
                NodeKind::Node256 => self.cast_unchecked::<Node256<_, _>>().get(key),
            }
        }
    }

    pub fn copy_insert(self, key: u8, value: NodeBox<K, V>) -> NodeBox<K, V> {
        unsafe {
            match self.data().kind() {
                NodeKind::Leaf => panic!("tried to insert a branch in a leaf node"),
                NodeKind::Node4 => self.cast_unchecked::<Node4<_, _>>().copy_insert(key, value),
                NodeKind::Node16 => self
                    .cast_unchecked::<Node16<_, _>>()
                    .copy_insert(key, value),
                NodeKind::Node48 => self
                    .cast_unchecked::<Node48<_, _>>()
                    .copy_insert(key, value),
                NodeKind::Node256 => self
                    .cast_unchecked::<Node256<_, _>>()
                    .copy_insert(key, value),
            }
        }
    }

    pub fn copy_remove(self, key: u8) -> Option<NodeBox<K, V>> {
        unsafe {
            match self.data().kind() {
                NodeKind::Leaf => panic!("tried to remove a branch from a leaf node"),
                NodeKind::Node4 => self.cast_unchecked::<Node4<_, _>>().copy_remove(key),
                NodeKind::Node16 => self.cast_unchecked::<Node16<_, _>>().copy_remove(key),
                NodeKind::Node48 => self.cast_unchecked::<Node48<_, _>>().copy_remove(key),
                NodeKind::Node256 => self.cast_unchecked::<Node256<_, _>>().copy_remove(key),
            }
        }
    }

    pub fn copy_drop_prefix(self, drop: usize) -> NodeBox<K, V> {
        unsafe {
            match self.data().kind() {
                NodeKind::Leaf => self
                    .cast_unchecked::<NodeLeaf<_, _>>()
                    .copy_drop_prefix(drop),
                NodeKind::Node4 => self.cast_unchecked::<Node4<_, _>>().copy_drop_prefix(drop),
                NodeKind::Node16 => self.cast_unchecked::<Node16<_, _>>().copy_drop_prefix(drop),
                NodeKind::Node48 => self.cast_unchecked::<Node48<_, _>>().copy_drop_prefix(drop),
                NodeKind::Node256 => self
                    .cast_unchecked::<Node256<_, _>>()
                    .copy_drop_prefix(drop),
            }
        }
    }
}

impl<K: KeyBytes + ?Sized, V> Deref for NodeRef<'_, K, V> {
    type Target = NodeHeader<K, V>;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

#[repr(transparent)]
pub struct NodeBox<K: KeyBytes + ?Sized, V>(NonNull<NodeHeader<K, V>>);
unsafe impl<K: KeyBytes + ?Sized, V> ZeroableInOption for NodeBox<K, V> {}

unsafe impl<K: KeyBytes + ?Sized, V> Send for NodeBox<K, V> {}

impl<K: KeyBytes + ?Sized, V> NodeBox<K, V> {
    pub fn new<N>(node: N) -> Self
    where
        N: Node<Key = K, Value = V>,
    {
        debug_assert_eq!(node.header().kind(), N::KIND);
        unsafe {
            let ptr = NonNull::new(alloc::alloc(Layout::new::<N>()))
                .unwrap()
                .cast::<N>();
            ptr.as_ptr().write(node);
            NodeBox(ptr.cast())
        }
    }

    pub unsafe fn from_nonnull(ptr: NonNull<NodeHeader<K, V>>) -> Self {
        Self(ptr)
    }

    pub unsafe fn drop_in_place(ptr: NonNull<NodeHeader<K, V>>) {
        match ptr.as_ref().kind() {
            NodeKind::Leaf => {
                let ptr = ptr.cast::<NodeLeaf<K, V>>();
                unsafe {
                    std::ptr::drop_in_place(ptr.as_ptr());
                    alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<NodeLeaf<K, V>>())
                }
            }
            NodeKind::Node4 => {
                let ptr = ptr.cast::<Node4<K, V>>();
                unsafe {
                    std::ptr::drop_in_place(ptr.as_ptr());
                    alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<Node4<K, V>>())
                }
            }
            NodeKind::Node16 => {
                let ptr = ptr.cast::<Node16<K, V>>();
                unsafe {
                    std::ptr::drop_in_place(ptr.as_ptr());
                    alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<Node16<K, V>>())
                }
            }
            NodeKind::Node48 => {
                let ptr = ptr.cast::<Node48<K, V>>();
                unsafe {
                    std::ptr::drop_in_place(ptr.as_ptr());
                    alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<Node48<K, V>>())
                }
            }
            NodeKind::Node256 => {
                let ptr = ptr.cast::<Node256<K, V>>();
                unsafe {
                    std::ptr::drop_in_place(ptr.as_ptr());
                    alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<Node256<K, V>>())
                }
            }
        }
    }

    pub fn as_ref(&self) -> NodeRef<K, V> {
        NodeRef {
            ptr: self.0,
            _marker: PhantomData,
        }
    }

    pub fn as_ptr(&self) -> *mut NodeHeader<K, V> {
        self.0.as_ptr()
    }

    pub fn into_nonnull(self) -> NonNull<NodeHeader<K, V>> {
        let res = self.0;
        std::mem::forget(self);
        res
    }
}

impl<K: KeyBytes + ?Sized, V> Clone for NodeBox<K, V> {
    fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::AcqRel);
        NodeBox(self.0)
    }
}

impl<K: KeyBytes + ?Sized, V> Deref for NodeBox<K, V> {
    type Target = NodeHeader<K, V>;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<K: KeyBytes + ?Sized, V> Drop for NodeBox<K, V> {
    fn drop(&mut self) {
        let count = self.ref_count.fetch_sub(1, Ordering::AcqRel);

        if count == 1 {
            unsafe { Self::drop_in_place(self.0) }
        }
    }
}
