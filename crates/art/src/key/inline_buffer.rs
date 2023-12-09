use std::{alloc::Layout, mem::MaybeUninit, ops::Range, ptr::NonNull};

use crate::nodes::NodeData;

use super::{Key, KeyStorage};

const INLINE_MAX: u8 = std::mem::size_of::<NonNull<u8>>() as u8;
const INLINE_FULL: u8 = INLINE_MAX + 1;

/// Inline buffer for generic byte-like keys.
#[repr(transparent)]
pub struct InlineHeader {
    len: usize,
}

pub union InlinedUnion {
    ptr: NonNull<InlineHeader>,
    inline: MaybeUninit<[u8; INLINE_MAX as usize]>,
}

pub struct InlineStorage {
    buffer: InlinedUnion,
    len: u8,
    data: NodeData,
}

impl InlineStorage {
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

    pub unsafe fn new(len: usize, data: NodeData) -> Self {
        if len <= INLINE_MAX as usize {
            return InlineStorage {
                buffer: InlinedUnion {
                    inline: MaybeUninit::uninit(),
                },
                len: len as u8,
                data,
            };
        }

        let ptr = unsafe { Self::allocate_buffer(len) };
        InlineStorage {
            buffer: InlinedUnion { ptr },
            len: INLINE_FULL,
            data,
        }
    }

    fn buffer_ptr(&mut self) -> NonNull<u8> {
        unsafe {
            if self.len <= INLINE_MAX {
                NonNull::new_unchecked(self.buffer.inline.as_mut_ptr().cast::<u8>())
            } else {
                NonNull::new_unchecked(self.buffer.ptr.as_ptr().add(1).cast::<u8>())
            }
        }
    }

    fn key(&self) -> &[u8] {
        unsafe {
            if self.len < INLINE_FULL {
                return std::slice::from_raw_parts(
                    self.buffer.inline.as_ptr().cast(),
                    self.len as usize,
                );
            }
            let len = self.buffer.ptr.as_ref().len;
            let ptr = self.buffer.ptr.as_ptr().add(1).cast();
            std::slice::from_raw_parts(ptr, len)
        }
    }

    fn key_mut(&mut self) -> &mut [u8] {
        unsafe {
            if self.len < INLINE_FULL {
                return std::slice::from_raw_parts_mut(
                    self.buffer.inline.as_mut_ptr().cast(),
                    self.len as usize,
                );
            }
            let len = self.buffer.ptr.as_ref().len;
            let ptr = self.buffer.ptr.as_ptr().add(1).cast();
            std::slice::from_raw_parts_mut(ptr, len)
        }
    }
}

impl Drop for InlineStorage {
    fn drop(&mut self) {
        if self.len < INLINE_FULL {
            return;
        }
        unsafe { Self::free_buffer(self.buffer.ptr) };
    }
}

unsafe impl<K: Key + ?Sized> KeyStorage<K> for InlineStorage {
    fn store(key: &K, range: Range<usize>, data: NodeData) -> Self {
        let mut this = unsafe { InlineStorage::new(range.len(), data) };
        let mut ptr = this.buffer_ptr().as_ptr();
        unsafe {
            for i in range {
                ptr.write(key.at(i));
                ptr = ptr.add(1);
            }
        }
        this
    }

    fn data(&self) -> &NodeData {
        &self.data
    }

    fn data_mut(&mut self) -> &mut NodeData {
        &mut self.data
    }

    fn prefix(&self) -> &[u8] {
        self.key()
    }

    fn drop_prefix(&mut self, offset: usize) {
        let key = self.key();
        let mut new = unsafe { InlineStorage::new(key.len() - offset, self.data) };
        unsafe {
            std::ptr::copy_nonoverlapping(
                key[offset..].as_ptr(),
                new.buffer_ptr().as_ptr(),
                key.len() - offset,
            );
        }

        *self = new;
    }

    fn prepend_prefix(&mut self, prefix: &[u8], key: u8) {
        let len = if self.len < INLINE_MAX {
            self.len as usize
        } else {
            unsafe { self.buffer.ptr.as_ref().len }
        };
        let new_len = len + prefix.len() + 1;
        unsafe {
            let mut new = InlineStorage::new(new_len, self.data);
            // copy new prefix.
            std::ptr::copy_nonoverlapping(prefix.as_ptr(), new.buffer_ptr().as_ptr(), prefix.len());
            // copy the key.
            let offset_ptr = new.buffer_ptr().as_ptr().add(prefix.len());
            offset_ptr.write(key);
            // copy old prefix.
            let offset_ptr = offset_ptr.add(1);
            std::ptr::copy_nonoverlapping(self.key().as_ptr(), offset_ptr, self.key().len());
            *self = new;
        };
    }
}
