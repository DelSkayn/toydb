use bytemuck::{Pod, TransparentWrapper, Zeroable};
use core::fmt;
use std::{cmp::Ordering, marker::PhantomData};

use crate::raw::nodes::NodeHeader;

use self::{inline_buffer::InlineStorage, pod::PodStorageU8};

mod inline_buffer;
mod pod;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct NodeData(pub(crate) [u8; 3]);

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
pub unsafe trait KeyStorage<K: KeyBytes + ?Sized>: Sized {
    /// Create the storage for a key.
    fn store(key: &K, until: usize, data: NodeData) -> Self;

    fn new_from(existing: &Self, data: NodeData) -> Self;

    /// Return a reference to NodeData.
    ///
    /// The implementation of this trait must ensure that the reference to this node data is to the
    /// same object as given by the call to store.
    fn data(&self) -> NodeData;

    /// Retrieve the prefix stored in the storage.
    fn prefix(&self) -> &[u8];

    /// Drop the start of the key, after calling this the storage should only contain [offset..]
    fn copy_drop_prefix(&self, offset: usize) -> Self;

    /// Append the prefix followed by the key to the current prefix.
    fn prepend_prefix(&mut self, prefix: &[u8], key: u8);
}

pub struct KeyPrefixError;

impl fmt::Debug for KeyPrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "key was a prefix of an existing key")
    }
}

impl fmt::Display for KeyPrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "key was a prefix of an existing key")
    }
}

pub trait IntoKey<Key> {
    fn into_key(self) -> Key;
}

impl<K: Key> IntoKey<Self> for K {
    fn into_key(self) -> Self {
        self
    }
}

/// A art key
pub trait Key {
    type Bytes: KeyBytes + ?Sized;

    fn as_key_bytes(&self) -> &Self::Bytes;
}

pub trait KeyBytes {
    type Storage: KeyStorage<Self>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn at(&self, idx: usize) -> Option<u8>;

    fn drop_prefix(&self, start: usize) -> &Self;

    fn common_prefix_length(&self, other: &[u8]) -> Result<usize, KeyPrefixError> {
        for (idx, p) in other.iter().copied().enumerate() {
            let k = self.at(idx).ok_or(KeyPrefixError)?;
            if p != k {
                return Ok(idx);
            }
        }
        Ok(other.len())
    }
}

impl Key for str {
    type Bytes = StrBytes;

    fn as_key_bytes(&self) -> &Self::Bytes {
        StrBytes::from_bytes(self.as_bytes())
    }
}

// A byte which is not allowed to continue a string in a valid utf-8 string
// Specifically a byte tagged with a continue bit pattern.
//
// This ensures that no string can be a prefix of another.
pub const INVALID_STR_BYTE: u8 = 0b1011_1111;

#[repr(transparent)]
pub struct PostfixedBytes<const POSTFIX: u8>([u8]);
unsafe impl<const POSTFIX: u8> TransparentWrapper<[u8]> for PostfixedBytes<POSTFIX> {}

impl<const POSTFIX: u8> PostfixedBytes<{ POSTFIX }> {
    pub fn from_bytes(b: &[u8]) -> &Self {
        TransparentWrapper::wrap_ref(b)
    }
}

impl<const POSTFIX: u8> KeyBytes for PostfixedBytes<POSTFIX> {
    type Storage = InlineStorage;

    fn len(&self) -> usize {
        self.0.len() + 1
    }

    fn at(&self, idx: usize) -> Option<u8> {
        match idx.cmp(&self.0.len()) {
            Ordering::Less => Some(self.0[idx]),
            Ordering::Equal => Some(POSTFIX),
            Ordering::Greater => None,
        }
    }

    fn drop_prefix(&self, start: usize) -> &Self {
        Self::wrap_ref(&self.0[start..])
    }
}

pub type StrBytes = PostfixedBytes<INVALID_STR_BYTE>;

#[repr(transparent)]
pub struct PodBytes<P> {
    _marker: PhantomData<P>,
    bytes: [u8],
}
unsafe impl<P> TransparentWrapper<[u8]> for PodBytes<P> {}

impl<P: Pod> KeyBytes for PodBytes<P> {
    type Storage = InlineStorage;

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn at(&self, idx: usize) -> Option<u8> {
        self.bytes.get(idx).copied()
    }

    fn drop_prefix(&self, start: usize) -> &Self {
        Self::wrap_ref(&self.bytes[start..])
    }
}

#[repr(transparent)]
pub struct PodKey<P: Pod>(P);
unsafe impl<P: Pod> TransparentWrapper<P> for PodKey<P> {}

impl KeyBytes for [u8] {
    type Storage = InlineStorage;

    fn len(&self) -> usize {
        self.len()
    }

    fn at(&self, idx: usize) -> Option<u8> {
        self.get(idx).copied()
    }

    fn drop_prefix(&self, start: usize) -> &Self {
        &self[start..]
    }
}

#[repr(transparent)]
pub struct PodBytesU8<P> {
    _marker: PhantomData<P>,
    bytes: [u8],
}

unsafe impl<P> TransparentWrapper<[u8]> for PodBytesU8<P> {}

impl<P: Pod> KeyBytes for PodBytesU8<P> {
    type Storage = PodStorageU8<P>;

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn at(&self, idx: usize) -> Option<u8> {
        self.bytes.get(idx).copied()
    }

    fn drop_prefix(&self, start: usize) -> &Self {
        Self::wrap_ref(&self.bytes[start..])
    }
}

macro_rules! impl_pod {
    ($($t:ident),*$(,)?) => {
        $(
            impl Key for $t{
                type Bytes = PodBytesU8<$t>;

                fn as_key_bytes(&self) -> &Self::Bytes{
                    PodBytesU8::wrap_ref(bytemuck::bytes_of(self))
                }
            }
        )*
    }
}
impl_pod!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, usize, isize);
//impl_pod!(u8);
