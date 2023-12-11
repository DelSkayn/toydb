use super::{BoxedNode, Node16, NodeHeader, NodeKind, NodeType, RawBoxedNode};
use crate::{
    key::{Key, KeyStorage},
    nodes::owned_node::RawOwnedNode,
};
use std::{fmt, mem::MaybeUninit, ops::Range, ptr::addr_of_mut};

#[repr(C)]
pub struct Node4<K: Key + ?Sized, V> {
    pub header: NodeHeader<K>,
    pub ptr: [MaybeUninit<RawBoxedNode<K, V>>; 4],
    pub keys: [u8; 4],
}

unsafe impl<K: Key + ?Sized, V> NodeType for Node4<K, V> {
    const KIND: super::NodeKind = NodeKind::Node4;
    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> Node4<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> Self {
        Node4 {
            header: NodeHeader::new::<Self>(key, range),
            keys: [0; 4],
            ptr: [MaybeUninit::uninit(); 4],
        }
    }

    pub fn is_full(&self) -> bool {
        self.header.data().len == 4
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len == 1
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

    pub unsafe fn insert_grow(
        this: &mut RawBoxedNode<K, V>,
        key: u8,
        v: BoxedNode<K, V>,
    ) -> Option<BoxedNode<K, V>> {
        debug_assert!(this.is::<Self>());

        if let Some(x) = this.as_ref::<Self>().keys[..this.header().data().len as usize]
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
            let mut ptr = Self::grow(this.into_owned::<Self>());
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

                // swap remove
                self.keys.swap(i as usize, (len - 1) as usize);
                self.ptr.swap(i as usize, (len - 1) as usize);

                self.header.data_mut().len -= 1;
                return unsafe { Some(BoxedNode::from_raw(ptr)) };
            }
        }
        None
    }

    unsafe fn fold(mut this: RawOwnedNode<Self>) -> RawBoxedNode<K, V> {
        debug_assert_eq!(this.as_ref().header.data().len, 1);
        // move out the child
        let mut child = this.as_mut().ptr[0].assume_init();

        // append the current prefix and the key of the child to the prefix of the child.
        child.header_mut().storage_mut().prepend_prefix(
            this.as_ref().header.storage().prefix(),
            this.as_ref().keys[0],
        );

        this.drop_in_place();

        child
    }

    unsafe fn grow(this: RawOwnedNode<Self>) -> RawOwnedNode<Node16<K, V>> {
        debug_assert!(this.as_ref().is_full());

        let new_ptr = this.realloc::<Node16<K, V>>();
        let src_ptr = new_ptr.as_ptr().cast::<Self>();
        std::ptr::copy(
            addr_of_mut!((*src_ptr).keys).cast::<u8>(),
            addr_of_mut!((*new_ptr.as_ptr()).keys).cast::<u8>(),
            4,
        );
        new_ptr
    }
}

impl<K: Key + ?Sized, V: fmt::Display> Node4<K, V> {
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

impl<K: Key + ?Sized, V> Drop for Node4<K, V> {
    fn drop(&mut self) {
        for i in 0..self.header.data().len {
            unsafe { self.ptr[i as usize].assume_init().drop_in_place() }
        }
    }
}
