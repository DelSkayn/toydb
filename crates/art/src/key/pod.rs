use super::{Key, KeyStorage};
use crate::header::NodeData;
use bytemuck::Pod;
use std::{mem::MaybeUninit, ops::Range};

pub struct PodStorageU8<T: Pod> {
    value: MaybeUninit<T>,
    len: u8,
    data: NodeData,
}

impl<T: Pod + Key> KeyStorage<T> for PodStorageU8<T> {
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

    fn key(&self) -> &[u8] {
        dbg!(self.len);
        unsafe { std::slice::from_raw_parts(self.value.as_ptr().cast(), self.len as usize) }
    }

    fn drop_start(&mut self, offset: usize) {
        let slice = unsafe { bytemuck::bytes_of_mut(self.value.assume_init_mut()) };
        let new_len = slice.len().checked_sub(offset).unwrap();
        slice.copy_within(offset.., 0);
        self.len = new_len as u8;
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
