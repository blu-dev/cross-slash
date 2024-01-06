use crate::BinaryRepr;

bitflags::bitflags! {
    /// Flags that control loading behavior and version information for a file
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct FileFlags : u32 {
        /// This file data is compressed using ZSTD. This flag is **always** set for files in the production
        /// release of Smash Ultimate. If this flag is set, then [`Self::IS_COMPRESSED`] must also be set
        /// or else the resource loaders will abort.
        const IS_ZSTD_COMPRESSION = 1 << 0;

        /// Indicates whether this file is compressed. Uses ZSTD compression if [`Self::IS_ZSTD_COMPRESSION`] is set,
        /// or a seemingly proprietary compression otherwise.
        const IS_COMPRESSED = 1 << 1;

        /// This file data is for a versioned, regional file. Regional files are **not** localized files,
        /// there are only a few of them in the game, and this flag is notably never been set and is only assumed
        /// to be the case.
        ///
        /// This flag is also never read in the resource loaders.
        const IS_REGIONAL_VERSIONED_DATA = 1 << 2;

        /// This file data is for a versioned, localized file. This flag is set in the 13.0.1 version of the
        /// data.arc for Shulk's monado arts.
        ///
        /// THis flag is never read in the resource loaders.
        const IS_LOCALIZED_VERSIONED_DATA = 1 << 3;
    }
}

/// Contains information on how to read the data on disk
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FileData {
    /// The offset to add to the `archive_offset` of the [`FileGroup`](super::file_group::FileGroup) that contains this data
    in_group_offset: u32,

    /// The size of the compressed data to read. If [`FileFlags::IS_COMPRESSED`] is not set, then this will be the same
    /// as `decompressed_size`
    compressed_size: u32,

    /// The size of the data once decompressed. This is used to allocate a large enough buffer, since IO swaps happen
    /// between threads this buffer needs to be preallocated.
    decompressed_size: u32,

    /// Flags that describe this file data
    flags: FileFlags,
}

impl BinaryRepr for FileData {}
