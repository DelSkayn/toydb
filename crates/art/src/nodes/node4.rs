use super::{BoxedNode, Node16, NodePtr};
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
pub struct Node4<K: Key + ?Sized, V> {
    pub header: NodeHeader<K>,
    pub ptr: [MaybeUninit<NodePtr<K, V>>; 4],
    pub keys: [u8; 4],
}

impl<K: Key + ?Sized, V> Node4<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> Self {
        Node4 {
            header: NodeHeader::new(key, range, NodeKind::Node4),
            keys: [0; 4],
            ptr: unsafe {
                std::mem::transmute::<
                    MaybeUninit<[NodePtr<K, V>; 4]>,
                    [MaybeUninit<NodePtr<K, V>>; 4],
                >(MaybeUninit::uninit())
            },
        }
    }

    pub fn is_full(&self) -> bool {
        self.header.data().len == 4
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len == 1
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

    pub unsafe fn insert_grow(
        this: &mut NodePtr<K, V>,
        key: u8,
        v: NodePtr<K, V>,
    ) -> Option<NodePtr<K, V>> {
        debug_assert_eq!(this.header().data().kind, NodeKind::Node4);
        let mut ptr = this.0.cast::<Self>();
        if let Some(x) = ptr.as_ref().keys[..ptr.as_ref().header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)
        {
            let res = std::mem::replace(&mut ptr.as_mut().ptr[x], MaybeUninit::new(v));
            return Some(res.assume_init());
        }

        if ptr.as_ref().is_full() {
            let mut ptr = Self::grow(ptr);
            this.0 = ptr.cast();
            ptr.as_mut().insert(key, v);
            return None;
        }

        let idx = ptr.as_ref().header.data().len;
        ptr.as_mut().header.data_mut().len += 1;
        ptr.as_mut().ptr[idx as usize].write(v);
        ptr.as_mut().keys[idx as usize] = key;
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

    unsafe fn grow(this: NonNull<Self>) -> NonNull<Node16<K, V>> {
        debug_assert!(this.as_ref().is_full());
        let ptr = this.as_ptr();
        let layout = Layout::new::<Self>();
        unsafe {
            let new_size = std::mem::size_of::<Node16<K, V>>();
            // because of the layout of nodes we only need to copy the keys into the right place
            // and alter the kind.
            let src_ptr = std::alloc::realloc(ptr.cast(), layout, new_size).cast::<Self>();
            let mut dst_ptr = NonNull::new(src_ptr.cast::<Node16<K, V>>()).unwrap();
            std::ptr::copy(
                addr_of_mut!((*src_ptr).keys).cast::<u8>(),
                addr_of_mut!((*dst_ptr.as_ptr()).keys).cast::<u8>(),
                4,
            );
            dst_ptr.as_mut().header.data_mut().kind = NodeKind::Node16;
            dst_ptr
        }
    }
}

impl<K: Key + ?Sized, V> Drop for Node4<K, V> {
    fn drop(&mut self) {
        for i in 0..self.header.data().len {
            unsafe { self.ptr[i as usize].assume_init_drop() }
        }
    }
}
