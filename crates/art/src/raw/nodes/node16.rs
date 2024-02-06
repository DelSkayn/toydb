use crate::{
    key::Key,
    raw::{
        nodes::Node48,
        ptr::{
            MutablePtr, NodePtr, OwnedNodePtr, OwnedTypedNodePtr, TypedNodePtr, Unknown, ValidPtr,
        },
    },
};
use std::{
    mem::MaybeUninit,
    ops::Range,
    ptr::{addr_of, addr_of_mut},
};

use super::{Node4, NodeHeader, NodeKind, NodeType};

#[repr(C)]
pub struct Node16<K: Key + ?Sized, V> {
    pub header: NodeHeader<K, V>,
    pub ptr: [NodePtr<Unknown, K, V>; 16],
    pub keys: [u8; 16],
}

unsafe impl<K: Key + ?Sized, V> NodeType for Node16<K, V> {
    const KIND: NodeKind = NodeKind::Node16;
    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> Node16<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> Self {
        Self::new_from_header(NodeHeader::new::<Self>(key, range))
    }

    pub fn new_from_header(header: NodeHeader<K, V>) -> Self {
        Node16 {
            header,
            keys: [0; 16],
            ptr: [NodePtr::<Unknown, K, V>::dangling(); 16],
        }
    }

    pub fn is_full(&self) -> bool {
        self.header.data().len == 16
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len < 5
    }

    pub fn remove(&mut self, key: u8) -> Option<OwnedNodePtr<K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;

        self.keys.copy_within((idx + 1).min(16).., idx);
        self.header.data_mut().len -= 1;

        unsafe { Some(self.ptr[idx].assume_owned()) }
    }

    /// Copy over from node 16 into an uninitalized node48.
    ///
    /// This function is designed to avoid unnessacery copying
    pub unsafe fn copy_from_node4(
        node: OwnedTypedNodePtr<Node4<K, V>>,
        place: &mut MaybeUninit<Self>,
    ) {
        debug_assert!(node.is_full());

        let node = node.as_unknown();
        let dst_ptr = place.as_mut_ptr();

        // copy over pointers into the array.
        let src = addr_of!((*node.as_ptr()).ptr[0]);
        let dst = addr_of_mut!((*dst_ptr).ptr[0]);

        std::ptr::copy_nonoverlapping(src, dst, 4);

        let src = addr_of!((*node.as_ptr()).keys[0]);
        let dst = addr_of_mut!((*dst_ptr).keys[0]);

        std::ptr::copy_nonoverlapping(src, dst, 4);

        // copy over header.
        let dst = addr_of_mut!((*dst_ptr).header);
        let mut header = node.erase_type().take_header();
        header.change_type::<Self>();
        dst.write(header);

        // everthing copied over, delete node since it is unused.
        TypedNodePtr::dealloc(node);
    }

    pub unsafe fn copy_from_node48(
        node: OwnedTypedNodePtr<Node48<K, V>>,
        place: &mut MaybeUninit<Self>,
    ) {
        debug_assert!(node.should_shrink());

        let node = node.as_unknown();
        let dst_ptr = place.as_mut_ptr();

        let ptr_src = addr_of!((*node.as_ptr()).ptr[0]);
        let idx_src = addr_of!((*node.as_ptr()).idx[0]);
        let ptr_dst = addr_of_mut!((*dst_ptr).ptr[0]);
        let key_dst = addr_of_mut!((*dst_ptr).keys[0]);

        let mut insert_at = 0u8;
        for i in 0..=255u8 {
            let idx = idx_src.add(i as usize).read();
            if idx != u8::MAX {
                key_dst.add(insert_at as usize).write(i);
                let ptr = ptr_src.add(idx as usize).read();
                ptr_dst.add(insert_at as usize).write(ptr.ptr);
                insert_at += 1;
            }
        }

        let dst = addr_of_mut!((*dst_ptr).header);
        let mut header = node.erase_type().take_header();
        header.change_type::<Self>();
        dst.write(header);
    }
}

