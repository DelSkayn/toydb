use super::{
    owned_node::RawOwnedNode, BoxedNode, Node48, NodeHeader, NodeKind, NodeType, OwnedNode,
};
use crate::{
    key::{Key, KeyStorage},
    nodes::node48::PtrUnion,
};
use core::fmt;
use std::{mem::MaybeUninit, ops::Range, ptr::addr_of_mut};

#[repr(C)]
pub struct Node256<K: Key + ?Sized, V> {
    pub header: NodeHeader<K>,
    pub ptr: [Option<BoxedNode<K, V>>; 256],
}

unsafe impl<K: Key + ?Sized, V> NodeType for Node256<K, V> {
    const KIND: super::NodeKind = NodeKind::Node256;
    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> Node256<K, V> {
    pub fn new(key: &K, range: Range<usize>) -> OwnedNode<Self> {
        OwnedNode::new(Node256 {
            header: NodeHeader::new::<Self>(key, range),
            ptr: unsafe { MaybeUninit::zeroed().assume_init() },
        })
    }

    pub fn is_full(&self) -> bool {
        // HACK: slight quirk with len being only u8
        // Can't fit full length so actuall length is self.header.data().len + 1
        self.header.data().len == 255
    }

    pub fn should_shrink(&self) -> bool {
        // HACK: slight quirk with len being only u8
        // Can't fit full length so actuall length is self.header.data().len + 1
        self.header.data().len < 48
    }

    pub fn get(&self, key: u8) -> Option<&BoxedNode<K, V>> {
        self.ptr[key as usize].as_ref()
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut BoxedNode<K, V>> {
        self.ptr[key as usize].as_mut()
    }

    pub fn insert(&mut self, key: u8, ptr: BoxedNode<K, V>) -> Option<BoxedNode<K, V>> {
        let res = self.ptr[key as usize].replace(ptr);
        self.header.data_mut().len += res.is_none() as u8;
        res
    }

    pub fn remove(&mut self, key: u8) -> Option<BoxedNode<K, V>> {
        let res = self.ptr[key as usize].take();
        self.header.data_mut().len -= res.is_some() as u8;
        res
    }

    pub unsafe fn shrink(mut this: RawOwnedNode<Self>) -> RawOwnedNode<Node48<K, V>> {
        assert!(this.as_ref().should_shrink());
        let mut new_node = RawOwnedNode::<Node48<K, V>>::alloc();

        let key_ptr = addr_of_mut!((*new_node.as_ptr()).idx[0]);
        let ptr_ptr = addr_of_mut!((*new_node.as_ptr()).ptr[0]);
        let mut written = 0;

        std::ptr::write_bytes(key_ptr, u8::MAX, 256);

        for (idx, p) in this.as_mut().ptr.iter_mut().enumerate() {
            if let Some(x) = p.take() {
                ptr_ptr.add(written).write(PtrUnion { ptr: x.into_raw() });
                key_ptr.add(idx).write(written as u8);
                written += 1;
            }
        }
        debug_assert_eq!(written, 48);

        new_node.copy_header_from(this);
        // HACK: undo storage quirk.
        new_node.header_mut().data_mut().len += 1;
        new_node.header_mut().data_mut().free = u8::MAX;

        RawOwnedNode::dealloc(this);

        new_node
    }
}

impl<K: Key + ?Sized, V: fmt::Debug> Node256<K, V> {
    pub fn display(&self, fmt: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        writeln!(
            fmt,
            "NODE256: len={},prefix={:?}",
            self.header.storage().data().len,
            self.header.storage().prefix()
        )?;
        for (idx, p) in self.ptr.iter().enumerate() {
            let Some(p) = p else { break };
            for _ in 0..depth {
                fmt.write_str("  ")?;
            }
            write!(fmt, "[{}] = ", idx)?;
            p.display(fmt, depth + 1)?;
        }
        Ok(())
    }
}
