use core::fmt;
use std::{
    alloc::Layout,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use super::{boxed_node::RawBoxedNode, NodeHeader, NodeType};

#[repr(transparent)]
pub struct RawOwnedNode<N: NodeType>(NonNull<N>);

impl<N: NodeType> Copy for RawOwnedNode<N> {}
impl<N: NodeType> Clone for RawOwnedNode<N> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: NodeType> RawOwnedNode<N> {
    pub unsafe fn alloc() -> Self {
        let ptr = std::alloc::alloc(Layout::new::<N>());
        Self(NonNull::new(ptr).unwrap().cast::<N>())
    }

    pub unsafe fn dealloc(ptr: Self) {
        std::alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<N>());
    }

    pub unsafe fn realloc<R: NodeType>(self) -> RawOwnedNode<R> {
        let new_size = std::mem::size_of::<R>();
        let old_layout = Layout::new::<N>();
        let ptr = std::alloc::realloc(self.as_ptr().cast(), old_layout, new_size).cast::<R>();
        let mut ptr = RawOwnedNode(NonNull::new(ptr).unwrap());
        ptr.header_mut().data_mut().kind = R::KIND;
        ptr
    }

    pub fn as_ptr(self) -> *mut N {
        self.0.as_ptr()
    }

    pub unsafe fn from_raw_node_ptr(ptr: RawBoxedNode<N::Key, N::Value>) -> Self {
        debug_assert!(ptr.is::<N>());
        Self(ptr.into_nonnull().cast())
    }

    pub unsafe fn from_header_ptr(ptr: NonNull<NodeHeader<N::Key>>) -> Self {
        debug_assert!(ptr.as_ref().is::<N>());
        Self(ptr.cast())
    }

    pub fn from_ptr(ptr: NonNull<N>) -> Self {
        RawOwnedNode(ptr)
    }

    pub fn into_ptr(self) -> NonNull<N> {
        self.0
    }

    pub unsafe fn as_mut(&mut self) -> &mut N {
        self.0.as_mut()
    }

    pub unsafe fn as_ref(&self) -> &N {
        self.0.as_ref()
    }

    pub fn into_boxed(self) -> RawBoxedNode<N::Key, N::Value> {
        RawBoxedNode::from_ptr(self.0.cast())
    }

    pub unsafe fn drop_in_place(self) {
        std::ptr::drop_in_place(self.0.as_ptr());
    }

    pub unsafe fn header_mut(&mut self) -> &mut NodeHeader<N::Key> {
        self.0.cast().as_mut()
    }

    pub unsafe fn copy_header_from<O>(&mut self, other: RawOwnedNode<O>)
    where
        O: NodeType<Key = N::Key, Value = N::Value>,
    {
        self.0
            .cast::<NodeHeader<N::Key>>()
            .as_ptr()
            .write(other.0.cast::<NodeHeader<N::Key>>().as_ptr().read());
        self.header_mut().data_mut().kind = N::KIND;
    }
}

impl<N: NodeType> From<NonNull<N>> for RawOwnedNode<N> {
    fn from(value: NonNull<N>) -> Self {
        RawOwnedNode(value)
    }
}

impl<N: NodeType> From<RawOwnedNode<N>> for NonNull<N> {
    fn from(value: RawOwnedNode<N>) -> Self {
        value.0
    }
}

/// An owned pointer to a specific type of node.
#[repr(transparent)]
pub struct OwnedNode<N: NodeType>(RawOwnedNode<N>);

impl<N: NodeType + fmt::Debug> fmt::Debug for OwnedNode<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { f.debug_tuple("BoxedNode").field(self.0.as_ref()).finish() }
    }
}

impl<N: NodeType> OwnedNode<N> {
    /// Allocates and then creates a new node.
    pub fn new(node: N) -> Self {
        unsafe {
            let this = RawOwnedNode::<N>::alloc();
            this.as_ptr().write(node);
            OwnedNode(this)
        }
    }

    pub unsafe fn from_raw(raw: RawOwnedNode<N>) -> Self {
        Self(raw)
    }

    pub fn into_raw(self) -> RawOwnedNode<N> {
        let res = self.0;
        std::mem::forget(self);
        res
    }

    //// Returns the underlying pointer.
    pub fn into_nonnull(self) -> NonNull<N> {
        self.into_raw().0
    }

    pub fn as_nonnull(&self) -> NonNull<N> {
        self.0 .0
    }
}

impl<N: NodeType> Drop for OwnedNode<N> {
    fn drop(&mut self) {
        unsafe {
            self.0.drop_in_place();
            RawOwnedNode::dealloc(self.0);
        }
    }
}

impl<N: NodeType> Deref for OwnedNode<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<N: NodeType> DerefMut for OwnedNode<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}
