use crossbeam_epoch::Guard;
use crossbeam_utils::atomic::AtomicConsume;

use super::nodes::{NodeBox, NodeHeader, NodeRef};
use crate::{key::KeyBytes, prim::sync::atomic::AtomicPtr};
use std::{
    ptr::{self, NonNull},
    sync::atomic::Ordering,
};

#[repr(C)]
pub struct RootPtr<K: KeyBytes + ?Sized, V> {
    ptr: AtomicPtr<NodeHeader<K, V>>,
}

impl<K: KeyBytes + ?Sized, V> RootPtr<K, V> {
    pub fn null() -> Self {
        RootPtr {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }
    }

    #[must_use]
    pub fn clone(&self, guard: &Guard) -> Option<NodeBox<K, V>> {
        let _ = guard;
        unsafe {
            loop {
                let ptr = self.ptr.load_consume();
                let ptr = NonNull::new(ptr)?;
                let node: &NodeHeader<K, V> = ptr.as_ref();
                let mut count = node.ref_count.load_consume();
                loop {
                    if count == 0 {
                        break;
                    }

                    let new_count = count.checked_add(1).unwrap();
                    match node.ref_count.compare_exchange_weak(
                        count,
                        new_count,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => return Some(NodeBox::from_nonnull(ptr)),
                        Err(x) => count = x,
                    }
                }
            }
        }
    }

    pub fn exchange(
        &self,
        current: Option<NodeRef<K, V>>,
        new: Option<NodeBox<K, V>>,
        guard: &Guard,
    ) -> Result<(), Option<NodeBox<K, V>>> {
        let cur_ptr = current.map(|x| x.as_ptr()).unwrap_or_else(ptr::null_mut);

        let new_ptr = new
            .as_ref()
            .map(|p| p.as_ptr())
            .unwrap_or_else(ptr::null_mut);

        match self
            .ptr
            .compare_exchange(cur_ptr, new_ptr, Ordering::AcqRel, Ordering::Relaxed)
        {
            Ok(x) => {
                std::mem::forget(new);
                if let Some(x) = NonNull::new(x) {
                    unsafe {
                        let count = x.as_ref().ref_count.fetch_sub(1, Ordering::AcqRel);
                        if count == 1 {
                            guard.defer_unchecked(move || NodeBox::drop_in_place(x))
                        }
                    }
                }
                Ok(())
            }
            Err(_) => Err(new),
        }
    }
}

impl<K: KeyBytes + ?Sized, V> From<Option<NodeBox<K, V>>> for RootPtr<K, V> {
    fn from(value: Option<NodeBox<K, V>>) -> Self {
        Self {
            ptr: AtomicPtr::from(
                value
                    .map(|x| x.into_nonnull().as_ptr())
                    .unwrap_or_else(ptr::null_mut),
            ),
        }
    }
}

impl<K: KeyBytes + ?Sized, V> From<NodeBox<K, V>> for RootPtr<K, V> {
    fn from(value: NodeBox<K, V>) -> Self {
        Self {
            ptr: AtomicPtr::from(value.into_nonnull().as_ptr()),
        }
    }
}

impl<K: KeyBytes + ?Sized, V> Drop for RootPtr<K, V> {
    fn drop(&mut self) {
        let ptr = self.ptr.load(Ordering::Relaxed);
        NonNull::new(ptr).map(|x| unsafe { NodeBox::from_nonnull(x) });
    }
}

#[cfg(test)]
mod test {}
