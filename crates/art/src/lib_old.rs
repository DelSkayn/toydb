use std::{alloc::Layout, ptr::NonNull};

mod header;
mod key;
mod nodes;

use nodes::{LeafNode, NodeMut, NodeRef};

use self::header::NodeHeader;
use self::nodes::{Node16, Node256, Node4, Node48, NodePtr};

pub struct Art<V> {
    root: Option<NodePtr<V>>,
}

impl<V> Default for Art<V> {
    fn default() -> Self {
        Art::new()
    }
}

impl<V> Art<V> {
    pub fn new() -> Self {
        dbg!(std::mem::size_of::<NodeHeader>());
        dbg!(std::mem::size_of::<Node4<V>>());
        dbg!(std::mem::size_of::<Node16<V>>());
        dbg!(std::mem::size_of::<Node48<V>>());
        dbg!(std::mem::size_of::<Node256<V>>());

        assert_eq!(
            std::mem::align_of::<Node4<V>>(),
            std::mem::align_of::<Node16<V>>()
        );
        assert_eq!(
            std::mem::align_of::<Node4<V>>(),
            std::mem::align_of::<Node48<V>>()
        );
        assert_eq!(
            std::mem::align_of::<Node4<V>>(),
            std::mem::align_of::<Node256<V>>()
        );
        Art { root: None }
    }

    unsafe fn alloc_node<N>(node: N) -> NonNull<N> {
        let ptr = std::alloc::alloc(Layout::new::<N>()).cast::<N>();
        let ptr = NonNull::new(ptr).unwrap();
        ptr.as_ptr().write(node);
        ptr
    }

    fn match_prefix(key: &[u8], prefix: &[u8]) -> Option<usize> {
        for (i, prefix) in prefix.iter().enumerate() {
            let k = key.get(i).copied().unwrap();
            if k != *prefix {
                return Some(i);
            }
        }
        None
    }

    pub fn insert(&mut self, key: &[u8], value: V) -> Option<V> {
        if let Some(root) = self.root.as_mut() {
            return Self::insert_node(root, key, value);
        }
        let ptr = unsafe {
            let ptr = Self::alloc_node(LeafNode::new(key, value));
            NodePtr::from_node_ptr(ptr.cast())
        };
        self.root = Some(ptr);
        None
    }

    pub fn get<'a>(&'a self, key: &[u8]) -> Option<&'a V> {
        let Some(root) = self.root.as_ref() else {
            return None;
        };
        Self::get_node(root, key)
    }

    fn get_node<'a>(node: &'a NodePtr<V>, key: &[u8]) -> Option<&'a V> {
        let header = node.header();
        if Self::match_prefix(key, header.prefix()).is_some() {
            return None;
        };

        if key.len() == header.prefix().len() {
            let NodeRef::Leaf(node) = node.as_ref() else {
                return None;
            };
            return Some(&node.value);
        }

        let remaining_key = &key[header.prefix().len()..];
        let Some(node) = node.lookup(remaining_key[0]) else {
            return None;
        };
        Self::get_node(node, &remaining_key[1..])
    }

    fn insert_node(node: &mut NodePtr<V>, key: &[u8], value: V) -> Option<V> {
        unsafe {
            let header = node.header();
            if let Some(x) = Self::match_prefix(key, header.prefix()) {
                // diverging prefix, split node.

                let existing_key = header.prefix()[x];
                let new_key = key[x];
                let mut new_node = Self::alloc_node(Node4::<V>::new(&key[..x]));
                let key_node = Self::alloc_node(LeafNode::new(&key[x + 1..], value));
                node.header_mut().split_prefix(x + 1);
                let key_node = NodePtr::from_node_ptr(key_node.cast());

                new_node.as_mut().insert_at(existing_key, *node);
                new_node.as_mut().insert_at(new_key, key_node);

                *node = NodePtr::from_node_ptr(new_node.cast());
                return None;
            }

            let remaining_key = &key[header.prefix().len()..];
            if remaining_key.is_empty() {
                let NodeMut::Leaf(node) = node.as_mut() else {
                    panic!()
                };
                return Some(std::mem::replace(&mut node.value, value));
            }

            if let Some(node) = node.lookup_mut(key[0]) {
                return Self::insert_node(node, &remaining_key[1..], value);
            }

            let new_node = Self::alloc_node(LeafNode::new(&remaining_key[1..], value));
            node.insert_at(remaining_key[0], NodePtr::from_node_ptr(new_node.cast()));
            None
        }
    }
}
