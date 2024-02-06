use std::{
    mem::MaybeUninit,
    ptr::{addr_of, addr_of_mut},
};

use crate::{
    key::Key,
    raw::{
        ptr::{OwnedNodePtr, OwnedTypedNodePtr, TypedNodePtr},
        MutablePtr, NodePtr, Unknown,
    },
};

use super::{Node48, NodeHeader, NodeKind, NodeType};

#[repr(C)]
pub struct Node256<K: Key + ?Sized, V> {
    pub header: NodeHeader<K, V>,
    pub ptr: [Option<OwnedNodePtr<K, V>>; 256],
}

unsafe impl<K: Key + ?Sized, V> NodeType for Node256<K, V> {
    const KIND: NodeKind = NodeKind::Node256;
    type Key = K;
    type Value = V;
}

impl<K: Key + ?Sized, V> Node256<K, V> {
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

    pub fn insert(&mut self, key: u8, v: OwnedNodePtr<K, V>) -> Option<OwnedNodePtr<K, V>> {
        let res = self.ptr[key as usize].replace(v);
        self.header.data_mut().len += res.is_none() as u8;
        res
    }

    pub fn remove(&mut self, key: u8) -> Option<OwnedNodePtr<K, V>> {
        let res = self.ptr[key as usize].take();
        self.header.data_mut().len -= res.is_some() as u8;
        res
    }

    pub unsafe fn copy_from_node48(
        node: OwnedTypedNodePtr<Node48<K, V>>,
        place: &mut MaybeUninit<Self>,
    ) {
        debug_assert!(node.is_full());

        let node = node.as_unknown();
        let dst_ptr = place.as_mut_ptr();

        // copy over pointers into the array.
        let src_ptr = addr_of!((*node.as_ptr()).ptr[0]);
        let src_idx = addr_of!((*node.as_ptr()).idx[0]);
        let dst = addr_of_mut!((*dst_ptr).ptr[0]);

        std::ptr::write_bytes(dst, 0, 256);

        for i in 0..256 {
            let idx = src_idx.add(i).read();
            debug_assert_ne!(idx, u8::MAX);

            let ptr = src_ptr.add(i).read();
            dst.add(idx as usize).write(Some(ptr.ptr.assume_owned()));
        }

        // copy over header.
        let dst = addr_of_mut!((*dst_ptr).header);
        let mut header = node.erase_type().take_header();
        header.change_type::<Self>();
        header.data_mut().len -= 1;
        dst.write(header);

        // everthing copied over, delete node since it is unused.
        TypedNodePtr::dealloc(node);
    }
}

impl<O: MutablePtr, K: Key + ?Sized, V> NodePtr<O, K, V> {
    pub unsafe fn shrink_256(&mut self) {
        let this = self
            .as_unknown()
            .cast_unchecked::<Node256<K, V>>()
            .assume_owned();

        let ptr = TypedNodePtr::<Unknown, Node48<K, V>>::alloc();
        Node48::copy_from_node256(this, ptr.as_nonnull().cast().as_mut());
        *self = ptr.erase_type().assume_ownership();
    }
}

/*
    pub fn get(&self, key: u8) -> Option<&BoxedNode<K, V>> {
        self.ptr[key as usize].as_ref()
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut BoxedNode<K, V>> {
        self.ptr[key as usize].as_mut()
    }

    pub fn next_node(&mut self, from: u8) -> Option<(u8, RawBoxedNode<K, V>)> {
        self.ptr[from as usize..]
            .iter()
            .enumerate()
            .find_map(|(idx, x)| x.as_ref().map(|x| (idx as u8, x.as_raw())))
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
            self.header.storage.data().len,
            self.header.storage.prefix()
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
*/
