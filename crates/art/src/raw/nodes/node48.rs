use crate::{
    key::Key,
    raw::{
        ptr::{NodePtr, OwnedTypedNodePtr, TypedNodePtr, Unknown},
        MutablePtr, OwnedNodePtr,
    },
};
use std::{
    mem::MaybeUninit,
    ptr::{addr_of, addr_of_mut},
};

use super::{Node16, Node256, NodeHeader, NodeKind, NodeType};

pub union PtrUnion<K: Key + ?Sized, V> {
    pub free: u8,
    pub ptr: NodePtr<Unknown, K, V>,
}

/// A node with a maximum of 48 branches.
///
/// Lookup is done by looking into the idx array, if the idx array is u8::MAX the node contains no
#[repr(C)]
pub struct Node48<K: Key + ?Sized, V> {
    pub header: NodeHeader<K, V>,
    pub ptr: [PtrUnion<K, V>; 48],
    pub idx: [u8; 256],
}

unsafe impl<K: Key + ?Sized, V> NodeType for Node48<K, V> {
    const KIND: NodeKind = NodeKind::Node48;
    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> Node48<K, V> {
    pub fn is_full(&self) -> bool {
        self.header.data().len == 48
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len < 16
    }

    pub fn remove(&mut self, key: u8) -> Option<OwnedNodePtr<K, V>> {
        let idx = self.idx[key as usize];
        if idx == u8::MAX {
            return None;
        }

        let free = std::mem::replace(&mut self.header.data_mut().free, idx);
        let res = std::mem::replace(&mut self.ptr[idx as usize], PtrUnion { free });
        self.header.data_mut().len -= 1;

        unsafe { Some(res.ptr.assume_owned()) }
    }

    /// Copy over from node 16 into an uninitalized node48.
    ///
    /// This function is designed to avoid unnessacery copying
    pub unsafe fn copy_from_node16(
        node: OwnedTypedNodePtr<Node16<K, V>>,
        place: &mut MaybeUninit<Self>,
    ) {
        debug_assert!(node.is_full());

        let node = node.as_unknown();
        let dst_ptr = place.as_mut_ptr();

        // copy over pointers into the array.
        let src = addr_of!((*node.as_ptr()).ptr).cast::<PtrUnion<K, V>>();
        let dst = addr_of_mut!((*dst_ptr).ptr).cast::<PtrUnion<K, V>>();

        std::ptr::copy_nonoverlapping(src, dst, 16);

        for i in 16u8..47 {
            dst.add(i as usize).write(PtrUnion { free: i })
        }
        dst.add(47).write(PtrUnion { free: u8::MAX });

        // write in the keys
        let src = addr_of!((*node.as_ptr()).keys).cast::<u8>();
        let dst = addr_of_mut!((*dst_ptr).idx).cast::<u8>();

        // intialize keys to u8 max
        std::ptr::write_bytes(dst, u8::MAX, 256);

        // write over keys.
        for i in 0..16u8 {
            let key = src.add(i as usize).read();
            dst.add(key as usize).write(i);
        }

        // copy over header.
        let dst = addr_of_mut!((*dst_ptr).header);
        let mut header = node.erase_type().take_header();
        header.data_mut().kind = NodeKind::Node48;
        dst.write(header);

        // everthing copied over, delete node since it is unused.
        TypedNodePtr::dealloc(node);
    }

    pub unsafe fn copy_from_node256(
        node: OwnedTypedNodePtr<Node256<K, V>>,
        place: &mut MaybeUninit<Self>,
    ) {
        debug_assert!(node.should_shrink());

        let node = node.as_unknown();
        let dst_ptr = place.as_mut_ptr();

        let ptr_src = addr_of!((*node.as_ptr()).ptr[0]);
        let ptr_dst = addr_of_mut!((*dst_ptr).ptr[0]);
        let key_dst = addr_of_mut!((*dst_ptr).idx[0]);

        // intialize keys to u8 max
        std::ptr::write_bytes(key_dst, u8::MAX, 256);

        let mut insert_at = 0u8;
        for i in 0..255u8 {
            if let Some(ptr) = ptr_src.add(i as usize).read() {
                ptr_dst.add(insert_at as usize).write(PtrUnion {
                    ptr: ptr.into_unknown(),
                });
                key_dst.add(i as usize).write(insert_at);
                insert_at += 1;
            }
        }

        let dst = addr_of_mut!((*dst_ptr).header);
        let mut header = node.erase_type().take_header();
        header.change_type::<Self>();
        header.data_mut().len += 1;
        dst.write(header);
    }
}

impl<O: MutablePtr, K: Key + ?Sized, V> NodePtr<O, K, V> {
    pub fn insert_grow_48(&mut self, key: u8, v: OwnedNodePtr<K, V>) -> Option<OwnedNodePtr<K, V>> {
        debug_assert!(self.is::<Node48<K, V>>());
        unsafe {
            let mut cast_ptr = self.cast_mut_unchecked::<Node48<K, V>>();

            let len = cast_ptr.header().data().len;

            let idx = cast_ptr.idx[key as usize];
            if idx != u8::MAX {
                let this = cast_ptr.as_mut();
                let res = std::mem::replace(&mut this.ptr[idx as usize].ptr, v.into_unknown());
                return unsafe { Some(res.assume_owned()) };
            }

            if !cast_ptr.is_full() {
                cast_ptr.header_mut().data_mut().len += 1;
                cast_ptr.as_mut().idx[key as usize] = len;
                cast_ptr.as_mut().ptr[len as usize] = PtrUnion {
                    ptr: v.into_unknown(),
                };

                return None;
            }

            let this = self
                .as_unknown()
                .cast_unchecked::<Node48<K, V>>()
                .assume_owned();

            let ptr = TypedNodePtr::<Unknown, Node48<K, V>>::alloc();
            Node256::copy_from_node48(this, ptr.as_nonnull().cast().as_mut());
            *self = ptr.erase_type().assume_ownership();

            self.cast_mut_unchecked::<Node256<K, V>>()
                .as_mut()
                .insert(key, v);

            None
        }
    }

