use super::{KeyBytes, KeyStorage, NodeData, PodBytesU8};
use bytemuck::Pod;
use std::mem::MaybeUninit;

pub struct PodStorageU8<T: Pod> {
    value: MaybeUninit<T>,
    len: u8,
    data: NodeData,
}

unsafe impl<P: Pod> KeyStorage<PodBytesU8<P>> for PodStorageU8<P> {
    #[inline]
    fn store(key: &PodBytesU8<P>, until: usize, data: NodeData) -> Self {
        assert!(std::mem::size_of::<P>() < u8::MAX as usize);
        assert!(until <= std::mem::size_of::<P>());

        let mut value: MaybeUninit<P> = MaybeUninit::uninit();
        let mut ptr = value.as_mut_ptr().cast::<u8>();
        for i in 0..until {
            unsafe { ptr.write(key.at(i).unwrap()) };
            unsafe { ptr = ptr.add(1) };
        }
        PodStorageU8 {
            value,
            len: until as u8,
            data,
        }
    }

    fn new_from(existing: &Self, data: NodeData) -> Self {
        Self {
            value: existing.value,
            len: existing.len,
            data,
        }
    }

    fn data(&self) -> NodeData {
        self.data
    }

    fn prefix(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.value.as_ptr().cast(), self.len as usize) }
    }

    fn copy_drop_prefix(&self, offset: usize) -> Self {
        if offset == self.len as usize {
            return PodStorageU8 {
                value: MaybeUninit::uninit(),
                len: 0,
                data: self.data,
            };
        }
        assert!(offset < self.len as usize);
        let count = self.len as usize - offset;

        let mut value: MaybeUninit<P> = MaybeUninit::uninit();

        unsafe {
            let from = self.value.as_ptr().cast::<u8>().add(offset);
            let to = value.as_mut_ptr().cast::<u8>();
            std::ptr::copy(from, to, count);
        }

        PodStorageU8 {
            len: count as u8,
            value,
            data: self.data,
        }
    }

    fn prepend_prefix(&mut self, prefix: &[u8], key: u8) {
        let new_len = self.len as usize + prefix.len() + 1;

        assert!(new_len <= std::mem::size_of::<P>());

        unsafe {
            let from = self.value.as_mut_ptr().cast::<u8>();
            // +1 for the key
            let to = from.add(prefix.len() + 1);
            std::ptr::copy(from, to, self.len as usize);
            std::ptr::copy(prefix.as_ptr(), from, prefix.len());
            from.add(prefix.len()).write(key);
        }

        self.len = new_len as u8
    }
}
