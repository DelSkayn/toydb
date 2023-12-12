mod boxed_node;
mod header;
mod leaf;
mod node16;
mod node256;
mod node4;
mod node48;
mod owned_node;

pub use boxed_node::{BoxedNode, RawBoxedNode};
pub use header::{NodeData, NodeHeader, NodeKind};
pub use leaf::LeafNode;
pub use node16::Node16;
pub use node256::Node256;
pub use node4::Node4;
pub use node48::Node48;
pub use owned_node::{OwnedNode, RawOwnedNode};

use crate::key::Key;

/// # Safety
/// Implementor must ensure that the associated KIND value is distinct from any other type
/// impelementing NodeType.
pub unsafe trait NodeType {
    const KIND: NodeKind;
    type Key: Key + ?Sized;
    type Value;
}
