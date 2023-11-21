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
pub struct Node16<K: Key + ?Sized, V> {
    pub header: NodeHeader<K>,
    pub ptr: [MaybeUninit<NodePtr<K, V>>; 16],
    pub keys: [u8; 16],
}

impl<K: Key + ?Sized, V> Node16<K, V> {
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

    /// # Safety
    /// Caller must ensure that the given `NodePtr` is a pointer to `Node4`.
    pub unsafe fn insert_grow(
        this: &mut NodePtr<K, V>,
        key: u8,
        v: NodePtr<K, V>,
    ) -> Option<NodePtr<K, V>> {
        let mut ptr = this.0.cast::<Node16<K, V>>();

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
            ptr.as_mut().insert(key, v);
            this.0 = ptr.cast();
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

    pub fn shrink(this: BoxedNode<Self>) -> BoxedNode<Node4<K, V>> {
        todo!();
        /*
        assert!(this.should_shrink());
        // copy over the keys;
        let keys = <[u8; 4]>::try_from(&this.keys[0..4]).unwrap();
        let ptr = this.as_raw();
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
        */
    }

    unsafe fn grow(this: NonNull<Self>) -> NonNull<Node48<K, V>> {
        assert!(this.as_ref().is_full());
        let ptr = this;
        let src_ptr = std::alloc::realloc(
            ptr.as_ptr().cast(),
            Layout::new::<Self>(),
            std::mem::size_of::<Node48<K, V>>(),
        );
        let src_ptr = NonNull::new(src_ptr).unwrap().cast::<Self>();
        let mut dst_ptr = src_ptr.cast::<Node48<K, V>>();
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

        dst_ptr.as_mut().header.data_mut().free = 16;
        dst_ptr.as_mut().header.data_mut().kind = NodeKind::Node48;
        dst_ptr
    }
}

impl<K: Key + ?Sized, V> Drop for Node16<K, V> {
    fn drop(&mut self) {
        for i in 0..self.header.data().len {
            unsafe { self.ptr[i as usize].assume_init_drop() }
        }
    }
}
