use super::{Node16, NodeHeader, NodeKind, NodeType};
use crate::{
    key::Key,
    raw::{
        ptr::{NodePtr, OwnedTypedNodePtr, TypedNodePtr, Unknown},
        MutablePtr, OwnedNodePtr,
    },
};
use std::{
    mem::MaybeUninit,
    ops::Range,
    ptr::{addr_of, addr_of_mut},
};

#[repr(C)]
pub struct Node4<K: Key + ?Sized, V> {
    pub header: NodeHeader<K, V>,
    pub ptr: [NodePtr<Unknown, K, V>; 4],
    pub keys: [u8; 4],
}

unsafe impl<K: Key + ?Sized, V> NodeType for Node4<K, V> {
    const KIND: NodeKind = NodeKind::Node4;
    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> Node4<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> Self {
        Node4 {
            header: NodeHeader::new::<Self>(key, range),
            keys: [0; 4],
            ptr: [NodePtr::dangling(); 4],
        }
    }

    pub fn is_full(&self) -> bool {
        self.header.data().len == 4
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len == 1
    }

    pub fn remove(&mut self, key: u8) -> Option<OwnedNodePtr<K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;

        self.keys.copy_within((idx + 1).min(4).., idx);
        self.header.data_mut().len -= 1;

        unsafe { Some(self.ptr[idx].assume_owned()) }
    }

    pub unsafe fn copy_from_node16(
        node: OwnedTypedNodePtr<Node16<K, V>>,
        place: &mut MaybeUninit<Self>,
    ) {
        debug_assert!(node.should_shrink());

        let node = node.as_unknown();
        let dst_ptr = place.as_mut_ptr();

        // copy over pointers into the array.
        let src = addr_of!((*node.as_ptr()).ptr[0]);
        let dst = addr_of_mut!((*dst_ptr).ptr[0]);
        std::ptr::copy_nonoverlapping(src, dst, 4);

        // copy over pointers into the array.
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
}

impl<O: MutablePtr, K: Key + ?Sized, V> NodePtr<O, K, V> {
    pub fn insert_grow_4(&mut self, key: u8, v: OwnedNodePtr<K, V>) -> Option<OwnedNodePtr<K, V>> {
        unsafe {
            debug_assert!(self.is::<Node4<K, V>>());

            let mut cast_ptr = self.cast_mut_unchecked::<Node4<K, V>>();

            let len = cast_ptr.header().data().len;

            if let Some(x) = cast_ptr.keys[..len as usize]
                .iter()
                .copied()
                .position(|x| x == key)
            {
                let this = cast_ptr.as_mut();
                let res = std::mem::replace(&mut this.ptr[x], v.into_unknown());
                return Some(res.assume_owned());
            }

            if !cast_ptr.is_full() {
                cast_ptr.header_mut().data_mut().len += 1;
                cast_ptr.as_mut().ptr[len as usize] = v.into_unknown();
                cast_ptr.as_mut().keys[len as usize] = key;

                return None;
            }

            let this = self
                .as_unknown()
                .cast_unchecked::<Node4<K, V>>()
                .assume_owned();

            let ptr = TypedNodePtr::<Unknown, Node4<K, V>>::alloc();
            Node16::copy_from_node4(this, ptr.as_nonnull().cast().as_mut());
            *self = ptr.erase_type().assume_ownership();

            None
        }
    }
}

/*

    pub fn get(&self, key: u8) -> Option<NodePtr<Borrow, K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;
        unsafe { Some(self.ptr[idx].assume_borrow()) }
    }

    pub fn get_mut(&mut self, key: u8) -> Option<NodePtr<BorrowMut, K, V>> {
        let idx = self.keys[..self.header.data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)?;
        unsafe { Some(self.ptr[idx].assume_borrow_mut()) }
    }

    pub fn next_node(&self, from: u8) -> Option<(u8, NodePtr<Unknown, K, V>)> {
        let (ptr_idx, next_key) = self
            .keys
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, x)| *x >= from)
            .min_by_key(|(_, x)| *x)?;
        Some((next_key, unsafe { self.ptr[ptr_idx] }))
    }

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

    pub unsafe fn insert_grow(
        this: &mut OwnedNodePtr<K, V>,
        key: u8,
        v: OwnedNodePtr<K, V>,
    ) -> Option<OwnedNodePtr<K, V>> {
        debug_assert!(this.is::<Self>());

        if let Some(x) = this.cast_ref::<Self>().keys[..this.header().data().len as usize]
            .iter()
            .copied()
            .position(|x| x == key)
        {
            let res = std::mem::replace(
                &mut this.as_unknown().cast_mut_unchecked::<Self>().ptr[x],
                v.into_unknown(),
            );
            return Some(res.assume_owned());
        }

        if this.cast_ref::<Self>().is_full() {
            Self::grow(this)
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

    pub unsafe fn fold(mut this: RawOwnedNode<Self>) -> RawBoxedNode<K, V> {
        debug_assert_eq!(this.as_ref().header.data().len, 1);
        // move out the child
        let mut child = this.as_mut().ptr[0].assume_init();

        // append the current prefix and the key of the child to the prefix of the child.
        child
            .header_mut()
            .storage
            .prepend_prefix(this.as_ref().header.storage.prefix(), this.as_ref().keys[0]);

        child.header_mut().parent = this.header().parent;

        this.drop_in_place();

        child
    }

    unsafe fn grow(this: &mut NodePtr<Owned,K,V>) {
        let old = this.cast_ref_unchecked::<Self>().as_unknown();
        debug_assert!(old.is_full());

        let new_node = Node16::new(key, range)

        let new_ptr = TypedNodePtr::<Unknown,Node16<K, V>>::alloc();
        new_ptr.take_header(old);

        let src_ptr = new_ptr.as_ptr().cast::<Self>();
        std::ptr::copy(
            addr_of_mut!((*src_ptr).keys).cast::<u8>(),
            addr_of_mut!((*new_ptr.as_ptr()).keys).cast::<u8>(),
            4,
        );
        new_ptr
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> Node4<K, V> {
    pub fn display(&self, fmt: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        writeln!(
            fmt,
            "NODE4: len={},prefix={:?}",
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

impl<K: Key + ?Sized, V> Drop for Node4<K, V> {
    fn drop(&mut self) {
        for i in 0..self.header.data().len {
            unsafe { self.ptr[i as usize].assume_init().drop_in_place() }
        }
    }
}
*/
