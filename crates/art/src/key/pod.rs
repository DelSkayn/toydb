use super::{Key, KeyStorage};
use crate::raw::NodeData;
use bytemuck::Pod;
use std::{mem::MaybeUninit, ops::Range};

pub struct PodStorageU8<T: Pod> {
    value: MaybeUninit<T>,
    len: u8,
    data: NodeData,
}

unsafe impl<T: Pod + Key> KeyStorage<T> for PodStorageU8<T> {
    #[inline]
    fn store(key: &T, range: Range<usize>, data: NodeData) -> Self {
        assert!(std::mem::size_of::<T>() < u8::MAX as usize);
        assert!(range.end <= std::mem::size_of::<T>());

        let mut value: MaybeUninit<T> = MaybeUninit::uninit();
        let mut ptr = value.as_mut_ptr().cast::<u8>();
        for i in range.clone() {
            unsafe { ptr.write(key.at(i)) };
            unsafe { ptr = ptr.add(1) };
        }
        PodStorageU8 {
            value,
            len: range.len() as u8,
            data,
        }
    }

    fn data(&self) -> &NodeData {
        &self.data
    }

    fn data_mut(&mut self) -> &mut NodeData {
        &mut self.data
    }

    fn prefix(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.value.as_ptr().cast(), self.len as usize) }
    }

    fn drop_prefix(&mut self, offset: usize) {
        let slice = unsafe {
            &mut bytemuck::bytes_of_mut(self.value.assume_init_mut())[..self.len as usize]
        };
        let new_len = slice.len().checked_sub(offset).unwrap();
        slice.copy_within(offset.., 0);
        self.len = new_len as u8;
    }

    fn prepend_prefix(&mut self, prefix: &[u8], key: u8) {
        let len = self.len as usize + prefix.len() + 1;
        let old = self.value;
        let slice = unsafe { &mut bytemuck::bytes_of_mut(self.value.assume_init_mut()) };
        let old_slice = unsafe { &bytemuck::bytes_of(old.assume_init_ref()) };
        slice[..prefix.len()].copy_from_slice(prefix);
        slice[prefix.len()] = key;
        slice[prefix.len() + 1..len].copy_from_slice(&old_slice[..self.len as usize]);
        self.len = len as u8
    }
}

/*
#[repr(transparent)]
pub struct PodKey<T: Pod>(T);

impl<T: Pod> Key for PodKey<T> {
    fn len(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn at(&self, idx: usize) -> u8 {
        bytemuck::bytes_of(&self.0)[idx]
    }
}
*/
