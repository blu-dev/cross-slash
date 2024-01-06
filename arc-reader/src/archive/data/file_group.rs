use std::ops::Range;

use crate::{
    archive::{
        containers::{TableRef, TableSliceRef},
        resource::serialization::SerState,
    },
    index::{checked_range, INVALID_INDEX},
    BinaryRepr,
};

use super::{file_data::FileData, file_info::FileInfo, file_package::FilePackage};

/// Represents a collection of either [`FileInfo`](super::file_info::FileInfo) or [`FileData`](super::file_data::FileData)
///
/// The purpose of these groups is to store chunks of data to optimize file reading when loading as part of a group.
/// For example, a notable place where the game *could* but does *not* use this kind of optimization is when loading
/// CSPs, all of them are loaded individually one file at a time.
///
/// These groups will refer to `FileInfos` in some situations:
/// - When they are referring to versioned data groups
/// - When they are referring to data shared across package/group boundaries
///
/// When they are referring to file info instead of file data, they still point to valid data chunks but the way
/// the resource loaders interpret the contents of these files changes: instead of using the [`FileDesc`](super::file_desc::FileDesc)
/// to determine which group to load the data from, it uses the group itself for more optimal load times.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FileGroup {
    /// The starting offset of this data chunk group
    ///
    /// This is aligned to 0x4 boundary, so we do this to split it
    pub(crate) archive_offset: [u32; 2],

    /// The size of all of this group's contents when decompressed. This is used when performing group loading
    /// in order to decompress the entire group's worth of content in one thread while it is being inflated
    /// in another thread.
    pub(crate) decompressed_size: u32,

    /// The size of all of this group's contents when compressed. This is used to determine
    /// how much data is remaining to decompress when loading as a group.
    pub(crate) compressed_size: u32,

    /// The start index of this group's children. Whether this refers to [`FileData`](super::file_data::FileData)
    /// or [`FileInfo`](super::file_info::FileInfo) is contextual
    child_start: u32,

    /// The number of children this group points to
    child_count: u32,

    /// Index used when this group is being driven by an owning [`FilePackage`](super::file_package::FilePackage).
    ///
    /// If this group is pointing to [`FileData`](super::file_data::FileData), then this index will point to either
    /// a group containing [`FileInfo`](super::file_info::FileInfo) or a `FilePackage` depending on context. Or it will be invalid
    /// and point to nothing.
    ///
    /// If this group is pointing to [`FileInfo`](super::file_info::FileInfo), then this index will be recursively
    /// pointing to itself
    pub(crate) redirection: u32,
}

impl BinaryRepr for FileGroup {}

impl FileGroup {
    pub(crate) fn child_range(&self) -> Range<u32> {
        checked_range(self.child_start, self.child_count)
    }

    pub(crate) fn redirection_index(&self) -> u32 {
        self.redirection
    }
}

pub struct FileInfoGroupRef<'a>(pub(super) TableRef<'a, FileGroup>);

impl<'a> std::ops::Deref for FileInfoGroupRef<'a> {
    type Target = TableRef<'a, FileGroup>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FileInfoGroupRef<'_> {
    pub fn file_info(&self) -> TableSliceRef<'_, FileInfo> {
        self.0
            .archive()
            .get_file_info_slice(self.0.child_start, self.0.child_count)
            .expect("file info group should point to valid file info")
    }

    pub(crate) fn file_info_range(&self) -> Range<u32> {
        checked_range(self.0.child_start, self.0.child_count)
    }
}

pub struct FileDataGroupRef<'a>(pub(super) TableRef<'a, FileGroup>);

impl FileDataGroupRef<'_> {
    pub fn file_data(&self) -> TableSliceRef<'_, FileData> {
        self.0
            .archive()
            .get_file_data_slice(self.0.child_start, self.0.child_count)
            .expect("file data group should point to valid file data")
    }

    pub(crate) fn file_data_range(&self) -> Range<u32> {
        checked_range(self.0.child_start, self.0.child_count)
    }
}

impl FileGroup {
    pub(crate) fn reserve(&self, state: &mut SerState, is_data: bool) {
        if is_data {
            for index in self.child_range() {
                state.try_reserve::<FileData>(index);
            }
        } else {
            state.reserve_range::<FileInfo>(self.child_start, self.child_count);
        }
    }

    pub(crate) fn reinternalize_data(&mut self, state: &SerState, package_len: u32) {
        self.child_start = state.get::<FileData>(self.child_start);

        if self.redirection != INVALID_INDEX {
            if self.redirection >= package_len {
                self.redirection = state.get::<FileGroup>(self.redirection);
            } else {
                self.redirection = state.get::<FilePackage>(self.redirection);
            }
        }
    }

    pub(crate) fn reinternalize_info(&mut self, state: &SerState) {
        self.child_start = state.get::<FileInfo>(self.child_start);
        self.redirection = state.get::<FileGroup>(self.redirection);
    }
}
