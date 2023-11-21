use core::fmt;
use std::{
    alloc::Layout,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

#[repr(transparent)]
pub struct BoxedNode<N>(NonNull<N>);

impl<N> fmt::Debug for BoxedNode<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BoxedNode").field(&self.0).finish()
    }
}

impl<N> BoxedNode<N> {
    pub unsafe fn alloc() -> NonNull<N> {
        let ptr = std::alloc::alloc(Layout::new::<N>());
        NonNull::new(ptr).unwrap().cast::<N>()
    }

    pub unsafe fn dealloc(ptr: NonNull<N>) {
        std::alloc::dealloc(ptr.as_ptr().cast(), Layout::new::<N>());
    }

    pub fn new(node: N) -> Self {
        unsafe {
            let this = Self::alloc();
            this.as_ptr().write(node);
            BoxedNode(this)
        }
    }

    pub fn into_raw(self) -> NonNull<N> {
        let res = self.0;
        std::mem::forget(self);
        res
    }

    pub fn as_raw(&self) -> NonNull<N> {
        self.0
    }

    pub unsafe fn from_raw(ptr: NonNull<N>) -> Self {
        BoxedNode(ptr)
    }
}

impl<N> Drop for BoxedNode<N> {
    fn drop(&mut self) {
        unsafe { Self::dealloc(self.0) }
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
