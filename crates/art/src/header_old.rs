use std::{alloc::Layout, mem::MaybeUninit, ptr::NonNull};

const INLINE_MAX: u8 = std::mem::size_of::<NonNull<u8>>() as u8;
const INLINE_FULL: u8 = INLINE_MAX + 1;

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum NodeKind {
    Leaf = 0,
    Node4 = 1,
    Node16 = 2,
    Node48 = 3,
    Node256 = 4,
}

#[repr(transparent)]
pub struct InlineHeader {
    len: usize,
}

pub union InlinedBuffer {
    ptr: NonNull<InlineHeader>,
    inline: MaybeUninit<[u8; INLINE_MAX as usize]>,
}

impl InlinedBuffer {
    unsafe fn allocate_buffer(len: usize) -> NonNull<InlineHeader> {
        let (layout, offset) = Layout::new::<InlineHeader>()
            .extend(Layout::array::<u8>(len).unwrap())
            .unwrap();
        assert_eq!(offset, INLINE_MAX as usize);

        let buffer = std::alloc::alloc(layout).cast::<InlineHeader>();
        let buffer = NonNull::new(buffer).unwrap();
        buffer.as_ptr().write(InlineHeader { len });
        buffer
    }

    unsafe fn free_buffer(ptr: NonNull<InlineHeader>) {
        let len = ptr.as_ref().len;
        let (layout, _) = Layout::new::<InlineHeader>()
            .extend(Layout::array::<u8>(len).unwrap())
            .unwrap();
        std::alloc::dealloc(ptr.as_ptr().cast(), layout);
    }

    pub unsafe fn allocate(slice: &[u8], len: &mut u8) -> InlinedBuffer {
        if slice.len() <= INLINE_MAX as usize {
            *len = slice.len() as u8;
            let mut inline: MaybeUninit<[u8; INLINE_MAX as usize]> = MaybeUninit::uninit();
            let ptr = inline.as_mut_ptr().cast::<u8>();
            std::ptr::copy_nonoverlapping(slice.as_ptr(), ptr, slice.len());
            return InlinedBuffer { inline };
        }

        let buffer = Self::allocate_buffer(slice.len());
        std::ptr::copy_nonoverlapping(
            slice.as_ptr(),
            buffer.as_ptr().add(1).cast::<u8>(),
            slice.len(),
        );
        InlinedBuffer { ptr: buffer }
    }

    pub unsafe fn free(&self, len: u8) {
        if len != INLINE_FULL {
            return;
        }

        let (layout, _) = Layout::new::<InlineHeader>()
            .extend(Layout::array::<u8>(self.ptr.as_ref().len).unwrap())
            .unwrap();

        std::alloc::dealloc(self.ptr.as_ptr().cast(), layout);
    }

    pub unsafe fn as_slice(&self, len: u8) -> &[u8] {
        if len != INLINE_FULL {
            let len = self.ptr.as_ref().len;
            let ptr = self.ptr.as_ptr().add(1).cast::<u8>();
            return std::slice::from_raw_parts(ptr, len);
        }
        &self.inline.assume_init_ref()[..len as usize]
    }

    pub unsafe fn split_prefix(&mut self, len: &mut u8, at: usize) {
        let (src_ptr, cur_len) = if *len != INLINE_FULL {
            (self.ptr.as_ptr().add(1).cast::<u8>(), self.ptr.as_ref().len)
        } else {
            (self.inline.as_mut_ptr().cast::<u8>(), *len as usize)
        };
        assert!(cur_len >= at);
        let new_len = cur_len - at;
        let dst_ptr = if new_len <= INLINE_MAX as usize {
            NonNull::from(&self.inline).cast::<u8>()
        } else {
            Self::allocate_buffer(new_len).cast()
        };
        std::ptr::copy(src_ptr.add(at), dst_ptr.as_ptr(), new_len);
        if *len == INLINE_FULL {
            Self::free_buffer(self.ptr);
        }
        if new_len > INLINE_MAX as usize {
            *len = INLINE_FULL;
            self.ptr = dst_ptr.cast();
        } else {
            *len = new_len as u8
        }
    }
}

pub struct NodeHeader {
    prefix: InlinedBuffer,
    pub kind: NodeKind,
    // the amount of leaf nodes present.
    pub len: u8,
    // the length of the inline buffer if inlined.
    inline_len: u8,
}

impl NodeHeader {
    pub fn new_for_prefix(kind: NodeKind, prefix: &[u8]) -> NodeHeader {
        let mut inline_len = 0u8;
        let prefix = unsafe { InlinedBuffer::allocate(prefix, &mut inline_len) };
        NodeHeader {
            prefix,
            kind,
            len: 0,
            inline_len,
        }
    }

    pub fn prefix(&self) -> &[u8] {
        unsafe { self.prefix.as_slice(self.inline_len) }
    }

    pub fn split_prefix(&mut self, at: usize) {
        unsafe { self.prefix.split_prefix(&mut self.inline_len, at) }
    }
}

impl Drop for NodeHeader {
    fn drop(&mut self) {
        unsafe { self.prefix.free(self.inline_len) }
    }
}