    pub unsafe fn shrink_48(&mut self) {
        let this = self
            .as_unknown()
            .cast_unchecked::<Node48<K, V>>()
            .assume_owned();

        let ptr = TypedNodePtr::<Unknown, Node16<K, V>>::alloc();
        Node16::copy_from_node48(this, ptr.as_nonnull().cast().as_mut());
        *self = ptr.erase_type().assume_ownership();
    }
}
/*
    pub fn get(&self, key: u8) -> Option<&BoxedNode<K, V>> {
        let idx = self.idx[key as usize];
        if idx != u8::MAX {
            unsafe { Some(BoxedNode::from_raw_ref(&self.ptr[idx as usize].ptr)) }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut BoxedNode<K, V>> {
        let idx = self.idx[key as usize];
        if idx != u8::MAX {
            unsafe { Some(BoxedNode::from_raw_mut(&mut self.ptr[idx as usize].ptr)) }
        } else {
            None
        }
    }

    pub fn next_node(&mut self, from: u8) -> Option<(u8, RawBoxedNode<K, V>)> {
        let (key, idx) = self.idx[from as usize..]
            .iter()
            .copied()
            .enumerate()
            .find(|(_, x)| *x != u8::MAX)?;
        Some((key as u8, unsafe { self.ptr[idx as usize].ptr }))
    }

    pub fn insert(&mut self, key: u8, ptr: BoxedNode<K, V>) -> Option<BoxedNode<K, V>> {
        assert!(!self.is_full());
        let idx = self.idx[key as usize];
        if idx == u8::MAX {
            let next_free = self.header.data().free;
            self.header.data_mut().free = unsafe { self.ptr[next_free as usize].free };
            self.header.data_mut().len += 1;

            self.idx[key as usize] = next_free;
            self.ptr[next_free as usize].ptr = ptr.into_raw();

            return None;
        }
        unsafe {
            let res = std::mem::replace(&mut self.ptr[idx as usize].ptr, ptr.into_raw());
            Some(BoxedNode::from_raw(res))
        }
    }

    pub unsafe fn insert_grow(
        this: &mut RawBoxedNode<K, V>,
        key: u8,
        v: BoxedNode<K, V>,
    ) -> Option<BoxedNode<K, V>> {
        debug_assert!(this.is::<Self>());

        let idx = this.as_ref::<Self>().idx[key as usize];
        if idx != u8::MAX {
            let res = std::mem::replace(
                &mut this.as_mut::<Self>().ptr[idx as usize].ptr,
                v.into_raw(),
            );
            return Some(BoxedNode::from_raw(res));
        }

        if this.as_ref::<Self>().is_full() {
            let mut ptr = Self::grow(this.into_owned());
            ptr.as_mut().insert(key, v);
            *this = ptr.into_boxed();
            return None;
        }

        let free = this.as_ref::<Self>().header.data().free;
        this.as_mut::<Self>().header.data_mut().free =
            unsafe { this.as_ref::<Self>().ptr[free as usize].free };
        this.as_mut::<Self>().header.data_mut().len += 1;
        this.as_mut::<Self>().idx[key as usize] = free;
        this.as_mut::<Self>().ptr[free as usize].ptr = v.into_raw();

        None
    }

    pub fn remove(&mut self, key: u8) -> Option<BoxedNode<K, V>> {
        if self.idx[key as usize] == u8::MAX {
            return None;
        }
        let idx = self.idx[key as usize];
        self.idx[key as usize] = u8::MAX;
        self.header.data_mut().len -= 1;
        unsafe { Some(BoxedNode::from_raw(self.ptr[idx as usize].ptr)) }
    }

    pub unsafe fn shrink(this: RawOwnedNode<Self>) -> RawOwnedNode<Node16<K, V>> {
        assert!(this.as_ref().should_shrink());
        let mut new_node = RawOwnedNode::<Node16<K, V>>::alloc();

        this.as_ref()
            .idx
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(idx, x)| (x != u8::MAX).then_some(idx))
            .enumerate()
            .for_each(|(idx, at)| {
                addr_of_mut!((*new_node.as_ptr()).keys[0])
                    .add(idx)
                    .write(at as u8);
                addr_of_mut!((*new_node.as_ptr()).ptr[0])
                    .add(idx)
                    .write(MaybeUninit::new(
                        this.as_ref().ptr[this.as_ref().idx[at] as usize].ptr,
                    ))
            });

        new_node.copy_header_from(this);

        RawOwnedNode::dealloc(this);

        new_node
    }

    unsafe fn grow(mut this: RawOwnedNode<Self>) -> RawOwnedNode<Node256<K, V>> {
        let mut new_ptr = RawOwnedNode::<Node256<K, V>>::alloc();

        let ptr_ptr = addr_of_mut!((*new_ptr.as_ptr()).ptr).cast::<Option<BoxedNode<K, V>>>();
        // init zero
        std::ptr::write_bytes(ptr_ptr, 0, 256);
        // copy over pointer
        for i in 0..=255u8 {
            let idx = this.as_ref().idx[i as usize];
            if idx != u8::MAX {
                ptr_ptr.add(i as usize).write(Some(BoxedNode::from_raw(
                    this.as_mut().ptr[idx as usize].ptr,
                )));
            }
        }

        new_ptr.copy_header_from(this);

        RawOwnedNode::dealloc(this);
        // HACK: in order to be able to fit the max size of node256 into a u8 we make the len one
        // smaller. So node256 will be full when its len is 255.
        new_ptr.as_mut().header.data_mut().len = 47;
        new_ptr.as_mut().header.data_mut().kind = NodeKind::Node256;
        new_ptr
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> Node48<K, V> {
    pub fn display(&self, fmt: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        writeln!(
            fmt,
            "NODE48: len={},prefix={:?}",
            self.header.storage.data().len,
            self.header.storage.prefix()
        )?;
        for i in 0..255 {
            if self.idx[i] == u8::MAX {
                continue;
            }
            for _ in 0..depth {
                fmt.write_str("  ")?;
            }
            write!(fmt, "[{}] = ", i)?;
            unsafe {
                self.ptr[self.idx[i] as usize].ptr.display(fmt, depth + 1)?;
            }
        }
        Ok(())
    }
}

impl<K: Key + ?Sized, V> Drop for Node48<K, V> {
    fn drop(&mut self) {
        self.idx
            .into_iter()
            .filter(|x| *x < 48)
            .for_each(|x| unsafe { self.ptr[x as usize].ptr.drop_in_place() })
    }
}
*/
