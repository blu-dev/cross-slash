use crate::{archive::resource::serialization::SerState, index::INVALID_INDEX, BinaryRepr};

use super::{
    file_data::FileData, file_entity::FileEntity, file_group::FileGroup, file_info::FileInfo,
};

/// The load method that is used when loading file data
///
/// This enum is actually a set of bitflags in the resource loading engine,
/// however I've categorized them as an enum for easier understanding
/// and pulled out all of the combinations that exist in the archive
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum LoadMethod {
    /// Loading file data using the descriptor this came from would be considered invalid
    /// and load invalid data in the game. This would either cause a crash or an infinite load
    ///
    /// The index is for a [`FileEntity`](super::file_entity::FileEntity) and points to the file entity
    /// that should be used to load this file instead.
    Unowned(u32),

    /// Loading file data from this descriptor is considered valid, and should load data.
    ///
    /// The index is for a version patch section from the versioned fs. This index indicates
    /// which version this data is for
    Owned(u32),

    /// This file should be skipped when loading this directory as a package. This is because the file is shared
    /// across package and group boundaries, and it should either already be loaded, or will be loaded shortly,
    /// from the file group
    PackageSkip(u32),

    /// I'm not sure what this load method entails. This is only present on like, 2 or 3 files
    Unknown,

    /// This load method is **exclusively** used by pokemon trainer's effect file, IIRC.
    ///
    /// The checking of this flag is also so absurd that I can only imagine it is a bug, the game checks it like this:
    /// ```cpp
    /// // C++
    /// archive->file_descs[archive->file_infos[file_info_index].file_entity_index].flags & 0x8 != 0
    /// ```
    SharedButOwned(u32),

    /// Indicates that this file descriptor is a regional/localized descriptor for a file, however this region/locale
    /// does not have any specialized data. The argument is a region/locale to use instead.
    ///
    /// For example, Lyn's voicelines are available in both English and Japanese, but no other locales. The other localized
    /// descriptors will point to the English and Japanese locales depending on which language they are for.
    UnsupprotedRegionLocale(u32),
}

impl From<LoadMethod> for FileLoadMethod {
    fn from(value: LoadMethod) -> Self {
        match value {
            LoadMethod::Unowned(index) => Self(u32::from(index)),
            LoadMethod::Owned(index) => Self((0x01 << 24) | u32::from(index)),
            LoadMethod::PackageSkip(index) => Self((0x03 << 24) | u32::from(index)),
            LoadMethod::Unknown => Self(0x05 << 24),
            LoadMethod::SharedButOwned(index) => Self((0x09 << 24) | u32::from(index)),
            LoadMethod::UnsupprotedRegionLocale(region_locale) => {
                Self((0x10 << 24) | region_locale)
            }
        }
    }
}

impl From<FileLoadMethod> for LoadMethod {
    fn from(value: FileLoadMethod) -> Self {
        let kind = value.0 >> 24;
        match kind {
            0x00 => Self::Unowned(value.0 & 0x00FF_FFFF),
            0x01 => Self::Owned(value.0 & 0x00FF_FFFF),
            0x03 => Self::PackageSkip(value.0 & 0x00FF_FFFF),
            0x05 => Self::Unknown,
            0x09 => Self::SharedButOwned(value.0 & 0x00FF_FFFF),
            0x10 => Self::UnsupprotedRegionLocale(value.0 & 0x00FF_FFFF),
            _ => panic!("Unsupported load method {:#02x}", kind),
        }
    }
}

/// Transparent representation of [`LoadMethod`]
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct FileLoadMethod(u32);

/// Represents information about how to load and locate binary file data
///
/// File descriptors represent the last "turn back" location in the resource system where you can detect that a
/// file that you are about to load could be incorrect or be pointing to invalid data.
///
/// Once you move past the file descriptor, and into the [`FileData`](super::file_data::FileData), you have no information
/// about the file except for how to interpret the bytes on disk
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FileDesc {
    /// The file group that contains this file descriptor. This is used to instruct the resource loader where in the archive
    /// the start of the file data chunk is
    group: u32,

    /// Points to a [`FileData`](super::file_data::FileData) that informs the resource loader how much and how to read
    /// the file data
    file_data: u32,

    /// Information about how to load the data, including whether or not it should be loaded as part of a package
    /// or if it should be loaded at all/points to invalid data
    load_method: FileLoadMethod,
}

impl BinaryRepr for FileDesc {}

impl FileDesc {
    pub(crate) fn file_data_index(&self) -> u32 {
        self.file_data
    }
}

impl FileDesc {
    pub(crate) fn reserve(&self, state: &mut SerState) {
        state.reserve::<FileData>(self.file_data);
    }

    pub(crate) fn reinternalize(&mut self, state: &SerState) {
        self.group = state.get::<FileGroup>(self.group);
        self.file_data = state.get::<FileData>(self.file_data);

        let mut load_method = LoadMethod::from(self.load_method);

        match &mut load_method {
            LoadMethod::Unowned(index) => *index = state.get::<FileEntity>(*index),

            // This points to versioned data, we are eliminating that here
            LoadMethod::Owned(index) => *index = INVALID_INDEX,
            LoadMethod::PackageSkip(index) => *index = state.get::<FileInfo>(*index),
            LoadMethod::Unknown => {}
            LoadMethod::SharedButOwned(index) => *index = state.get::<FileEntity>(*index),
            LoadMethod::UnsupprotedRegionLocale(_) => {}
        }

        self.load_method = FileLoadMethod::from(load_method);
    }
}
