use std::ops::Range;

use hash40::Hash40;

use crate::{
    archive::resource::serialization::SerState, hash::HashWithData, index::checked_range,
    BinaryRepr, Locale, Region,
};

use super::stream_desc::StreamDesc;

bitflags::bitflags! {
    /// Regional flags for stream file data
    #[repr(transparent)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct StreamFileFlags : u32 {
        /// This flag is mutually exclusive from [`Self::IS_REGIONAL`], and indicates that
        /// this [`StreamPath`] points to [`LOCALE_COUNT`](crate::Locale) [`StreamEntity`](super::stream_entity::StreamEntity),
        /// one for each locale
        const IS_LOCALIZED = 1 << 0;

        /// This flag is mutually exclusive from [`Self::IS_LOCALIZED`], and indicates that
        /// this [`StreamPath`] points to [`REGION_COUNT`](crate::Region) [`StreamEntity`](super::stream_entity::StreamEntity),
        /// one for each region
        const IS_REGIONAL = 1 << 1;
    }
}

/// Representation of a stream file
///
/// This value is 1-1 with every streamable file in the archive, meaning that if there is a path
/// that can be used to identify the file, it will be present here.
///
/// For files that are localized/regional, there is still only one of these paths however this path will point
/// to multiple [`StreamDesc`](super::stream_desc::StreamDesc), depending on whether it is regional
/// or localized (see [`StreamFileFlags`] for more info).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StreamPath {
    /// The path of this file, including the `stream:` prefix, and the start index of the [`StreamDesc`](super::stream_desc::StreamDesc)
    /// that this points to.
    path_and_desc: HashWithData,

    /// Region/localization flags that instruct the resource service how to load/stream this file
    flags: StreamFileFlags,
}

impl StreamPath {
    pub fn path(&self) -> Hash40 {
        self.path_and_desc.hash40()
    }

    pub(crate) fn descriptor_range(&self) -> Range<u32> {
        let count = if self.flags.contains(StreamFileFlags::IS_LOCALIZED) {
            Locale::COUNT as u32
        } else if self.flags.contains(StreamFileFlags::IS_REGIONAL) {
            Region::COUNT as u32
        } else {
            1
        };

        checked_range(self.path_and_desc.data(), count)
    }

    pub(crate) fn set_descriptor_start(&mut self, index: u32) {
        self.path_and_desc.set_data(index);
    }
}

impl BinaryRepr for StreamPath {}

impl StreamPath {
    pub(crate) fn reserve(&self, state: &mut SerState) {
        state.reserve_range::<StreamDesc>(
            self.path_and_desc.data(),
            self.descriptor_range().count() as u32,
        );
    }

    pub(crate) fn reinternalize(&mut self, state: &SerState) {
        let index = self.path_and_desc.data();
        self.path_and_desc
            .set_data(u32::from(state.get::<StreamDesc>(index)));
    }
}