impl<O: ValidPtr, K: Key + ?Sized, V> TypedNodePtr<O, Node16<K, V>> {
    pub fn get(&self, key: u8) -> Option<NodePtr<O, K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;
        unsafe { Some(self.ptr[idx].assume_ownership::<O>()) }
    }

    pub fn next_node(&self, from: u8) -> Option<(u8, NodePtr<O, K, V>)> {
        let (ptr_idx, next_key) = self
            .keys
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, x)| *x >= from)
            .min_by_key(|(_, x)| *x)?;
        Some((next_key, unsafe {
            self.ptr[ptr_idx].assume_ownership::<O>()
        }))
    }
}

impl<O: MutablePtr, K: Key + ?Sized, V> NodePtr<O, K, V> {
    pub fn insert_grow_16(&mut self, key: u8, v: OwnedNodePtr<K, V>) -> Option<OwnedNodePtr<K, V>> {
        unsafe {
            debug_assert!(self.is::<Node16<K, V>>());

            let mut cast_ptr = self.cast_mut_unchecked::<Node16<K, V>>();

            let len = cast_ptr.header().data().len;

            if let Some(x) = cast_ptr.keys[..len as usize]
                .iter()
                .copied()
                .position(|x| x == key)
            {
                let this = cast_ptr.as_mut();
                let res = std::mem::replace(&mut this.ptr[x], v.into_unknown());
                return unsafe { Some(res.assume_owned()) };
            }

            if !cast_ptr.is_full() {
                cast_ptr.header_mut().data_mut().len += 1;
                cast_ptr.as_mut().ptr[len as usize] = v.into_unknown();
                cast_ptr.as_mut().keys[len as usize] = key;

                return None;
            }

            let this = self
                .as_unknown()
                .cast_unchecked::<Node16<K, V>>()
                .assume_owned();

            let ptr = TypedNodePtr::<Unknown, Node48<K, V>>::alloc();
            Node48::copy_from_node16(this, ptr.as_nonnull().cast().as_mut());
            *self = ptr.erase_type().assume_ownership();

            None
        }
    }

    pub unsafe fn shrink_16(&mut self) {
        let this = self
            .as_unknown()
            .cast_unchecked::<Node16<K, V>>()
            .assume_owned();

        let ptr = TypedNodePtr::<Unknown, Node4<K, V>>::alloc();
        Node4::copy_from_node16(this, ptr.as_nonnull().cast().as_mut());
        *self = ptr.erase_type().assume_ownership();
    }
}
/*
    pub fn insert(&mut self, key: u8, ptr: OwnedNodePtr<K, V>) -> Option<OwnedNodePtr<K, V>> {
        assert!(!self.is_full());
        if let Some(x) = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)
        {
            let res = std::mem::replace(&mut self.ptr[x], ptr.into_unknown());
            return unsafe { Some(res.assume_owned()) };
        }

        let idx = self.header.data().len;
        self.header.data_mut().len += 1;
        self.ptr[idx as usize] = ptr.into_unknown();
        self.keys[idx as usize] = key;
        None
    }

    /// # Safety
    /// Caller must ensure that the given `NodePtr` is a pointer to `Node4`.
    pub unsafe fn insert_grow(
        this: &mut RawBoxedNode<K, V>,
        key: u8,
        v: OwnedNodePtr<K, V>,
    ) -> Option<OwnedNodePtr<K, V>> {
        debug_assert!(this.is::<Self>());

        if let Some(x) = this.as_ref::<Self>().keys
            [..this.as_ref::<Self>().header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)
        {
            let res = std::mem::replace(&mut this.as_mut::<Self>().ptr[x], v.into_unknown());
            return Some(res.assume_owned());
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

    unsafe fn grow(this: &mut OwnedNodePtr<Self>) {
        assert!(this.as_ref().is_full());

        let old_ptr = this.into_unknown();
        let mut new_ptr = TypedNodePtr::<Unknown, Node48<K, V>>::alloc();
        new_ptr.take_header(this.as_unknown());

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

        let mut new_ptr = new_ptr.assume_owned();
        new_ptr.header_mut().data_mut().free = 16;
        new_ptr
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> Node16<K, V> {
    pub fn display(&self, fmt: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        writeln!(
            fmt,
            "NODE16: len={},prefix={:?}",
            self.header.storage.data().len,
            self.header.storage.prefix()
        )?;
        for i in 0..self.header.storage.data().len {
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
*/
