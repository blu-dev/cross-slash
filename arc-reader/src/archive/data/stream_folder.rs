use std::ops::Range;

use crate::{
    archive::resource::serialization::SerState, hash::HashWithData, index::checked_range,
    BinaryRepr,
};

use super::stream_path::StreamPath;

/// Represents a collection of stream file paths, all prefixed by the same folder name
///
/// Unlike [`FilePackage`](super::file_package::FilePackage), this type is very simple. It is
/// just a named slice of [`StreamPath`](super::stream_path::StreamPath).
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StreamFolder {
    /// The hash part of this value is the name of the folder, without the `stream:`
    /// prefix, and the data part is the number of [`StreamPath`](super::stream_path::StreamPath)
    /// this folder owns
    name_and_child_count: HashWithData,

    /// Index into the table of [`StreamPath`](super::stream_path::StreamPath) for where the children start
    child_start_index: u32,
}

impl BinaryRepr for StreamFolder {}

impl StreamFolder {
    pub(crate) fn stream_path_range(&self) -> Range<u32> {
        checked_range(self.child_start_index, self.name_and_child_count.data())
    }

    pub(crate) fn set_stream_path_start(&mut self, index: u32) {
        self.child_start_index = index;
    }
}

impl StreamFolder {
    pub(crate) fn reserve(&self, state: &mut SerState) {
        state.reserve_range::<StreamPath>(self.child_start_index, self.name_and_child_count.data());
    }

    pub(crate) fn reinternalize(&mut self, state: &SerState) {
        self.child_start_index = state.get::<StreamPath>(self.child_start_index);
    }
}
