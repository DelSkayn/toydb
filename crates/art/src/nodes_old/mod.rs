use std::{alloc::Layout, marker::PhantomData, ptr::NonNull};

mod node4;
pub use node4::Node4;

use crate::header::{NodeHeader, NodeKind};

#[repr(transparent)]
pub struct NodePtr<V> {
    ptr: NonNull<NodeHeader>,
    _marker: PhantomData<V>,
}

#[repr(u8)]
pub enum NodeRef<'a, V> {
    Leaf(&'a LeafNode<V>) = 0,
    Node4(&'a Node4<V>) = 1,
    Node16(&'a Node16<V>) = 2,
    Node48(&'a Node48<V>) = 3,
    Node256(&'a Node256<V>) = 4,
}

#[repr(u8)]
pub enum NodeMut<'a, V> {
    Leaf(&'a mut LeafNode<V>) = 0,
    Node4(&'a mut Node4<V>) = 1,
    Node16(&'a mut Node16<V>) = 2,
    Node48(&'a mut Node48<V>) = 3,
    Node256(&'a mut Node256<V>) = 4,
}

impl<V> NodePtr<V> {
    pub unsafe fn from_node_ptr(ptr: NonNull<NodeHeader>) -> Self {
        NodePtr {
            ptr: ptr.cast(),
            _marker: PhantomData,
        }
    }

    pub unsafe fn dangling() -> Self {
        NodePtr {
            ptr: NonNull::dangling(),
            _marker: PhantomData,
        }
    }

    pub fn header(&self) -> &NodeHeader {
        unsafe { self.ptr.as_ref() }
    }

    pub fn header_mut(&mut self) -> &mut NodeHeader {
        unsafe { self.ptr.as_mut() }
    }

    pub fn as_ref(&self) -> NodeRef<V> {
        unsafe {
            let kind = self.ptr.as_ref().kind;
            match kind {
                NodeKind::Leaf => NodeRef::Leaf(self.ptr.cast().as_ref()),
                NodeKind::Node4 => NodeRef::Node4(self.ptr.cast().as_ref()),
                NodeKind::Node16 => NodeRef::Node16(self.ptr.cast().as_ref()),
                NodeKind::Node48 => NodeRef::Node48(self.ptr.cast().as_ref()),
                NodeKind::Node256 => NodeRef::Node256(self.ptr.cast().as_ref()),
            }
        }
    }

    pub fn as_mut(&mut self) -> NodeMut<V> {
        unsafe {
            let kind = self.ptr.as_ref().kind;
            match kind {
                NodeKind::Leaf => NodeMut::Leaf(self.ptr.cast().as_mut()),
                NodeKind::Node4 => NodeMut::Node4(self.ptr.cast().as_mut()),
                NodeKind::Node48 => NodeMut::Node48(self.ptr.cast().as_mut()),
                NodeKind::Node16 => NodeMut::Node16(self.ptr.cast().as_mut()),
                NodeKind::Node256 => NodeMut::Node256(self.ptr.cast().as_mut()),
            }
        }
    }

    pub fn lookup(&self, key: u8) -> Option<&NodePtr<V>> {
        unsafe {
            let kind = self.ptr.as_ref().kind;
            match kind {
                NodeKind::Leaf => panic!(),
                NodeKind::Node4 => self.ptr.cast::<Node4<V>>().as_mut().lookup(key),
                NodeKind::Node16 => self.ptr.cast::<Node16<V>>().as_mut().lookup(key),
                NodeKind::Node48 => self.ptr.cast::<Node48<V>>().as_mut().lookup(key),
                NodeKind::Node256 => self.ptr.cast::<Node256<V>>().as_mut().lookup(key),
            }
        }
    }

    pub fn lookup_mut(&mut self, key: u8) -> Option<&mut NodePtr<V>> {
        unsafe {
            let kind = self.ptr.as_ref().kind;
            match kind {
                NodeKind::Leaf => panic!(),
                NodeKind::Node4 => self.ptr.cast::<Node4<V>>().as_mut().lookup_mut(key),
                NodeKind::Node16 => self.ptr.cast::<Node16<V>>().as_mut().lookup_mut(key),
                NodeKind::Node48 => self.ptr.cast::<Node48<V>>().as_mut().lookup_mut(key),
                NodeKind::Node256 => self.ptr.cast::<Node256<V>>().as_mut().lookup_mut(key),
            }
        }
    }

    pub fn insert_at(&mut self, key: u8, node: NodePtr<V>) {
        unsafe {
            let kind = self.ptr.as_ref().kind;
            match kind {
                NodeKind::Leaf => panic!(),
                NodeKind::Node4 => {
                    if let Some(x) = Self::insert_new_add_node4(self.ptr.cast(), key, node) {
                        self.ptr = x.cast();
                    }
                }
                NodeKind::Node16 => {
                    if let Some(x) = Self::insert_new_add_node16(self.ptr.cast(), key, node) {
                        self.ptr = x.cast();
                    }
                }
                NodeKind::Node48 => {
                    if let Some(x) = Self::insert_new_add_node48(self.ptr.cast(), key, node) {
                        self.ptr = x.cast();
                    }
                }
                NodeKind::Node256 => {
                    Self::insert_new_add_node256(self.ptr.cast(), key, node);
                }
            }
        }
    }

    unsafe fn insert_new_add_node4(
        mut this: NonNull<Node4<V>>,
        key: u8,
        ptr: NodePtr<V>,
    ) -> Option<NonNull<Node16<V>>> {
        let len = this.as_ref().header.len;
        if len == 4 {
            let dst_ptr = Node4::grow(this);

            dst_ptr.as_mut().key[4] = key;
            dst_ptr.as_mut().ptr[4] = ptr;
            dst_ptr.as_mut().header.len = 5;
            return Some(dst_ptr);
        }
        this.as_mut().key[len as usize] = key;
        this.as_mut().ptr[len as usize] = ptr;
        this.as_mut().header.len += 1;
        None
    }

    unsafe fn insert_new_add_node16(
        mut this: NonNull<Node16<V>>,
        key: u8,
        ptr: NodePtr<V>,
    ) -> Option<NonNull<Node48<V>>> {
        let len = this.as_ref().header.len;
        if len == 16 {
            let new_ptr = NonNull::new(std::alloc::realloc(
                this.as_ptr().cast(),
                Layout::new::<Node16<V>>(),
                std::mem::size_of::<Node48<V>>(),
            ))
            .unwrap()
            .cast::<Node16<V>>();
            let mut dst_ptr = new_ptr.cast::<Node48<V>>();
            for i in 0..16u8 {
                let key = new_ptr.as_ref().key[i as usize];
                dst_ptr.as_mut().idx[key as usize] = i;
            }

            dst_ptr.as_mut().header.kind = NodeKind::Node48;
            dst_ptr.as_mut().idx[key as usize] = 16;
            dst_ptr.as_mut().ptr[16] = Some(ptr);
            dst_ptr.as_mut().ptr[17..].fill(None);
            dst_ptr.as_mut().header.len = 17;
            return Some(dst_ptr);
        }
        this.as_mut().key[len as usize] = key;
        this.as_mut().ptr[len as usize] = ptr;
        this.as_mut().header.len += 1;
        None
    }

    unsafe fn insert_new_add_node48(
        mut this: NonNull<Node48<V>>,
        key: u8,
        ptr: NodePtr<V>,
    ) -> Option<NonNull<Node256<V>>> {
        let len = this.as_ref().header.len;
        if len == 48 {
            let new_node = NonNull::new(std::alloc::alloc(Layout::new::<Node256<V>>())).unwrap();
            // copy over the header
            std::ptr::copy(
                this.as_ptr().cast::<u8>(),
                new_node.as_ptr(),
                std::mem::size_of::<NodeHeader>(),
            );
            // initialize the rest as zero
            std::ptr::write_bytes(
                new_node.as_ptr().add(std::mem::size_of::<NodeHeader>()),
                0,
                std::mem::size_of::<Node256<V>>() - std::mem::size_of::<NodeHeader>(),
            );

            let mut dst_ptr = new_node.cast::<Node256<V>>();

            for i in 0..256 {
                let idx = this.as_ref().idx[i];
                if idx >= 48 {
                    continue;
                }
                dst_ptr.as_mut().ptr[i] = this.as_ref().ptr[idx as usize];
            }
            dst_ptr.as_mut().header.kind = NodeKind::Node256;
            dst_ptr.as_mut().header.len = 49;

            std::alloc::dealloc(this.as_ptr().cast(), Layout::new::<Node48<V>>());

            return Some(dst_ptr);
        }
        let postion = this.as_mut().ptr.iter().position(|x| x.is_none()).unwrap();
        this.as_mut().idx[key as usize] = postion as u8;
        this.as_mut().ptr[postion] = Some(ptr);
        this.as_mut().header.len += 1;
        None
    }

    unsafe fn insert_new_add_node256(mut this: NonNull<Node256<V>>, key: u8, ptr: NodePtr<V>) {
        this.as_mut().ptr[key as usize] = Some(ptr);
        this.as_mut().header.len += 1;
    }
}

#[repr(C)]
pub struct LeafNode<V> {
    header: NodeHeader,
    pub value: V,
}

impl<V> LeafNode<V> {
    pub fn new(prefix: &[u8], v: V) -> Self {
        LeafNode {
            header: NodeHeader::new_for_prefix(NodeKind::Leaf, prefix),
            value: v,
        }
    }
}

#[repr(C)]
pub struct Node16<V> {
    header: NodeHeader,
    ptr: [NodePtr<V>; 16],
    key: [u8; 16],
}

impl<V> Node16<V> {
    pub fn lookup(&self, key: u8) -> Option<&NodePtr<V>> {
        let idx = self.key.iter().position(|x| *x == key)?;
        Some(&self.ptr[idx])
    }

    pub fn lookup_mut(&mut self, key: u8) -> Option<&mut NodePtr<V>> {
        let idx = self.key.iter().position(|x| *x == key)?;
        Some(&mut self.ptr[idx])
    }
}

#[repr(C)]
pub struct Node48<V> {
    header: NodeHeader,
    ptr: [Option<NodePtr<V>>; 48],
    idx: [u8; 256],
}

impl<V> Node48<V> {
    pub fn lookup(&self, key: u8) -> Option<&NodePtr<V>> {
        let lookup = self.idx[key as usize];
        if lookup >= 48 {
            return None;
        }
        self.ptr[lookup as usize].as_ref()
    }

    pub fn lookup_mut(&mut self, key: u8) -> Option<&mut NodePtr<V>> {
        let lookup = self.idx[key as usize];
        if lookup >= 48 {
            return None;
        }
        self.ptr[lookup as usize].as_mut()
    }
}

#[repr(C)]
pub struct Node256<V> {
    header: NodeHeader,
    ptr: [Option<NodePtr<V>>; 256],
}

impl<V> Node256<V> {
    pub fn lookup_mut(&mut self, key: u8) -> Option<&mut NodePtr<V>> {
        self.ptr[key as usize].as_mut()
    }

    pub fn lookup(&self, key: u8) -> Option<&NodePtr<V>> {
        self.ptr[key as usize].as_ref()
    }
}
