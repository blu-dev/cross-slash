use crate::BinaryRepr;

/// Simple informational data structure that informs the resource service of the location and size
/// of a streamable data file
///
/// There is no other information associated with this file because there doesn't need to be. Streamable
/// file contents are provided as file paths and offsets into the resource streaming utilities provided
/// by the game. The data **must** be uncompressed and it is not loaded by the game
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StreamData {
    /// The size of the file, in bytes
    size: u64,

    /// The offset of the first byte of file data in the archive
    offset: u64,
}

impl BinaryRepr for StreamData {}
