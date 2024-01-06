use std::fmt::{Debug, Display};

use hash40::Hash40;

use crate::BinaryRepr;

/// A 4-byte aligned Hash40 value
///
/// This value is used over a [`u64`] because a [`u64`] has different alignment on different systems.
///
/// For example, on x86-64, a `u64`` is aligned to the 4-byte boundary but on arm systems it is aligned on 8-bytes
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct Hash {
    pub crc: u32,
    pub len: u8,
}

impl std::hash::Hash for Hash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash40().0)
    }
}

impl Hash {
    /// Gets the hash value as a [`Hash40`], more useful for most operations
    pub const fn hash40(&self) -> Hash40 {
        Hash40(((self.len as u64) << 32) | (self.crc as u64))
    }

    /// Gets the length of the hash value
    #[allow(dead_code)]
    pub const fn length(&self) -> usize {
        self.len as usize
    }
}

impl Debug for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.hash40(), f)
    }
}

/// A 4-byte aligned Hash40 value
///
/// This value is used over a [`u64`] because a [`u64`] has different alignment on different systems.
///
/// For example, on x86-64, a `u64`` is aligned to the 4-byte boundary but on arm systems it is aligned on 8-bytes
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct HashWithData {
    pub crc: u32,
    len_and_data: u32,
}

impl HashWithData {
    pub const fn new(hash: Hash40, data: u32) -> Self {
        Self {
            crc: hash.crc(),
            len_and_data: (hash.str_len() as u32) | (data << 8),
        }
    }

    /// Gets the hash value as a [`Hash40`], more useful for most operations
    pub const fn hash40(&self) -> Hash40 {
        Hash40(((self.length() as u64) << 32) | (self.crc as u64))
    }

    /// Sets the hash of the hash value
    pub fn set_hash40(&mut self, hash: Hash40) {
        self.len_and_data = (self.len_and_data & 0xFFFF_FF00) | hash.str_len() as u32;
        self.crc = hash.crc();
    }

    /// Gets the length of the hash value
    pub const fn length(&self) -> usize {
        (self.len_and_data & 0xFF) as usize
    }

    /// Gets the data of the hash value
    pub const fn data(&self) -> u32 {
        (self.len_and_data & 0xFFFF_FF00) >> 8
    }

    /// Sets the data of the hash value
    pub fn set_data(&mut self, data: u32) {
        self.len_and_data = (self.len_and_data & 0xFF) | data << 24;
    }
}

impl Debug for HashWithData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashWithData")
            .field("hash", &format!("{}", self.hash40()))
            .field("data", &self.data())
            .finish()
    }
}

impl BinaryRepr for Hash {}
impl BinaryRepr for HashWithData {}
