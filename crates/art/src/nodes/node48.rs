use super::{node256::Node256, BoxedNode, NodePtr};
use crate::{
    header::{NodeHeader, NodeKind},
    key::Key,
    nodes::Node16,
};
use std::{
    mem::{ManuallyDrop, MaybeUninit},
    ops::Range,
    ptr::addr_of_mut,
};

pub union PtrUnion<K: Key, V> {
    pub free: u8,
    pub ptr: ManuallyDrop<NodePtr<K, V>>,
}

#[repr(C)]
pub struct Node48<K: Key, V> {
    pub header: NodeHeader<K>,
    pub ptr: [PtrUnion<K, V>; 48],
    pub idx: [u8; 256],
}

impl<K: Key, V> Node48<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> BoxedNode<Self> {
        BoxedNode::new(Node48 {
            header: NodeHeader::new(key, range, NodeKind::Node48),
            ptr: unsafe { MaybeUninit::zeroed().assume_init() },
            idx: [255; 256],
        })
    }

    pub fn is_full(&self) -> bool {
        self.header.data().len == 48
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len < 16
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut NodePtr<K, V>> {
        let idx = self.idx[key as usize];
        if idx != u8::MAX {
            unsafe { Some(&mut self.ptr[idx as usize].ptr) }
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: u8, ptr: NodePtr<K, V>) -> Option<NodePtr<K, V>> {
        assert!(!self.is_full());
        let idx = self.idx[key as usize];
        if idx == u8::MAX {
            let next_free = self.header.data().free;
            self.header.data_mut().free = unsafe { self.ptr[next_free as usize].free };
            self.header.data_mut().len += 1;

            self.idx[key as usize] = next_free;
            self.ptr[next_free as usize].ptr = ManuallyDrop::new(ptr);

            return None;
        }
        unsafe {
            let res = std::mem::replace(&mut self.ptr[idx as usize].ptr, ManuallyDrop::new(ptr));
            Some(ManuallyDrop::into_inner(res))
        }
    }

    pub fn remove(&mut self, key: u8) -> Option<NodePtr<K, V>> {
        if self.idx[key as usize] == u8::MAX {
            return None;
        }
        let idx = self.idx[key as usize];
        self.idx[key as usize] = u8::MAX;
        self.header.data_mut().len -= 1;
        Some(unsafe { ManuallyDrop::take(&mut self.ptr[idx as usize].ptr) })
    }

    pub fn shrink(mut this: BoxedNode<Self>) -> BoxedNode<Node16<K, V>> {
        assert!(this.should_shrink());

        unsafe {
            let ptr = BoxedNode::<Node16<K, V>>::alloc();
            let mut key_ptr = addr_of_mut!((*ptr.as_ptr()).keys).cast::<u8>();
            let mut ptr_ptr =
                addr_of_mut!((*ptr.as_ptr()).ptr).cast::<MaybeUninit<NodePtr<K, V>>>();

            for i in 0..256 {
                let idx = this.idx[i];
                if idx != u8::MAX {
                    key_ptr.write(i as u8);
                    (*ptr_ptr).write(ManuallyDrop::take(&mut this.ptr[idx as usize].ptr));
                    key_ptr = key_ptr.add(1);
                    ptr_ptr = ptr_ptr.add(1);
                }
            }
            let this_ptr = this.0;
            // copy over the header
            std::ptr::copy(
                ptr.cast::<NodeHeader<K>>().as_ptr(),
                this_ptr.cast::<NodeHeader<K>>().as_ptr(),
                1,
            );

            let mut res = BoxedNode(ptr);
            res.header.data_mut().kind = NodeKind::Node16;
            BoxedNode::dealloc(this_ptr);
            res
        }
    }

    pub fn grow(mut this: BoxedNode<Self>) -> BoxedNode<Node256<K, V>> {
        assert!(this.is_full());
        unsafe {
            let new_ptr = BoxedNode::<Node256<K, V>>::alloc();

            let ptr_ptr = addr_of_mut!((*new_ptr.as_ptr()).ptr).cast::<Option<NodePtr<K, V>>>();
            // init zero
            std::ptr::write_bytes(ptr_ptr, 0, 256);
            // copy over pointer
            for i in 0..=255u8 {
                let idx = this.idx[i as usize];
                if idx != u8::MAX {
                    ptr_ptr
                        .add(i as usize)
                        .write(Some(ManuallyDrop::take(&mut this.ptr[idx as usize].ptr)));
                }
            }

            // move out
            let ptr = this.0;

            // copy over header
            std::ptr::copy(
                ptr.cast::<NodeHeader<K>>().as_ptr(),
                new_ptr.cast::<NodeHeader<K>>().as_ptr(),
                1,
            );
            // ownership tranfered
            let mut res = BoxedNode(new_ptr);
            BoxedNode::dealloc(ptr);
            // make the len smaller so it will fit u8
            res.header.data_mut().len = 47;
            res.header.data_mut().kind = NodeKind::Node256;
            res
        }
    }
}

impl<K: Key, V> Drop for Node48<K, V> {
    fn drop(&mut self) {
        self.idx
            .into_iter()
            .filter(|x| *x < 48)
            .for_each(|x| unsafe { ManuallyDrop::drop(&mut self.ptr[x as usize].ptr) })
    }
}
