use std::ops::Range;

use crate::nodes::NodeData;

mod inline_buffer;
mod pod;

pub use pod::PodStorageU8;

use self::inline_buffer::InlineStorage;

/// A trait used for the storage of key prefixes.
///
/// # Safety
/// This trait is marked unsafe to implement as implemenations which do not adhere to the following
/// rules will result in undefined behaviour.
///
/// - The implementation of this trait must ensure that any call to [`KeyStorage::data`] or
/// [`KeyStorage::data_mut`] returns an object with the same value as the one given by
/// [`KeyStorage::store`]. Further more the value object must not changed unless changed externally
/// by using [`KeyStorage::data_mut`].
///
/// In short the caller of this trait must be able to trust that the storage won't suddenly change
/// the value of NodeData.
pub unsafe trait KeyStorage<K: Key + ?Sized>: Sized {
    /// Create the storage for a key.
    fn store(key: &K, range: Range<usize>, data: NodeData) -> Self;

    /// Return a reference to NodeData.
    ///
    /// The implementation of this trait must ensure that the reference to this node data is to the
    /// same object as given by the call to store.
    fn data(&self) -> &NodeData;

    /// Return a mutable reference to NodeData.
    ///
    /// The implementation of this trait must ensure that the reference to this node data is to the
    /// same object as given by the call to store.
    fn data_mut(&mut self) -> &mut NodeData;

    /// Retrieve the prefix stored in the storage.
    fn prefix(&self) -> &[u8];

    /// Drop the start of the key, after calling this the storage should only contain [offset..]
    fn drop_prefix(&mut self, offset: usize);

    /// Append the prefix followed by the key to the current prefix.
    fn prepend_prefix(&mut self, prefix: &[u8], key: u8);
}

/// A art key
pub trait Key {
    type Storage: KeyStorage<Self>;

    fn len(&self) -> usize;

    fn at(&self, idx: usize) -> u8;
}

pub trait BorrowedKey {
    unsafe fn from_key_bytes(bytes: &[u8]) -> &Self;
}

impl BorrowedKey for str {
    unsafe fn from_key_bytes(bytes: &[u8]) -> &Self {
        std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap()
    }
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
