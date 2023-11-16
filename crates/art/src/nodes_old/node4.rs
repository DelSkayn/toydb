use std::{alloc::Layout, mem::ManuallyDrop, ptr::NonNull};

use super::{Node16, NodePtr};
use crate::header::{NodeHeader, NodeKind};

#[repr(C)]
pub struct Node4<V> {
    header: NodeHeader,
    ptr: [ManuallyDrop<NodePtr<V>>; 4],
    key: [u8; 4],
}

impl<V> Node4<V> {
    pub fn new(prefix: &[u8]) -> Self {
        Node4 {
            header: NodeHeader::new_for_prefix(NodeKind::Node4, prefix),
            ptr: [unsafe { NodePtr::dangling() }; 4],
            key: [0; 4],
        }
    }

    pub fn insert_at(&mut self, at: u8, ptr: NodePtr<V>) {
        self.key[self.header.len as usize] = at;
        self.ptr[self.header.len as usize] = ptr;
    }

    pub fn lookup_mut(&mut self, key: u8) -> Option<&mut NodePtr<V>> {
        let idx = self.key.iter().position(|x| *x == key)?;
        Some(&mut self.ptr[idx])
    }

    pub fn lookup(&self, key: u8) -> Option<&NodePtr<V>> {
        let idx = self.key.iter().position(|x| *x == key)?;
        Some(&self.ptr[idx])
    }

    pub unsafe fn grow(this: NonNull<Node4<V>>) -> NonNull<Node16<V>> {
        let new_ptr = NonNull::new(std::alloc::realloc(
            this.as_ptr().cast(),
            Layout::new::<Node4<V>>(),
            std::mem::size_of::<Node16<V>>(),
        ))
        .unwrap()
        .cast::<Node4<V>>();
        let mut dst_ptr = new_ptr.cast::<Node16<V>>();
        dst_ptr.as_mut().key[0..4].copy_from_slice(&new_ptr.as_ref().key);
        dst_ptr.as_mut().header.kind = NodeKind::Node16;
        dst_ptr
    }
}
