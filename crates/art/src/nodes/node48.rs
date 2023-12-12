use super::{
    node256::Node256, owned_node::RawOwnedNode, BoxedNode, NodeHeader, NodeKind, NodeType,
    OwnedNode, RawBoxedNode,
};
use crate::{
    key::{Key, KeyStorage},
    nodes::Node16,
};
use std::{fmt, mem::MaybeUninit, ops::Range, ptr::addr_of_mut};

pub union PtrUnion<K: Key + ?Sized, V> {
    pub free: u8,
    pub ptr: RawBoxedNode<K, V>,
}

/// A node with a maximum of 48 branches.
///
/// Lookup is done by looking into the idx array, if the idx array is u8::MAX the node contains no
#[repr(C)]
pub struct Node48<K: Key + ?Sized, V> {
    pub header: NodeHeader<K>,
    pub ptr: [PtrUnion<K, V>; 48],
    pub idx: [u8; 256],
}

unsafe impl<K: Key + ?Sized, V> NodeType for Node48<K, V> {
    const KIND: super::NodeKind = NodeKind::Node48;
    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> Node48<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> OwnedNode<Self> {
        OwnedNode::new(Node48 {
            header: NodeHeader::new::<Self>(key, range),
            ptr: unsafe { MaybeUninit::zeroed().assume_init() },
            idx: [u8::MAX; 256],
        })
    }

    pub fn is_full(&self) -> bool {
        self.header.data().len == 48
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len < 16
    }

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
            self.header.storage().data().len,
            self.header.storage().prefix()
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
