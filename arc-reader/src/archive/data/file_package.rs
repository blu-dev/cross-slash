use std::ops::Range;

use hash40::Hash40;

use crate::{
    archive::{containers::TableRef, resource::serialization::SerState},
    hash::{Hash, HashWithData},
    index::{checked_range, INVALID_INDEX},
    BinaryRepr, Locale, Region,
};

use super::{
    file_group::{FileGroup, FileInfoGroupRef},
    file_info::FileInfo,
};

bitflags::bitflags! {
    /// Flags containing information about how to interpret and/or load a package
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct FilePackageFlags : u32 {
        /// Indicates that this package points to [`LOCALE_COUNT + 1`](crate::Locale)
        /// consecutive [`FileGroups`](super::file_group::FileGroup). One for each locale and an invalid one
        ///
        /// THis flag is mutually exclusive with [`Self::IS_REGIONAL`]
        const IS_LOCALIZED = 1 << 24;

        /// Indicates that this package points to [`REGION_COUNT + 1`](crate::Region)
        /// consecutive [`FileGroups`](super::file_group::FileGroup). One for each locale and an invalid one
        ///
        /// THis flag is mutually exclusive with [`Self::IS_LOCALIZED`]
        const IS_REGIONAL = 1 << 25;

        /// Indicates that this package points to a "subpackage". The index of the "subpackage" is the `redirect`
        /// field of the [`FileGroup`](super::file_group::FileGroup). The "subpackage" is another package
        /// if this package has the [`Self::IS_SYM_LINK`] flag set, otherwise it points to a [`FileInfo`](super::file_info::FileInfo)
        /// bearing file group
        const HAS_SUB_PACKAGE = 1 << 26;

        /// The symlink/subpackage that for this package is regional. This is used to help the resource loader optimize it's load requests
        /// and handling for packages ahead of time.
        ///
        /// This flag requires that [`Self::HAS_SUB_PACKAGE`] and [`Self::IS_SYM_LINK`] are set.
        const SYM_LINK_IS_REGIONAL = 1 << 27;

        /// The subpackage for this package is another package. That package's contents will be loaded instead of this one's.
        ///
        /// Requires that [`Self::HAS_SUB_PACKAGE`] is set.
        const IS_SYM_LINK = 1 << 28;
    }
}

/// A collection of files to load in bulk
///
/// For most aspects of the game, file packages will line up very closely to the traditional filesystem concept,
/// however there are times when they diverge from this format since they are designed for efficient loading
/// of bulk data. This most notably happens for fighters.
///
/// For example, loading all of the required resources for Mario's `c03` costume requires loading data from:
/// - the `sound/` folders
/// - `fighter/mario/model/*/c03`
/// - `fighter/mario/motion/*/c03`
/// - `fighter/mario/param/`
/// - a few other miscellaneous "folders"
///
/// In order to do this, the package to load Mario's `c03` costume is labeled `fighter/mario/c03` and will load
/// *all* of the required data for that costume of Mario to run in-engine.
///
/// Packages can be more complex though. Just like [`FileInfo`](super::file_info::FileInfo), packages can be both
/// regional and localized. Packages should **not** be thought of as a "directory" analog to file info, however,
/// as the analogy breaks down very quickly.
///
/// Packages can also take forms similar to symlinks/shortcuts in traditional filesystem models. For example,
/// when needing to load camera animations for the victory screen, Mario's `fighter/mario/c03` might
/// have a child `fighter/mario/c03/camera`, which would be a symlink to `fighter/mario/cmn/camera`. This
/// "symlink form" can also be regional/localized, although that is much more rare.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FilePackage {
    /// Full path hash and [`FileGroup`](super::file_group::FileGroup) index. The group that this index
    /// points to contains [`FileData`](super::file_data::FileData)
    path_and_group: HashWithData,

    /// The "file name" equivalent of this package (e.g. the `c03` in `fighter/mario/c03`)
    name: Hash,

    /// The hash of the parent of this package. This does not necessarily point ot another package,
    /// it is just the name of the parent in the traditional filesystem sense
    parent: Hash,

    /// This hash doesn't really get read, and might be a leftover from a previous time. This hash can either be:
    /// - Nothing
    /// - `disposable`
    /// - `resident`
    ///
    /// As far as I am aware, none of these are checked in the resource manager, the only packages to make use of these
    /// are loaded on startup
    lifetime: Hash,

    /// Starting index into the [`FileInfo`](super::file_info::FileInfo) table
    info_start: u32,

    /// Number of file infos that this package points to
    info_count: u32,

    /// Starting index into the [`FilePackageChild`] table
    child_start: u32,

    /// Number of package children that this package points to
    child_count: u32,

    /// Flags describing state about this package and how to load its contents
    flags: FilePackageFlags,
}

