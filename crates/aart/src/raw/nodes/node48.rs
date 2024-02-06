use bytemuck::Zeroable;

use crate::{key::KeyBytes, raw::nodes::node256::Node256};

use super::{node16::Node16, Node, NodeBox, NodeHeader, NodeHeaderData, NodeKind, NodeRef};

#[repr(C)]
pub struct Node48<K: KeyBytes + ?Sized, V> {
    pub(crate) header: NodeHeader<K, V>,
    pub(crate) ptr: [Option<NodeBox<K, V>>; 48],
    pub(crate) idxs: [u8; 256],
}

unsafe impl<K: KeyBytes + ?Sized, V> Node for Node48<K, V> {
    const KIND: NodeKind = NodeKind::Node48;

    type Key = K;

    type Value = V;
}

impl<K: KeyBytes + ?Sized, V> Node48<K, V> {
    pub fn is_full(&self) -> bool {
        self.header.data().len == 48
    }

    pub fn should_shrink(&self) -> bool {
        self.header.data().len == 17
    }

    pub fn get(&self, key: u8) -> Option<NodeRef<K, V>> {
        let idx = self.idxs[key as usize];
        if idx != u8::MAX {
            return self.ptr[idx as usize].as_ref().map(|x| x.as_ref());
        }
        None
    }

    pub fn copy_drop_prefix(&self, until: usize) -> NodeBox<K, V> {
        let header = self.header.copy_drop_prefix(until);
        let ptr = self.ptr.clone();
        let idxs = self.idxs;
        NodeBox::new(Node48 { header, ptr, idxs })
    }

    pub fn copy_insert(&self, key: u8, value: NodeBox<K, V>) -> NodeBox<K, V> {
        let data = self.header.data();

        let idx = self.idxs[key as usize];
        if idx != u8::MAX {
            let header = NodeHeader::new_from(&self.header, data);
            let idxs: [u8; 256] = self.idxs;
            let mut ptr = <[Option<NodeBox<K, V>>; 48] as Zeroable>::zeroed();
            for i in 0..data.len {
                if i == idx {
                    continue;
                }
                ptr[i as usize] = self.ptr[i as usize].clone();
            }
            ptr[idx as usize] = Some(value);

            return NodeBox::new(Self { header, ptr, idxs });
        }

        if !self.is_full() {
            let header = NodeHeader::new_from(
                &self.header,
                NodeHeaderData {
                    len: data.len + 1,
                    ..data
                },
            );

            let mut idxs: [u8; 256] = self.idxs;
            let mut ptr = <[Option<NodeBox<K, V>>; 48] as Zeroable>::zeroed();

            for i in 0..data.len {
                ptr[i as usize] = self.ptr[i as usize].clone();
            }
            ptr[data.len as usize] = Some(value);
            idxs[key as usize] = data.len;

            return NodeBox::new(Self { header, ptr, idxs });
        }

        // HACK: In order to fit the full capacity of node256 into a single byte we subtract 1 from
        // the length when we store it.
        let header =
            NodeHeader::new_from(&self.header, NodeHeaderData::new(48, NodeKind::Node256, 0));
        let mut ptr = <[Option<NodeBox<K, V>>; 256] as Zeroable>::zeroed();

        for (idx, i) in self
            .idxs
            .iter()
            .copied()
            .enumerate()
            .filter(|x| x.1 != u8::MAX)
        {
            ptr[idx] = self.ptr[i as usize].clone();
        }

        debug_assert!(ptr[key as usize].is_none());
        ptr[key as usize] = Some(value);

        NodeBox::new(Node256 { header, ptr })
    }

    pub fn copy_remove(&self, key: u8) -> Option<NodeBox<K, V>> {
        let key_idx = self.idxs[key as usize];
        if key_idx == u8::MAX {
            return None;
        }

        let data = self.header.data();

        if !self.should_shrink() {
            let header = NodeHeader::new_from(
                &self.header,
                NodeHeaderData {
                    len: data.len + 1,
                    ..data
                },
            );

            // TODO: Possibly optimize this to not inialize twice
            let mut idxs = [0u8; 256];
            for (i, idx) in self.idxs.iter().copied().enumerate() {
                let past = (idx != u8::MAX) & (idx > key_idx);
                idxs[i] = idx - past as u8
            }
            idxs[key as usize] = u8::MAX;

            let mut ptr = <[Option<NodeBox<K, V>>; 48] as Zeroable>::zeroed();

            for i in 0..data.len {
                if i == key_idx {
                    continue;
                }
                ptr[i as usize - (i >= key_idx) as usize] = self.ptr[i as usize].clone();
            }

            return Some(NodeBox::new(Self { header, idxs, ptr }));
        }

        let header =
            NodeHeader::new_from(&self.header, NodeHeaderData::new(16, NodeKind::Node16, 0));

        let mut ptr = <[Option<NodeBox<K, V>>; 16] as Zeroable>::zeroed();
        let mut keys: [u8; 16] = Zeroable::zeroed();

        let mut write = 0;
        for i in 0..256 {
            let idx = self.idxs[i];
            if idx == u8::MAX {
                continue;
            }
            keys[write] = i as u8;
            ptr[write] = self.ptr[idx as usize].clone();
            write += 1;
        }

        Some(NodeBox::new(Node16 { header, ptr, keys }))
    }
}
