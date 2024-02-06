use std::{
    alloc::Layout,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::key::Key;

use super::nodes::{LeafNode, Node16, Node256, Node4, Node48, NodeHeader, NodeKind, NodeType};

/// # Safety
/// These traits are only for internal uses and should be implemented outside of this crate.
pub unsafe trait ValidPtr {}

/// # Safety
/// These traits are only for internal uses and should be implemented outside of this crate.
pub unsafe trait MutValuePtr: ValidPtr {}

/// # Safety
/// These traits are only for internal uses and should be implemented outside of this crate.
pub unsafe trait MutablePtr: MutValuePtr {}

/// # Safety
/// These traits are only for internal uses and should be implemented outside of this crate.
pub unsafe trait OwnedPtr: MutablePtr {}

unsafe impl<'a> ValidPtr for Borrow<'a> {}
unsafe impl<'a> ValidPtr for MutValue<'a> {}
unsafe impl<'a> ValidPtr for BorrowMut<'a> {}
unsafe impl ValidPtr for Owned {}

unsafe impl<'a> MutValuePtr for MutValue<'a> {}
unsafe impl<'a> MutValuePtr for BorrowMut<'a> {}
unsafe impl MutValuePtr for Owned {}

unsafe impl<'a> MutablePtr for BorrowMut<'a> {}
unsafe impl MutablePtr for Owned {}

unsafe impl OwnedPtr for Owned {}

/// A marker type signifying that the relation ship of pointer to its pointing node is unknown
#[derive(Clone, Copy)]
pub enum Unknown {}

/// A marker type signifying that pointer has borrowed the node immutable
#[derive(Clone, Copy)]
pub struct Borrow<'a>(PhantomData<&'a ()>);

/// A marker type signifying that pointer has borrowed the node immutable
#[derive(Clone, Copy)]
pub struct MutValue<'a>(PhantomData<&'a ()>);

/// A marker type signifying that pointer has borrowed the node mutably
pub struct BorrowMut<'a>(PhantomData<&'a mut ()>);

/// A marker type signifying that pointer has ownership of the node
pub enum Owned {}

pub struct TypedNodePtr<Owner, N: NodeType> {
    owner: PhantomData<Owner>,
    ptr: NonNull<N>,
}

impl<O: Clone, N: NodeType> Clone for TypedNodePtr<O, N> {
    fn clone(&self) -> Self {
        TypedNodePtr {
            owner: PhantomData,
            ptr: self.ptr,
        }
    }
}

impl<O: Copy, N: NodeType> Copy for TypedNodePtr<O, N> {}

impl<O, N: NodeType> TypedNodePtr<O, N> {
    pub unsafe fn from_nonnull(ptr: NonNull<N>) -> Self {
        TypedNodePtr {
            owner: PhantomData,
            ptr,
        }
    }

    pub unsafe fn from_ptr(ptr: *mut N) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| TypedNodePtr {
            owner: PhantomData,
            ptr,
        })
    }

    pub fn as_ptr(&self) -> *mut N {
        self.ptr.as_ptr()
    }

    pub fn as_nonnull(&self) -> NonNull<N> {
        self.ptr
    }

    pub fn as_unknown(&self) -> TypedNodePtr<Unknown, N> {
        TypedNodePtr::<Unknown, _>::unknown_from_nonnull(self.as_nonnull())
    }

    pub fn erase_type(self) -> NodePtr<O, N::Key, N::Value> {
        NodePtr {
            ptr: self.ptr.cast(),
            owner: PhantomData,
        }
    }
}

impl<N: NodeType> TypedNodePtr<Unknown, N> {
    pub const fn dangling() -> Self {
        TypedNodePtr {
            owner: PhantomData,
            ptr: NonNull::dangling(),
        }
    }

    pub fn alloc() -> Self {
        unsafe {
            let ptr = std::alloc::alloc(Layout::new::<N>());
            TypedNodePtr {
                owner: PhantomData,
                ptr: NonNull::new(ptr).unwrap().cast::<N>(),
            }
        }
    }

    pub fn unknown_from_nonnull(ptr: NonNull<N>) -> Self {
        TypedNodePtr {
            owner: PhantomData,
            ptr,
        }
    }

