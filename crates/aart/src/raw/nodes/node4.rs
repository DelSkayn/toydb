use std::cmp::Ordering;

use bytemuck::Zeroable;

use crate::{
    key::{KeyBytes, KeyStorage},
    raw::nodes::node16::Node16,
};

use super::{Node, NodeBox, NodeHeader, NodeHeaderData, NodeKind, NodeRef};

#[repr(C)]
pub struct Node4<K: KeyBytes + ?Sized, V> {
    pub(crate) header: NodeHeader<K, V>,
    pub(crate) ptr: [Option<NodeBox<K, V>>; 4],
    pub(crate) keys: [u8; 4],
}

unsafe impl<K: KeyBytes + ?Sized, V> Node for Node4<K, V> {
    const KIND: NodeKind = NodeKind::Node4;

    type Key = K;

    type Value = V;
}

impl<K: KeyBytes + ?Sized, V> Node4<K, V> {
    pub fn is_full(&self) -> bool {
        self.header.data().len == 4
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len == 2
    }

    pub fn get(&self, key: u8) -> Option<NodeRef<K, V>> {
        let position = self.find_key(key).ok()?;
        self.ptr[position as usize].as_ref().map(|x| x.as_ref())
    }

    fn find_key(&self, key: u8) -> Result<u8, u8> {
        for i in 0..self.header.data().len {
            match self.keys[i as usize].cmp(&key) {
                Ordering::Less => {}
                Ordering::Equal => return Ok(i),
                Ordering::Greater => return Err(i),
            }
        }
        Err(self.header.data().len)
    }

    pub fn copy_drop_prefix(&self, until: usize) -> NodeBox<K, V> {
        let header = self.header.copy_drop_prefix(until);
        let ptr = self.ptr.clone();
        let keys = self.keys;
        NodeBox::new(Node4 { header, ptr, keys })
    }

    pub fn copy_insert(&self, key: u8, value: NodeBox<K, V>) -> NodeBox<K, V> {
        let data = self.header.data();

        let should = match self.find_key(key) {
            Ok(p) => {
                // num exists just replace.
                let mut ptr = <[Option<NodeBox<K, V>>; 4] as Zeroable>::zeroed();
                let keys: [u8; 4] = self.keys;

                for i in 0..data.len {
                    if p == i {
                        continue;
                    }
                    ptr[i as usize] = self.ptr[i as usize].clone();
                }

                ptr[p as usize] = Some(value);

                return NodeBox::new(Self {
                    header: NodeHeader::new_from(&self.header, self.header.data()),
                    ptr,
                    keys,
                });
            }
            Err(s) => s,
        };

        if !self.is_full() {
            let len = self.header.data().len;
            let mut ptr = <[Option<NodeBox<K, V>>; 4] as Zeroable>::zeroed();
            let mut keys: [u8; 4] = self.keys;

            for (idx, (k, v)) in self.keys[..len as usize]
                .iter()
                .zip(self.ptr.iter())
                .enumerate()
            {
                let at = idx >= should as usize;
                ptr[idx + at as usize] = v.clone();
                keys[idx + at as usize] = *k;
            }

            keys[should as usize] = key;
            ptr[should as usize] = Some(value);
            let header = NodeHeader::new_from(
                &self.header,
                NodeHeaderData {
                    len: data.len + 1,
                    ..data
                },
            );

            return NodeBox::new(Self { header, ptr, keys });
        }

        // node is full so grow to node 16.
        let mut ptr = <[Option<NodeBox<K, V>>; 16]>::zeroed();
        let mut keys: [u8; 16] = Zeroable::zeroed();

        for (idx, (k, v)) in self.keys.iter().zip(self.ptr.iter()).enumerate() {
            let at = idx >= should as usize;
            ptr[idx + at as usize] = v.clone();
            keys[idx + at as usize] = *k;
        }

        keys[should as usize] = key;
        ptr[should as usize] = Some(value);
        let header =
            NodeHeader::new_from(&self.header, NodeHeaderData::new(5, NodeKind::Node16, 0));
        NodeBox::new(Node16 { header, ptr, keys })
    }

    pub fn copy_remove(&self, key: u8) -> Option<NodeBox<K, V>> {
        if !self.keys.contains(&key) {
            return None;
        }

        let data = self.header.data();

        if !self.should_shrink() {
            let header = NodeHeader::new_from(
                &self.header,
                NodeHeaderData {
                    len: data.len - 1,
                    ..data
                },
            );

            let mut ptr = <[Option<NodeBox<K, V>>; 4] as Zeroable>::zeroed();
            let mut keys: [u8; 4] = Zeroable::zeroed();

            for (idx, (k, v)) in self
                .keys
                .iter()
                .zip(self.ptr.iter())
                .filter(|x| *x.0 != key)
                .enumerate()
            {
                ptr[idx] = v.clone();
                keys[idx] = *k;
            }
            return Some(NodeBox::new(Self { header, ptr, keys }));
        }

        todo!()
        // Node has only one node left after removing, fold into a single node.
    }

    pub fn new_split(
        key: &K,
        until: usize,
        first: (u8, NodeBox<K, V>),
        second: (u8, NodeBox<K, V>),
    ) -> Self {
        let header = NodeHeader::new(key, until, NodeHeaderData::new(2, NodeKind::Node4, 0));

        let (first, second) = if first.0 < second.0 {
            (first, second)
        } else {
            (second, first)
        };

        Node4 {
            header,
            ptr: [Some(first.1), Some(second.1), None, None],
            keys: [first.0, second.0, 0, 0],
        }
    }
}
