use std::{cmp::Ordering, usize};

use bytemuck::Zeroable;

use crate::{key::KeyBytes, raw::nodes::node48::Node48};

use super::{Node, Node4, NodeBox, NodeHeader, NodeHeaderData, NodeKind, NodeRef};

#[repr(C)]
pub struct Node16<K: KeyBytes + ?Sized, V> {
    pub(crate) header: NodeHeader<K, V>,
    pub(crate) ptr: [Option<NodeBox<K, V>>; 16],
    pub(crate) keys: [u8; 16],
}

unsafe impl<K: KeyBytes + ?Sized, V> Node for Node16<K, V> {
    const KIND: NodeKind = NodeKind::Node16;

    type Key = K;

    type Value = V;
}

impl<K: KeyBytes + ?Sized, V> Node16<K, V> {
    pub fn is_full(&self) -> bool {
        self.header.data().len == 16
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len == 5
    }

    pub fn get(&self, key: u8) -> Option<NodeRef<K, V>> {
        let position = self.keys.iter().copied().position(|x| x == key)?;
        self.ptr[position].as_ref().map(|x| x.as_ref())
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
        NodeBox::new(Node16 { header, ptr, keys })
    }

    pub fn copy_insert(&self, key: u8, value: NodeBox<K, V>) -> NodeBox<K, V> {
        let data = self.header.data();

        let should = match self.find_key(key) {
            Ok(position) => {
                let header = NodeHeader::new_from(&self.header, data);
                let keys = self.keys;
                let mut ptr = <[Option<NodeBox<K, V>>; 16] as Zeroable>::zeroed();

                for i in 0..data.len {
                    if i == position {
                        continue;
                    }
                    ptr[i as usize] = self.ptr[i as usize].clone()
                }

                ptr[position as usize] = Some(value);

                return NodeBox::new(Self { header, keys, ptr });
            }
            Err(e) => e,
        };

        if !self.is_full() {
            let len = data.len;
            let header = NodeHeader::new_from(
                &self.header,
                NodeHeaderData {
                    len: len + 1,
                    ..data
                },
            );

            let mut ptr = <[Option<NodeBox<K, V>>; 16] as Zeroable>::zeroed();
            let mut keys: [u8; 16] = Zeroable::zeroed();

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
            return NodeBox::new(Self { header, ptr, keys });
        }

        let data = self.header.data();
        let header = NodeHeader::new_from(
            &self.header,
            NodeHeaderData::new(data.len + 1, NodeKind::Node48, 0),
        );

        let mut ptr = <[Option<NodeBox<K, V>>; 48] as Zeroable>::zeroed();
        let mut idxs = [u8::MAX; 256];

        for (idx, (k, v)) in self.keys.iter().zip(self.ptr.iter()).enumerate() {
            debug_assert_ne!(*k, key);
            ptr[idx] = v.clone();
            idxs[*k as usize] = idx as u8;
        }
        idxs[key as usize] = 16;
        ptr[16] = Some(value);
        NodeBox::new(Node48 { header, ptr, idxs })
    }

    pub fn copy_remove(&self, key: u8) -> Option<NodeBox<K, V>> {
        let data = self.header.data();
        if !self.keys[..data.len as usize].contains(&key) {
            return None;
        }

        if !self.should_shrink() {
            let header = NodeHeader::new_from(
                &self.header,
                NodeHeaderData {
                    len: data.len - 1,
                    ..data
                },
            );

            let mut ptr = <[Option<NodeBox<K, V>>; 16] as Zeroable>::zeroed();
            let mut keys: [u8; 16] = Zeroable::zeroed();

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

        let header = NodeHeader::new_from(&self.header, NodeHeaderData::new(4, NodeKind::Node4, 0));
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
        Some(NodeBox::new(Node4 { header, ptr, keys }))
    }
}