    pub fn unknown_from_ptr(ptr: *mut N) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| TypedNodePtr {
            owner: PhantomData,
            ptr,
        })
    }

    pub unsafe fn dealloc(ptr: Self) {
        std::alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<N>());
    }

    pub unsafe fn drop_in_place(self) {
        std::ptr::drop_in_place(self.as_ptr())
    }

    pub unsafe fn free(ptr: Self) {
        ptr.drop_in_place();
        Self::dealloc(ptr)
    }

    pub unsafe fn assume_owned(self) -> OwnedTypedNodePtr<N> {
        OwnedTypedNodePtr {
            ptr: TypedNodePtr {
                owner: PhantomData,
                ptr: self.ptr,
            },
        }
    }

    pub unsafe fn assume_ownership<O>(self) -> TypedNodePtr<O, N> {
        TypedNodePtr {
            owner: PhantomData,
            ptr: self.ptr,
        }
    }
}

impl<O: ValidPtr, N: NodeType> TypedNodePtr<O, N> {
    pub fn header(&self) -> &NodeHeader<N::Key, N::Value> {
        unsafe { self.ptr.cast().as_ref() }
    }

    pub fn as_ref(&self) -> &N {
        unsafe { self.ptr.as_ref() }
    }
}

impl<O: MutablePtr, N: NodeType> TypedNodePtr<O, N> {
    pub fn header_mut(&mut self) -> &mut NodeHeader<N::Key, N::Value> {
        unsafe { self.ptr.cast().as_mut() }
    }

    pub fn as_mut(&mut self) -> &mut N {
        unsafe { self.ptr.as_mut() }
    }
}

impl<N: NodeType> TypedNodePtr<Owned, N> {
    pub fn borrow(&self) -> TypedNodePtr<Borrow, N> {
        unsafe { TypedNodePtr::from_nonnull(self.ptr) }
    }

    pub fn borrow_mut(&self) -> TypedNodePtr<BorrowMut, N> {
        unsafe { TypedNodePtr::from_nonnull(self.ptr) }
    }
}

pub struct OwnedTypedNodePtr<N: NodeType> {
    ptr: TypedNodePtr<Owned, N>,
}

impl<N: NodeType> OwnedTypedNodePtr<N> {
    pub fn new(node: N) -> Self {
        let ptr = TypedNodePtr::<Unknown, N>::alloc();
        unsafe { ptr.as_ptr().write(node) };
        unsafe { ptr.assume_owned() }
    }

    pub fn into_unknown(self) -> TypedNodePtr<Unknown, N> {
        let res = self.ptr.as_unknown();
        std::mem::forget(self);
        res
    }

    pub fn erase_type(self) -> OwnedNodePtr<N::Key, N::Value> {
        unsafe { self.into_unknown().erase_type().assume_owned() }
    }
}

impl<N: NodeType> DerefMut for OwnedTypedNodePtr<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ptr
    }
}

impl<N: NodeType> Deref for OwnedTypedNodePtr<N> {
    type Target = TypedNodePtr<Owned, N>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<N: NodeType> Drop for OwnedTypedNodePtr<N> {
    fn drop(&mut self) {
        unsafe {
            let p = self.ptr.as_unknown();
            p.drop_in_place();
            TypedNodePtr::<Unknown, N>::dealloc(p);
        }
    }
}

impl<O: ValidPtr, N: NodeType> Deref for TypedNodePtr<O, N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<O: MutablePtr, N: NodeType> DerefMut for TypedNodePtr<O, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

pub struct NodePtr<Owner, K: Key + ?Sized, V> {
    owner: PhantomData<Owner>,
    pub(crate) ptr: NonNull<NodeHeader<K, V>>,
}

impl<O: Clone, K: Key + ?Sized, V> Clone for NodePtr<O, K, V> {
    fn clone(&self) -> Self {
        NodePtr {
            owner: PhantomData,
            ptr: self.ptr,
        }
    }
}

impl<O: Copy, K: Key + ?Sized, V> Copy for NodePtr<O, K, V> {}

impl<O, K: Key + ?Sized, V> NodePtr<O, K, V> {
    pub fn as_ptr(&self) -> *mut NodeHeader<K, V> {
        self.ptr.as_ptr()
    }

    pub fn as_nonnull(&self) -> NonNull<NodeHeader<K, V>> {
        self.ptr
    }

    pub fn as_unknown(&self) -> NodePtr<Unknown, K, V> {
        NodePtr {
            owner: PhantomData,
            ptr: self.ptr,
        }
    }

    pub unsafe fn cast_unchecked<N>(self) -> TypedNodePtr<O, N>
    where
        N: NodeType<Key = K, Value = V>,
    {
        TypedNodePtr {
            owner: PhantomData,
            ptr: self.ptr.cast(),
        }
    }

    pub unsafe fn cast_ref_unchecked<N>(&self) -> TypedNodePtr<Borrow, N>
    where
        N: NodeType<Key = K, Value = V>,
    {
        TypedNodePtr {
            owner: PhantomData,
            ptr: self.ptr.cast(),
        }
    }