impl BinaryRepr for FilePackage {}

/// Transparent represnetation of a [`HashWithData`]
///
/// The hash part of this value is the full path of the package, and the data
/// of this value is an index into the [`FilePackage`] array.
///
/// It is considered a logical error if the index is self-recursive and a package points to itself,
/// as this will cause infinite recursion and loading in the resource systems.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FilePackageChild(HashWithData);

impl BinaryRepr for FilePackageChild {}

impl FilePackageChild {
    pub(crate) fn inner(&self) -> &HashWithData {
        &self.0
    }

    pub(crate) fn inner_mut(&mut self) -> &mut HashWithData {
        &mut self.0
    }
}

impl FilePackageChild {
    pub(crate) fn reinternalize(&mut self, state: &SerState) {
        self.0.set_data(state.get::<FilePackage>(self.0.data()))
    }
}

impl FilePackage {
    pub(crate) fn info_range(&self) -> Range<u32> {
        checked_range(self.info_start, self.info_count)
    }

    pub(crate) fn child_package_range(&self) -> Range<u32> {
        checked_range(self.child_start, self.child_count)
    }

    pub(crate) fn data_group_range(&self) -> Range<u32> {
        let start = self.path_and_group.data();
        let count = if self.flags.intersects(FilePackageFlags::IS_LOCALIZED) {
            Locale::COUNT as u32 + 1
        } else if self.flags.intersects(FilePackageFlags::IS_REGIONAL) {
            Region::COUNT as u32 + 1
        } else {
            1
        };

        checked_range(start, count)
    }

    pub(crate) fn set_info_start(&mut self, index: u32) {
        self.info_start = index;
    }

    pub(crate) fn set_child_start(&mut self, index: u32) {
        self.child_start = index;
    }

    pub(crate) fn set_data_group_start(&mut self, index: u32) {
        self.path_and_group.set_data(u32::from(index));
    }

    pub fn path(&self) -> Hash40 {
        self.path_and_group.hash40()
    }
}

pub enum SubPackageRef<'a> {
    FileGroup(FileInfoGroupRef<'a>),
    SymLink(TableRef<'a, FilePackage>),
}

impl TableRef<'_, FilePackage> {
    pub fn get_sym_link(&self) -> Option<TableRef<'_, FilePackage>> {
        if !self
            .flags
            .contains(FilePackageFlags::HAS_SUB_PACKAGE | FilePackageFlags::IS_SYM_LINK)
        {
            return None;
        }

        let archive = self.archive();
        let group = archive
            .get_file_group(self.path_and_group.data())
            .expect("file group should exist");

        if group.redirection == INVALID_INDEX {
            panic!("data group on sym link must refer to file package");
        }

        let sym_link = archive
            .get_file_package(group.redirection)
            .expect("file package for sym link should exist");

        Some(sym_link)
    }

    pub fn data_group(&self) -> TableRef<'_, FileGroup> {
        self.archive()
            .get_file_group(self.path_and_group.data())
            .expect("file group should exist")
    }

    pub fn sub_package(&self) -> Option<SubPackageRef<'_>> {
        if !self.flags.contains(FilePackageFlags::HAS_SUB_PACKAGE) {
            return None;
        }

        let redirection = self.data_group().redirection;

        if self.flags.contains(FilePackageFlags::IS_SYM_LINK) {
            Some(SubPackageRef::SymLink(
                self.archive()
                    .get_file_package(u32::from(redirection))
                    .expect("sym link should exist"),
            ))
        } else if redirection != INVALID_INDEX {
            Some(SubPackageRef::FileGroup(FileInfoGroupRef(
                self.archive()
                    .get_file_group(redirection)
                    .expect("file group should exist"),
            )))
        } else {
            None
        }
    }
}

impl FilePackage {
    pub(crate) fn reserve(&self, state: &mut SerState) {
        state.reserve_range::<FilePackageChild>(self.child_start, self.child_count);
        state.reserve_range::<FileInfo>(self.info_start, self.info_count);
        state.reserve_range::<FileGroup>(
            self.path_and_group.data(),
            self.data_group_range().count() as u32,
        );
    }

    pub(crate) fn reinternalize(&mut self, state: &SerState) {
        self.child_start = if self.child_count == 0 {
            INVALID_INDEX
        } else {
            state.get::<FilePackageChild>(self.child_start)
        };

        self.info_start = if self.info_count == 0 {
            INVALID_INDEX
        } else {
            state.get::<FileInfo>(self.info_start)
        };

        let index = self.path_and_group.data();
        self.path_and_group
            .set_data(u32::from(state.get::<FileGroup>(index)));
    }
}
