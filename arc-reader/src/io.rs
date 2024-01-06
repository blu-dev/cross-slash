use std::io::{self, Read, Seek, Write};
use std::ops::Range;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::BinaryRepr;

/// Extension trait to allow easier reading of archive file data
pub(crate) trait ReadBinExt {
    /// Reads the exact number of bytes to read a value of `T`
    ///
    /// SAFETY: The caller must ensure that reading this data at the current location
    ///     will result in a proper/valid value of `T`
    unsafe fn read_binary<T: BinaryRepr>(&mut self) -> Result<T, io::Error> {
        let bytes = self.read_count_vec(std::mem::size_of::<T>())?;
        crate::single_value_sanity::<T>(&bytes);
        // SAFETY: We have confirmed via sanity checks that this data is proper
        Ok(std::ptr::read(bytes.as_ptr().cast::<T>()))
    }

    /// Reads an exact number of bytes, returning it as a boxed slice of bytes
    fn read_count(&mut self, count: usize) -> Result<Box<[u8]>, io::Error> {
        self.read_count_vec(count).map(Vec::into_boxed_slice)
    }

    /// Reads an exact number of bytes, returning it as a vec of bytes
    fn read_count_vec(&mut self, count: usize) -> Result<Vec<u8>, io::Error>;

    /// Reads a compressed data section, returning it as a decompressed
    /// slice of bytes
    fn read_compressed_data(&mut self) -> Result<Box<[u8]>, io::Error> {
        self.read_compressed_data_vec().map(Vec::into_boxed_slice)
    }

    /// Reads a compressed data section, returning it as a decompressed
    /// vec of bytes
    fn read_compressed_data_vec(&mut self) -> Result<Vec<u8>, io::Error>;
}

pub(crate) trait WriteBinExt: Write {
    fn write_binary<T: BinaryRepr + Copy>(&mut self, value: &T) -> Result<(), io::Error> {
        self.write_all(value.cast_bytes())
    }
}

impl<W: Write> WriteBinExt for W {}

impl<R: Read + Seek> ReadBinExt for R {
    fn read_count_vec(&mut self, count: usize) -> Result<Vec<u8>, io::Error> {
        // SAFETY: We are initializing a vec with invalid data, but then immediately
        //      reading into it/initializing. So long as the reader provided implements
        //      the `read_exact` contract properly, then this is safe.
        unsafe {
            let mut data = Vec::with_capacity(count);
            data.set_len(count);
            self.read_exact(&mut data)?;

            Ok(data)
        }
    }

    fn read_compressed_data_vec(&mut self) -> Result<Vec<u8>, io::Error> {
        const REQUIRED_TABLE_SIZE: u32 = 0x10;

        let starting_position = self.stream_position()?;

        let table_size = self.read_u32::<LittleEndian>()?;
        if table_size != REQUIRED_TABLE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected table size of {REQUIRED_TABLE_SIZE:#x}, found {table_size:#x}"),
            ));
        }

        let decompressed_size = self.read_u32::<LittleEndian>()? as usize;
        let compressed_size = self.read_u32::<LittleEndian>()? as u64;
        let offset_to_next = self.read_u32::<LittleEndian>()? as u64;

        // SAFETY: We are initializing a vec with valid data by reading it in after creating the buffer
        let mut data = Vec::with_capacity(decompressed_size);

        #[cfg(not(target_os = "switch"))]
        {
            zstd::stream::copy_decode(self.take(compressed_size), &mut data)?;
        }

        #[cfg(target_os = "switch")]
        {
            #[repr(C)]
            struct ZSTD_PtrBuffer {
                pub ptr: *mut u8,
                pub size: usize,
                pub pos: usize,
            }

            #[skyline::from_offset(0x39a2fc0)]
            fn decompress_stream(
                thing: *mut u64,
                output: &mut ZSTD_PtrBuffer,
                input: &mut ZSTD_PtrBuffer,
            ) -> usize;

            #[skyline::from_offset(0x35410b0)]
            fn initialize_decompressor(ptr: *mut u64);

            #[skyline::from_offset(0x3541030)]
            fn finalize_decompressor(ptr: *mut u64);

            let mut buf = Vec::with_capacity(compressed_size as usize);
            unsafe {
                buf.set_len(compressed_size as usize);
            }

            self.read_exact(&mut buf)?;

            let mut decompressor = [0u64; 2];
            unsafe {
                initialize_decompressor(decompressor.as_mut_ptr());
                data.set_len(decompressed_size);
            }

            let mut input_buffer = ZSTD_PtrBuffer {
                ptr: buf.as_ptr() as _,
                size: buf.len(),
                pos: 0,
            };

            let mut output_buffer = ZSTD_PtrBuffer {
                ptr: data.as_mut_ptr(),
                size: data.len(),
                pos: 0,
            };

            unsafe {
                decompress_stream(decompressor[1] as _, &mut output_buffer, &mut input_buffer);
                finalize_decompressor(decompressor.as_mut_ptr());
            }
        }

        if data.len() != decompressed_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Expected a decompressed size of {decompressed_size:#x}, received {:#x}",
                    data.len()
                ),
            ));
        }

        self.seek(io::SeekFrom::Start(starting_position + offset_to_next))?;

        Ok(data)
    }
}

/// Simple reader that reads over a slice of borrowed bytes,
/// allowing the caller to cast the data into other forms when needed
pub(crate) struct BorrowedReader<'a> {
    data: &'a [u8],
    cursor: usize,
}

#[allow(dead_code)]
impl<'a> BorrowedReader<'a> {
    /// Creates a new borrowed reader, with a cursor of `0`
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, cursor: 0 }
    }

    /// SAFETY: The caller guarantees that where we are reading is valid
    ///     and initialized contents for a value of T
    pub unsafe fn read_copy<T: BinaryRepr + Copy>(&mut self) -> T {
        let data = T::cast(&self.data[self.cursor..]);
        self.cursor += std::mem::size_of::<T>();
        *data
    }

    /// SAFETY: The caller guarantees that where we are reading is valid
    ///     and initialized contents for a value of T
    pub unsafe fn read<T: BinaryRepr>(&mut self) -> &T {
        let data = T::cast(&self.data[self.cursor..]);
        self.cursor += std::mem::size_of::<T>();
        data
    }

    /// SAFETY: The caller guarantees that where we are reading is valid
    ///     and initialized contents for `count` values of T
    pub unsafe fn read_slice<T: BinaryRepr>(&mut self, count: usize) -> &[T] {
        let data = T::cast_slice(
            &self.data[self.cursor..(self.cursor + std::mem::size_of::<T>() * count)],
        );
        self.cursor += std::mem::size_of::<T>() * count;
        data
    }

    /// Advances the cursor by an equivalent size of `count * std::mem::size_of::<T>()` while
    /// performing sanity checks on the byte range itself
    ///
    /// This is useful if you don't need the data right now, but need to know where it is
    /// located for future operations
    ///
    /// The return value of this function is guaranteed to be within range of the data
    /// provided when constructing this reader, so if you do not mutate the
    /// data container in any way it is safe to call `get_unchecked` with the range
    pub fn advance_byte_slice<T: Sized>(&mut self, count: usize) -> Range<usize> {
        let range = self.cursor..self.cursor + count * std::mem::size_of::<T>();

        // Check to make sure that this byte slice is appropriate for how we
        // are going to use it
        crate::slice_sanity::<T>(&self.data[range.clone()]);

        self.cursor += count;
        range
    }
}
