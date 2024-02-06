use crate::{key::KeyBytes, raw::nodes::Node4};

pub mod nodes;
use nodes::{NodeBox, NodeLeaf};
pub mod root;

#[cfg(test)]
mod test;

use self::nodes::{NodeHeader, NodeHeaderData, NodeRef};

pub struct RawAart<K: KeyBytes + ?Sized, V> {
    root: Option<NodeBox<K, V>>,
}

unsafe impl<K: KeyBytes + ?Sized, V> Send for RawAart<K, V> {}
unsafe impl<K: KeyBytes + ?Sized, V> Sync for RawAart<K, V> {}

impl<K: KeyBytes + ?Sized, V> RawAart<K, V> {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn get(&self, b: &K) -> Option<&NodeLeaf<K, V>> {
        let root = self.root.as_ref()?;

        let common_prefix_len = b.common_prefix_length(root.prefix()).unwrap();
        if common_prefix_len == b.len() {
            return Some(root.as_ref().cast::<NodeLeaf<K, V>>().unwrap());
        }

        unsafe { get_node(root.as_ref(), b) }
    }

    pub fn insert(&mut self, b: &K, value: V) {
        let Some(root) = self.root.as_ref() else {
            self.root = Some(NodeBox::new(NodeLeaf::new(b, b.len(), value)));
            return;
        };

        let leaf = NodeBox::new(NodeLeaf::new(b, 0, value));
        self.root = Some(unsafe { insert_node(root.as_ref(), b, leaf) });
    }
}

unsafe fn get_node<'a, K, V>(node: NodeRef<'a, K, V>, k: &K) -> Option<&'a NodeLeaf<K, V>>
where
    K: KeyBytes + ?Sized,
{
    let common_len = k.common_prefix_length(node.prefix()).unwrap();
    if common_len == k.len() {
        // exact match, return the leaf node.
        assert_eq!(k.len(), node.prefix().len());
        assert!(node.is::<NodeLeaf<_, _>>());
        return Some(node.cast_unchecked());
    }

    if common_len != node.prefix().len() {
        // diverges in prefix, node not in tree
        return None;
    }

    let branch_key = k.at(common_len).unwrap();
    let next = node.get(branch_key)?;
    get_node(next, k.drop_prefix(common_len + 1))
}

unsafe fn insert_node<K, V>(root: NodeRef<K, V>, b: &K, leaf: NodeBox<K, V>) -> NodeBox<K, V>
where
    K: KeyBytes + ?Sized,
{
    debug_assert!(leaf.as_ref().is::<NodeLeaf<_, _>>());
    let curr = root;
    let pref_common_len = b.common_prefix_length(curr.prefix()).unwrap();
    if pref_common_len == b.len() {
        // exact match, return the leaf node.
        assert_eq!(b.len(), curr.prefix().len());
        assert!(curr.is::<NodeLeaf<_, _>>());
        unsafe {
            leaf.as_ref()
                .as_ptr()
                .replace(NodeHeader::new(b, b.len(), NodeHeaderData::leaf()));
        }
        return leaf;
    }

    if pref_common_len == curr.prefix().len() {
        // prefixed matched uses remaining key to insert node.
        let key = b.at(pref_common_len).unwrap();
        let new_b = b.drop_prefix(pref_common_len + 1);

        if let Some(x) = curr.get(key) {
            let branch = insert_node(x, new_b, leaf);
            return copy_insert(curr, key, branch);
        } else {
            unsafe {
                leaf.as_ref().as_ptr().replace(NodeHeader::new(
                    new_b,
                    new_b.len(),
                    NodeHeaderData::leaf(),
                ));
            }
            return copy_insert(curr, key, leaf);
        }
    }

    // key diverges in the middle of the prefix.
    // Create a new node 4 with the new leaf node and a copy of the old node with a new prefix.
    assert!(pref_common_len < curr.prefix().len());

    let new_key = b.at(pref_common_len).unwrap();
    let new_b = b.drop_prefix(pref_common_len + 1);

    let old_key = curr.prefix()[pref_common_len];

    unsafe {
        leaf.as_ref()
            .as_ptr()
            .replace(NodeHeader::new(new_b, new_b.len(), NodeHeaderData::leaf()));
    }

    let old_node = curr.copy_drop_prefix(pref_common_len + 1);
    NodeBox::new(Node4::new_split(
        b,
        pref_common_len,
        (new_key, leaf),
        (old_key, old_node),
    ))
}

fn copy_insert<K, V>(target: NodeRef<K, V>, key: u8, node: NodeBox<K, V>) -> NodeBox<K, V>
where
    K: KeyBytes + ?Sized,
{
    target.copy_insert(key, node)
}

impl<K: KeyBytes + ?Sized, V> Default for RawAart<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
