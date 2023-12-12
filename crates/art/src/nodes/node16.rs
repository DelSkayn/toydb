use super::{
    node48::{Node48, PtrUnion},
    owned_node::RawOwnedNode,
    BoxedNode, Node4, NodeHeader, NodeKind, NodeType, OwnedNode, RawBoxedNode,
};
use crate::key::{Key, KeyStorage};
use core::fmt;
use std::{mem::MaybeUninit, ops::Range, ptr::addr_of_mut};

#[repr(C)]
pub struct Node16<K: Key + ?Sized, V> {
    pub header: NodeHeader<K>,
    pub ptr: [MaybeUninit<RawBoxedNode<K, V>>; 16],
    pub keys: [u8; 16],
}

unsafe impl<K: Key + ?Sized, V> NodeType for Node16<K, V> {
    const KIND: NodeKind = NodeKind::Node16;
    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> Node16<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> OwnedNode<Self> {
        OwnedNode::new(Node16 {
            header: NodeHeader::new::<Self>(key, range),
            keys: [0; 16],
            ptr: [MaybeUninit::uninit(); 16],
        })
    }

    pub fn is_full(&self) -> bool {
        self.header.data().len == 16
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len < 5
    }

    pub fn get(&self, key: u8) -> Option<&BoxedNode<K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;
        unsafe { Some(BoxedNode::from_raw_ref(self.ptr[idx].assume_init_ref())) }
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut BoxedNode<K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;
        unsafe { Some(BoxedNode::from_raw_mut(self.ptr[idx].assume_init_mut())) }
    }

    pub fn insert(&mut self, key: u8, ptr: BoxedNode<K, V>) -> Option<BoxedNode<K, V>> {
        assert!(!self.is_full());
        if let Some(x) = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)
        {
            let res = std::mem::replace(&mut self.ptr[x], MaybeUninit::new(ptr.into_raw()));
            return unsafe { Some(BoxedNode::from_raw(res.assume_init())) };
        }

        let idx = self.header.data().len;
        self.header.data_mut().len += 1;
        self.ptr[idx as usize] = MaybeUninit::new(ptr.into_raw());
        self.keys[idx as usize] = key;
        None
    }

    /// # Safety
    /// Caller must ensure that the given `NodePtr` is a pointer to `Node4`.
    pub unsafe fn insert_grow(
        this: &mut RawBoxedNode<K, V>,
        key: u8,
        v: BoxedNode<K, V>,
    ) -> Option<BoxedNode<K, V>> {
        debug_assert!(this.is::<Self>());

        if let Some(x) = this.as_ref::<Self>().keys
            [..this.as_ref::<Self>().header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)
        {
            let res = std::mem::replace(
                &mut this.as_mut::<Self>().ptr[x],
                MaybeUninit::new(v.into_raw()),
            );
            return Some(BoxedNode::from_raw(res.assume_init()));
        }

        if this.as_ref::<Self>().is_full() {
            let mut ptr = Self::grow(this.into_owned());
            ptr.as_mut().insert(key, v);
            *this = ptr.into_boxed();
            return None;
        }

        let idx = this.as_ref::<Self>().header.data().len;
        this.as_mut::<Self>().header.data_mut().len += 1;
        this.as_mut::<Self>().ptr[idx as usize].write(v.into_raw());
        this.as_mut::<Self>().keys[idx as usize] = key;
        None
    }

    pub fn remove(&mut self, key: u8) -> Option<BoxedNode<K, V>> {
        let len = self.header.data().len;
        for i in 0..len {
            if self.keys[i as usize] == key {
                let ptr = unsafe { self.ptr.get_unchecked(i as usize).assume_init() };
                self.keys.swap(i as usize, (len - 1) as usize);
                self.ptr.swap(i as usize, (len - 1) as usize);
                self.header.data_mut().len -= 1;
                return unsafe { Some(BoxedNode::from_raw(ptr)) };
            }
        }
        None
    }

    pub unsafe fn shrink(this: RawOwnedNode<Self>) -> RawOwnedNode<Node4<K, V>> {
        // ensure there is enough space for all the nodes.
        assert!(this.as_ref().should_shrink());
        // copy over the keys to be later copied back.
        let keys = <[u8; 4]>::try_from(&this.as_ref().keys[0..4]).unwrap();

        let mut ptr = this.realloc::<Node4<K, V>>();
        ptr.as_mut().keys = keys;
        ptr
    }

    unsafe fn grow(this: RawOwnedNode<Self>) -> RawOwnedNode<Node48<K, V>> {
        assert!(this.as_ref().is_full());

        let mut new_ptr = this.realloc::<Node48<K, V>>();

        let src_ptr = new_ptr.as_nonnull().cast::<Self>();

        // set all the idx to max
        let idx_ptr = addr_of_mut!(*((*new_ptr.as_ptr()).idx.as_mut_ptr())).cast::<u8>();
        std::ptr::write_bytes(idx_ptr, u8::MAX, 256);

        // write in the proper idx's
        for (idx, k) in src_ptr.as_ref().keys.iter().copied().enumerate() {
            idx_ptr.add(k as usize).write(idx as u8);
        }

        // intialize free list
        let ptr_ptr = addr_of_mut!((*new_ptr.as_ptr()).ptr).cast::<PtrUnion<K, V>>();
        for i in 16..47u8 {
            ptr_ptr.add(i as usize).write(PtrUnion { free: i + 1 })
        }

        ptr_ptr.add(47).write(PtrUnion { free: u8::MAX });

        new_ptr.as_mut().header.data_mut().free = 16;
        new_ptr
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> Node16<K, V> {
    pub fn display(&self, fmt: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        writeln!(
            fmt,
            "NODE16: len={},prefix={:?}",
            self.header.storage().data().len,
            self.header.storage().prefix()
        )?;
        for i in 0..self.header.storage().data().len {
            for _ in 0..depth {
                fmt.write_str("  ")?;
            }
            write!(fmt, "[{}] = ", self.keys[i as usize])?;
            unsafe {
                self.ptr[i as usize]
                    .assume_init_ref()
                    .display(fmt, depth + 1)?;
            }
        }
        Ok(())
    }
}

impl<K: Key + ?Sized, V> Drop for Node16<K, V> {
    fn drop(&mut self) {
        for i in 0..self.header.data().len {
            unsafe { self.ptr[i as usize].assume_init().drop_in_place() }
        }
    }
}
