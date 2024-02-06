use std::usize;

use bytemuck::Zeroable;

use super::{ptr::NodeBox, Node, Node48, NodeHeader, NodeHeaderData, NodeKind, NodeRef};
use crate::key::KeyBytes;

#[repr(C)]
pub struct Node256<K: KeyBytes + ?Sized, V> {
    pub(crate) header: NodeHeader<K, V>,
    pub(crate) ptr: [Option<NodeBox<K, V>>; 256],
}

unsafe impl<K: KeyBytes + ?Sized, V> Node for Node256<K, V> {
    const KIND: NodeKind = NodeKind::Node256;

    type Key = K;

    type Value = V;
}

impl<K: KeyBytes + ?Sized, V> Node256<K, V> {
    pub fn should_shrink(&self) -> bool {
        self.header.data().len == 48
    }

    pub fn get(&self, key: u8) -> Option<NodeRef<K, V>> {
        self.ptr[key as usize].as_ref().map(|x| x.as_ref())
    }

    pub fn copy_drop_prefix(&self, until: usize) -> NodeBox<K, V> {
        let header = self.header.copy_drop_prefix(until);
        let ptr = self.ptr.clone();
        NodeBox::new(Node256 { header, ptr })
    }

    pub fn copy_insert(&self, key: u8, value: NodeBox<K, V>) -> NodeBox<K, V> {
        let data = self.header.data();

        let mut ptr = self.ptr.clone();
        let added = self.ptr[key as usize].is_none();
        ptr[key as usize] = Some(value);

        let header = NodeHeader::new_from(
            &self.header,
            NodeHeaderData {
                len: data.len + added as u8,
                ..data
            },
        );

        NodeBox::new(Self { ptr, header })
    }

    pub fn copy_remove(&self, key: u8) -> Option<NodeBox<K, V>> {
        self.ptr[key as usize].as_ref()?;

        let data = self.header.data();

        if !self.should_shrink() {
            let mut ptr = <[Option<NodeBox<K, V>>; 256] as Zeroable>::zeroed();
            for i in 0..=255 {
                if i == key {
                    continue;
                }
                ptr[i as usize] = self.ptr[i as usize].clone();
            }

            let header = NodeHeader::new_from(
                &self.header,
                NodeHeaderData {
                    len: data.len - 1,
                    ..data
                },
            );

            return Some(NodeBox::new(Self { ptr, header }));
        }

        let mut idxs = [u8::MAX; 256];
        let mut ptr = <[Option<NodeBox<K, V>>; 48] as Zeroable>::zeroed();

        let mut insert_at = 0;
        for i in 0..=255 {
            if i == key {
                continue;
            }
            if let Some(p) = self.ptr[i as usize].as_ref() {
                ptr[insert_at] = Some(p.clone());
                idxs[i as usize] = insert_at as u8;
                insert_at += 1;
            }
        }

        let header =
            NodeHeader::new_from(&self.header, NodeHeaderData::new(48, NodeKind::Node48, 0));

        Some(NodeBox::new(Node48 { header, idxs, ptr }))
    }
}
