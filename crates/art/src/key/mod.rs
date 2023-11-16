use std::ops::Range;

use crate::header::NodeData;

mod inline_buffer;
mod pod;

pub use pod::PodStorageU8;

use self::inline_buffer::InlineStorage;

pub trait KeyStorage<K: Key + ?Sized>: Sized {
    /// Create the storage for a key.
    fn store(key: &K, range: Range<usize>, data: NodeData) -> Self;

    fn data(&self) -> &NodeData;

    fn data_mut(&mut self) -> &mut NodeData;

    /// Retrieve the
    fn key(&self) -> &[u8];

    // Drop the start of the key, after calling this the storage should only contain [offset..]
    fn drop_start(&mut self, offset: usize);
}

/// A art key
pub trait Key {
    type Storage: KeyStorage<Self>;

    fn len(&self) -> usize;

    fn at(&self, idx: usize) -> u8;
}

// A byte which is not allowed to continue a string in a valid utf-8 string
// Specifically a byte tagged with a continue bit pattern.
//
// This ensures that no string can be a prefix of another.
const INVALID_STR_BYTE: u8 = 0b1011_1111;

impl Key for str {
    type Storage = InlineStorage;

    fn at(&self, idx: usize) -> u8 {
        if idx >= self.len() {
            INVALID_STR_BYTE
        } else {
            self.as_bytes()[idx]
        }
    }

    fn len(&self) -> usize {
        // +1 for the INVALID_STR_BYTE
        self.len() + 1
    }
}

macro_rules! impl_pod {
    ($($t:ident),*$(,)?) => {
        $(
            impl Key for $t{
                type Storage = PodStorageU8<$t>;

                fn len(&self) -> usize{
                    ::std::mem::size_of::<$t>()
                }

                fn at(&self, idx: usize) -> u8{
                    bytemuck::bytes_of(self)[idx]
                }
            }
        )*
    }
}
impl_pod!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, usize, isize);
//impl_pod!(u8);
