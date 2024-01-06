use crate::{archive::resource::serialization::SerState, BinaryRepr};

use super::stream_data::StreamData;

/// Simple index redirection used by [`StreamPath`](super::stream_path::StreamPath) to locate
/// the streamable file data.
///
/// There can be multiple of these for each path, depending on if the path is regional or localized.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StreamDesc {
    /// Index into the [`StreamData`](super::stream_data::StreamData) table for this file's
    /// streamable file data
    stream_data: u32,
}

impl BinaryRepr for StreamDesc {}

impl StreamDesc {
    pub(crate) fn stream_data_index(&self) -> u32 {
        self.stream_data
    }

    pub(crate) fn set_stream_data_index(&mut self, index: u32) {
        self.stream_data = index;
    }
}

impl StreamDesc {
    pub(crate) fn reserve(&self, state: &mut SerState) {
        state.try_reserve::<StreamData>(self.stream_data);
    }

    pub(crate) fn reinternalize(&mut self, state: &SerState) {
        self.stream_data = state.get::<StreamData>(self.stream_data);
    }
}