    pub unsafe fn cast_mut_unchecked<N>(&mut self) -> TypedNodePtr<BorrowMut, N>
    where
        N: NodeType<Key = K, Value = V>,
    {
        TypedNodePtr {
            owner: PhantomData,
            ptr: self.ptr.cast(),
        }
    }
}

impl<K: Key + ?Sized, V> NodePtr<Unknown, K, V> {
    pub fn dangling() -> Self {
        NodePtr {
            owner: PhantomData,
            ptr: NonNull::dangling(),
        }
    }

    pub unsafe fn free(ptr: Self) {
        match ptr.assume_ownership::<Borrow>().header().kind() {
            NodeKind::Leaf => {
                TypedNodePtr::free(ptr.cast_unchecked::<LeafNode<K, V>>().as_unknown())
            }
            NodeKind::Node4 => TypedNodePtr::free(ptr.cast_unchecked::<Node4<K, V>>().as_unknown()),
            NodeKind::Node16 => {
                TypedNodePtr::free(ptr.cast_unchecked::<Node16<K, V>>().as_unknown())
            }
            NodeKind::Node48 => {
                TypedNodePtr::free(ptr.cast_unchecked::<Node48<K, V>>().as_unknown())
            }
            NodeKind::Node256 => {
                TypedNodePtr::free(ptr.cast_unchecked::<Node256<K, V>>().as_unknown())
            }
        }
    }

    pub unsafe fn assume_owned(self) -> OwnedNodePtr<K, V> {
        OwnedNodePtr {
            ptr: NodePtr {
                owner: PhantomData,
                ptr: self.ptr,
            },
        }
    }

    pub unsafe fn assume_ownership<O>(self) -> NodePtr<O, K, V> {
        NodePtr {
            owner: PhantomData,
            ptr: self.ptr,
        }
    }

    pub unsafe fn take_header(self) -> NodeHeader<K, V> {
        self.ptr.as_ptr().read()
    }
}

impl<O: ValidPtr, K: Key + ?Sized, V> NodePtr<O, K, V> {
    pub fn header(&self) -> &NodeHeader<K, V> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn is<N>(&self) -> bool
    where
        N: NodeType<Key = K, Value = V>,
    {
        self.header().is::<N>()
    }

    pub fn cast<N>(self) -> Option<TypedNodePtr<O, N>>
    where
        N: NodeType<Key = K, Value = V>,
    {
        self.is::<N>().then(|| unsafe { self.cast_unchecked() })
    }

    pub fn cast_ref<N>(&self) -> Option<TypedNodePtr<Borrow, N>>
    where
        N: NodeType<Key = K, Value = V>,
    {
        self.is::<N>().then(|| unsafe { self.cast_ref_unchecked() })
    }
}

impl<O: MutablePtr, K: Key + ?Sized, V> NodePtr<O, K, V> {
    pub fn header_mut(&mut self) -> &mut NodeHeader<K, V> {
        unsafe { self.ptr.as_mut() }
    }

    pub fn as_borrow(&self) -> NodePtr<Borrow, K, V> {
        unsafe { self.as_unknown().assume_ownership() }
    }

    pub fn cast_mut<N>(&mut self) -> Option<TypedNodePtr<BorrowMut, N>>
    where
        N: NodeType<Key = K, Value = V>,
    {
        self.is::<N>().then(|| unsafe { self.cast_mut_unchecked() })
    }
}

pub struct OwnedNodePtr<K: Key + ?Sized, V> {
    ptr: NodePtr<Owned, K, V>,
}

impl<K: Key + ?Sized, V> OwnedNodePtr<K, V> {
    pub fn into_unknown(self) -> NodePtr<Unknown, K, V> {
        let res = self.ptr.as_unknown();
        std::mem::forget(self);
        res
    }

    pub fn cast_owned<N>(self) -> Option<OwnedTypedNodePtr<N>>
    where
        N: NodeType<Key = K, Value = V>,
    {
        self.is::<N>()
            .then(|| unsafe { self.into_unknown().cast_unchecked().assume_owned() })
    }
}

impl<K: Key + ?Sized, V> DerefMut for OwnedNodePtr<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ptr
    }
}

impl<K: Key + ?Sized, V> Deref for OwnedNodePtr<K, V> {
    type Target = NodePtr<Owned, K, V>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<K: Key + ?Sized, V> Drop for OwnedNodePtr<K, V> {
    fn drop(&mut self) {
        unsafe { NodePtr::<Unknown, K, V>::free(self.ptr.as_unknown()) }
    }
}
