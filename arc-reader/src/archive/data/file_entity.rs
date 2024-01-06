use crate::{archive::resource::serialization::SerState, BinaryRepr};

use super::{file_group::FileGroup, file_info::FileInfo, file_package::FilePackage};

/// Represents a unique file entity
///
/// These file entities are used internally to track whether or not a file has been loaded.
/// Shared file data exists by having multiple [`FilePath`](super::file_path::FilePath)
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FileEntity {
    /// File entities can belong to either a [`FilePackage`](super::file_package::FilePackage)
    /// or a [`FileGroup`](super::file_group::FileGroup).
    ///
    /// This information is easy to tell: if this index is >= the index of the first file group of shared data,
    /// then this is owned by a group. If it is not, then it is owned by a package.
    package_or_group: u32,

    /// This index points to the file info that represents the *real* data of this entity.
    ///
    /// If this entity is owned by a file group, then this index will point to a [`FileInfo`](super::file_info::FileInfo)
    /// within that group. If it is onwed by a file package, then this index will point fo a `FileInfo` inside of that package.
    ///
    /// This should *never* point to a `FileInfo` that is not the "source of truth" for the data.
    info: u32,
}

impl BinaryRepr for FileEntity {}

impl FileEntity {
    /// Reinternalizes a file entity
    ///
    /// Everything that a file entity references should be reserved with the [`SerState`]
    /// before calling this method ([`FilePackage`], [`FileGroup`], [`FileInfo`])
    pub(crate) fn reinternalize(&mut self, state: &SerState, package_len: u32) {
        if self.package_or_group >= package_len {
            self.package_or_group = state.get::<FileGroup>(self.package_or_group);
        } else {
            self.package_or_group = state.get::<FilePackage>(self.package_or_group);
        }

        self.info = state.get::<FileInfo>(self.info);
    }
}
