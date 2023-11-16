use super::{
    node48::{Node48, PtrUnion},
    BoxedNode, Node4, NodePtr,
};
use crate::{
    header::{NodeHeader, NodeKind},
    key::Key,
};
use std::{
    alloc::Layout,
    mem::MaybeUninit,
    ops::Range,
    ptr::{addr_of_mut, NonNull},
};

#[repr(C)]
pub struct Node16<K: Key, V> {
    pub header: NodeHeader<K>,
    pub ptr: [MaybeUninit<NodePtr<K, V>>; 16],
    pub keys: [u8; 16],
}

impl<K: Key, V> Node16<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> BoxedNode<Self> {
        BoxedNode::new(Node16 {
            header: NodeHeader::new(key, range, NodeKind::Node16),
            keys: [0; 16],
            ptr: unsafe {
                std::mem::transmute::<
                    MaybeUninit<[NodePtr<K, V>; 16]>,
                    [MaybeUninit<NodePtr<K, V>>; 16],
                >(MaybeUninit::uninit())
            },
        })
    }

    pub fn is_full(&self) -> bool {
        self.header.data().len == 16
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len < 5
    }

    pub fn get(&self, key: u8) -> Option<&NodePtr<K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;
        unsafe { Some(self.ptr[idx].assume_init_ref()) }
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut NodePtr<K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;
        unsafe { Some(self.ptr[idx].assume_init_mut()) }
    }

    pub fn insert(&mut self, key: u8, ptr: NodePtr<K, V>) -> Option<NodePtr<K, V>> {
        assert!(!self.is_full());
        if let Some(x) = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)
        {
            let res = std::mem::replace(&mut self.ptr[x], MaybeUninit::new(ptr));
            return unsafe { Some(res.assume_init()) };
        }
        let idx = self.header.data().len;
        self.header.data_mut().len += 1;
        self.ptr[idx as usize].write(ptr);
        self.keys[idx as usize] = key;
        None
    }

    pub fn remove(&mut self, key: u8) -> Option<NodePtr<K, V>> {
        let len = self.header.data().len;
        for i in 0..len {
            if self.keys[i as usize] == key {
                let ptr = unsafe { self.ptr.get_unchecked(i as usize).assume_init_read() };
                self.keys.swap(i as usize, (len - 1) as usize);
                self.ptr.swap(i as usize, (len - 1) as usize);
                return Some(ptr);
            }
        }
        None
    }

    pub fn shrink(this: BoxedNode<Self>) -> BoxedNode<Node4<K, V>> {
        assert!(this.should_shrink());
        // copy over the keys;
        let keys = <[u8; 4]>::try_from(&this.keys[0..4]).unwrap();
        let ptr = this.0;
        unsafe {
            let ptr = std::alloc::realloc(
                ptr.as_ptr().cast(),
                Layout::new::<Self>(),
                std::mem::size_of::<Node4<K, V>>(),
            )
            .cast::<Node4<K, V>>();
            let ptr = NonNull::new(ptr).unwrap();
            let mut res = BoxedNode(ptr);
            res.keys = keys;
            res
        }
    }

    pub fn grow(this: BoxedNode<Self>) -> BoxedNode<Node48<K, V>> {
        assert!(this.is_full());
        let ptr = this.0;
        unsafe {
            let src_ptr = std::alloc::realloc(
                ptr.as_ptr().cast(),
                Layout::new::<Self>(),
                std::mem::size_of::<Node48<K, V>>(),
            );
            let src_ptr = NonNull::new(src_ptr).unwrap().cast::<Self>();
            let dst_ptr = src_ptr.cast::<Node48<K, V>>();
            // set all the idx to max
            let idx_ptr = addr_of_mut!((*dst_ptr.as_ptr()).idx).cast::<u8>();
            std::ptr::write_bytes(idx_ptr, u8::MAX, 256);
            // write in the proper idx's
            for (idx, k) in src_ptr.as_ref().keys.iter().copied().enumerate() {
                idx_ptr.add(k as usize).write(idx as u8);
            }

            // intialize free list
            let ptr_ptr = addr_of_mut!((*dst_ptr.as_ptr()).ptr).cast::<PtrUnion<K, V>>();
            for i in 16..47u8 {
                ptr_ptr.add(i as usize).write(PtrUnion { free: i + 1 })
            }
            ptr_ptr.add(47).write(PtrUnion { free: u8::MAX });

            let mut res = BoxedNode(dst_ptr);
            res.header.data_mut().free = 16;
            res.header.data_mut().kind = NodeKind::Node48;
            res
        }
    }
}

impl<K: Key, V> Drop for Node16<K, V> {
    fn drop(&mut self) {
        for i in 0..self.header.data().len {
            unsafe { self.ptr[i as usize].assume_init_drop() }
        }
    }
}
