use hash40::Hash40;

use crate::{
    archive::resource::serialization::SerState,
    hash::{Hash, HashWithData},
    index::INVALID_INDEX,
    BinaryRepr,
};

use super::file_entity::FileEntity;

/// Represents a single file in the archive
///
/// Every non-stream asset file has a single file path that represents it, which means that there is a 1-1 relationship
/// to every `FilePath` in the archive with every conceptual "file path" (i.e. `fighter/mario/model/body/c00/model.nuanmb`,
/// at least for files in the current version.
///
/// The archive supports a version history in its files, which means that for some files you are able to access and load
/// the old file data from previous versions. They reset this version information in the archive over time, so some times
/// the archive would actually shrink in size.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FilePath {
    /// The path of this file, and the [`FileEntity`](super::file_entity::FileEntity) that points to the "source of truth" for this
    /// file's data
    path_and_entity: HashWithData,

    /// The extension of this file, an index into the versioned file tables for the previous version of this file
    ext_and_version: HashWithData,

    /// The hash of the parent "folder", note that this is not the hash of the [`FilePackage`](super::file_package::FilePackage)
    /// which contains this file, but rather this refers to the traditional filesystem concept of a parent folder
    parent: Hash,

    /// The name of this file, including the extension
    file_name: Hash,
}

impl FilePath {
    pub fn path(&self) -> Hash40 {
        self.path_and_entity.hash40()
    }

    pub(crate) fn file_entity_index(&self) -> u32 {
        self.path_and_entity.data()
    }

    pub(crate) fn set_file_entity_index(&mut self, index: u32) {
        self.path_and_entity.set_data(index);
    }
}

impl BinaryRepr for FilePath {}

impl FilePath {
    pub(crate) fn reinternalize(&mut self, state: &SerState) {
        let index = self.path_and_entity.data();
        let index = state.get::<FileEntity>(index);
        self.path_and_entity.set_data(u32::from(index));

        self.ext_and_version.set_data(INVALID_INDEX);
    }
}
