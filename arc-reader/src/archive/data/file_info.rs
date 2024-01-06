use std::ops::Range;

use crate::{
    archive::resource::serialization::SerState, index::checked_range, BinaryRepr, Locale, Region,
};

use super::{file_desc::FileDesc, file_entity::FileEntity, file_path::FilePath};

bitflags::bitflags! {
    /// Flags that help loaders determine special behavior to apply to files
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct FileInfoFlags : u32 {
        /// This flag is mutually exclusive from [`Self::IS_GRAPHICS_ARCHIVE`]
        ///
        /// If this file is not a graphics archive, then this flag will be set.
        const IS_REGULAR_FILE = 1 << 4;

        /// This flag is mutually exclusive from [`Self::IS_REGULAR_FILE`]
        ///
        /// If this file is a graphics archive, then this flag will be set. I assume that data with this flag
        /// set are automatically loaded into an associated graphics buffer
        const IS_GRAPHICS_ARCHIVE = 1 << 12;

        /// Indicates that this file will point to [`LOCALE_COUNT + 1`](crate::Locale) file descriptors,
        /// one for each locale and an invalid one
        const IS_LOCALIZED = 1 << 15;

        /// Indicates that this file will point to [`REGION_COUNT + 1`](crate::Region) file descriptors,
        /// one for each locale and an invlaid one
        const IS_REGIONAL = 1 << 16;

        /// Indicates that this file is shared between packages exclusively, not between packages and groups.
        const IS_SHARED = 1 << 20;

        /// This flag usually comes with [`Self::IS_SHARED`] but I've never seen it checked in game
        const IS_UNKNOWN_FLAG = 1 << 21;
    }
}

/// Represents file information for a file
///
/// Unlike the [`FileEntity`](super::file_entity::FileEntity) which is 1-1 with every
/// actual piece of file binary data in the archive, and the [`FilePath`](super::file_path::FilePath) which is 1-1
/// with every file path in the game, there can be multiple of these that represent a single file.
///
/// For example, for files whose data is owned by file groups, instead of file packages, there will be two file infos
/// that exist with the same `path` and `entity` indexes.
///
/// For filepaths that use shared data and are not the "source of truth" for the file, there will only be one of these
/// and it is owned by the [`FilePackage`](super::file_package::FilePackage) that is used to load this file
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FileInfo {
    /// Points to the [`FilePath`](super::file_path::FilePath) that this info represents
    ///
    /// The file path this points to does not necessarily point back to this info.
    path: u32,

    /// Points to the [`FileEntity`](super::file_entity::FileEntity) that points to the
    /// source of truth for this file
    ///
    /// The file entity this points to does not necessarily point back to this info.
    entity: u32,

    /// Points to the [`FileDesc`](super::file_desc::FileDesc) for this info. If this is a shared file,
    /// and this info does not point to the source of truth, loading from that file descriptor is considered
    /// an invalid operation
    desc: u32,

    /// Flags for this info
    flags: FileInfoFlags,
}

impl BinaryRepr for FileInfo {}

impl FileInfo {
    pub(crate) fn descriptor_range(&self) -> Range<u32> {
        let count = if self.flags.intersects(FileInfoFlags::IS_LOCALIZED) {
            Locale::COUNT as u32 + 1
        } else if self.flags.intersects(FileInfoFlags::IS_REGIONAL) {
            Region::COUNT as u32 + 1
        } else {
            1
        };

        checked_range(self.desc, count)
    }
}

impl FileInfo {
    pub(crate) fn reserve(&self, state: &mut SerState) {
        state.reserve_range::<FileDesc>(self.desc, self.descriptor_range().count() as u32);
    }

    pub(crate) fn reinternalize(&mut self, state: &SerState) {
        self.path = state.get::<FilePath>(self.path);
        self.entity = state.get::<FileEntity>(self.entity);
        self.desc = state.get::<FileDesc>(self.desc);
    }
}
