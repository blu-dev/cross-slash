#![feature(pointer_is_aligned)]

use hash40::Hash40;

#[cfg(not(target_endian = "little"))]
compile_error!(
    "this crate requires the host system to be operating on a little-endian architecture"
);

pub mod prelude {
    pub use super::archive::{
        file_data::FileData,
        file_desc::FileDesc,
        file_entity::FileEntity,
        file_group::FileGroup,
        file_info::FileInfo,
        file_package::{FilePackage, FilePackageChild},
        file_path::FilePath,
        stream_data::StreamData,
        stream_desc::StreamDesc,
        stream_folder::StreamFolder,
        stream_path::StreamPath,
    };
}

pub mod archive;
pub mod index;
pub mod refs;

mod hash;
mod io;

mod __sealed {
    pub trait Sealed {}
}

#[cfg(feature = "cast-sanity")]
#[inline(always)]
#[track_caller]
fn single_value_sanity<T: Sized>(bytes: &[u8]) {
    let this_ptr = bytes.as_ptr().cast::<T>();

    assert!(this_ptr.is_aligned());
    assert!(bytes.len() >= std::mem::size_of::<T>());
}

#[cfg(not(feature = "cast-sanity"))]
#[inline(always)]
fn single_value_sanity<T: Sized>(_bytes: &[u8]) {}

#[cfg(feature = "cast-sanity")]
#[inline(always)]
#[track_caller]
fn slice_sanity<T: Sized>(bytes: &[u8]) {
    let this_ptr = bytes.as_ptr().cast::<T>();

    assert!(this_ptr.is_aligned());
    assert_eq!(bytes.len() % std::mem::size_of::<T>(), 0x0);
}

#[cfg(not(feature = "cast-sanity"))]
#[inline(always)]
fn slice_sanity<T: Sized>(_bytes: &[u8]) {}

/// Trait that enables zero-copy reading of archive tables
pub trait BinaryRepr: Sized {
    /// Casts a slice of bytes to a reference of this type
    /// SAFETY: The caller must ensure that the bytes provided contain a valid representation of this type
    #[track_caller]
    unsafe fn cast(bytes: &[u8]) -> &Self {
        single_value_sanity::<Self>(bytes);

        &*bytes.as_ptr().cast::<Self>()
    }

    fn cast_bytes(&self) -> &[u8]
    where
        Self: Copy,
    {
        let ptr = (self as *const Self).cast::<u8>();
        // SAFETY: This slice is the exact size as this value
        unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of::<Self>()) }
    }

    /// Casts a slice of bytes to a slice of this type
    ///
    /// When the `cast-sanity` feature is disabled, this has the same functionality as [`BinaryRepr::cast_slice_trailing`]
    /// SAFETY: The caller must ensure that the bytes provided contain a valid representation of at least one
    ///         value of this type
    #[track_caller]
    unsafe fn cast_slice(bytes: &[u8]) -> &[Self] {
        slice_sanity::<Self>(bytes);

        std::slice::from_raw_parts(
            bytes.as_ptr().cast::<Self>(),
            bytes.len() / std::mem::size_of::<Self>(),
        )
    }

    fn cast_slice_bytes(slice: &[Self]) -> &[u8]
    where
        Self: Copy,
    {
        let ptr = slice.as_ptr().cast::<u8>();
        // SAFETY: This slice is the exact size as this value
        unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of::<Self>() * slice.len()) }
    }

    /// Casts a slice of bytes to a slice of this type, ignoring trailing bytes
    /// SAFETY: The caller must ensure that the bytes provided contain a valid representation of at least one
    ///         value of this type
    #[track_caller]
    unsafe fn cast_slice_trailing(bytes: &[u8]) -> &[Self] {
        single_value_sanity::<Self>(bytes);

        std::slice::from_raw_parts(
            bytes.as_ptr().cast::<Self>(),
            bytes.len() / std::mem::size_of::<Self>(),
        )
    }

    /// Casts a slice of bytes to a mutable reference of this type
    /// SAFETY: The caller must ensure that the bytes provided contain a valid representation of this type
    #[track_caller]
    unsafe fn cast_mut(bytes: &mut [u8]) -> &mut Self {
        single_value_sanity::<Self>(bytes);

        &mut *bytes.as_mut_ptr().cast::<Self>()
    }

    /// Casts a slice of bytes to a mutable slice of this type
    ///
    /// When the `cast-sanity` feature is disabled, this has the same functionality as [`BinaryRepr::cast_slice_trailing_mut`]
    /// SAFETY: The caller must ensure that the bytes provided contain a valid representation of at least one
    ///         value of this type
    #[track_caller]
    unsafe fn cast_slice_mut(bytes: &mut [u8]) -> &mut [Self] {
        slice_sanity::<Self>(bytes);

        std::slice::from_raw_parts_mut(
            bytes.as_mut_ptr().cast::<Self>(),
            bytes.len() / std::mem::size_of::<Self>(),
        )
    }

    /// Casts a slice of bytes to a mutable slice of this type, ignoring trailing bytes
    /// SAFETY: The caller must ensure that the bytes provided contain a valid representation of at least one
    ///         value of this type
    #[track_caller]
    unsafe fn cast_slice_trailing_mut(bytes: &mut [u8]) -> &mut [Self] {
        single_value_sanity::<Self>(bytes);

        std::slice::from_raw_parts_mut(
            bytes.as_mut_ptr().cast::<Self>(),
            bytes.len() / std::mem::size_of::<Self>(),
        )
    }
}

pub trait IntoHash {
    fn into_hash(self) -> Hash40;
}

impl IntoHash for &str {
    fn into_hash(self) -> Hash40 {
        Hash40::new(self)
    }
}

impl IntoHash for String {
    fn into_hash(self) -> Hash40 {
        Hash40::new(self.as_str())
    }
}

impl IntoHash for Hash40 {
    fn into_hash(self) -> Hash40 {
        self
    }
}

impl IntoHash for u64 {
    fn into_hash(self) -> Hash40 {
        Hash40(self)
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Locale {
    Japanese = 0,
    UsEnglish,
    UsFrench,
    UsSpanish,
    EuEnglish,
    EuFrench,
    EuSpanish,
    German,
    Dutch,
    Italian,
    Russian,
    Korean,
    Chinese,
    Taiwanese,

    Invalid = -1,
}

impl Locale {
    pub const COUNT: usize = 14;
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Region {
    Japan = 0,
    NorthAmerica,
    Europe,
    Korea,
    China,

    Invalid = -1,
}

impl Region {
    pub const COUNT: usize = 5;
}
