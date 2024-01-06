use byteorder::{ByteOrder, LittleEndian};

use crate::{io::ReadBinExt, BinaryRepr, IntoHash};
use std::io::{Read, Seek, SeekFrom};

mod data;
pub use data::*;

mod containers;
pub mod resource;

use self::{
    containers::{BucketLookup, IndexLookup, Table, TableMut, TableRef, TableSliceRef},
    file_data::FileData,
    file_desc::FileDesc,
    file_entity::FileEntity,
    file_group::FileGroup,
    file_info::FileInfo,
    file_package::{FilePackage, FilePackageChild},
    file_path::FilePath,
    resource::ResourceTables,
    stream_data::StreamData,
    stream_desc::StreamDesc,
    stream_folder::StreamFolder,
    stream_path::StreamPath,
};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ArchiveMetadata {
    magic: u64,
    stream_data_offset: u64,
    file_data_offset: u64,
    shared_file_data_offset: u64,
    resource_table_offset: u64,
    user_table_offset: u64,
    unknown_table_offset: u64,
}

impl ArchiveMetadata {
    const MAGIC: u64 = 0xABCDEF9876543210;
}

impl BinaryRepr for ArchiveMetadata {}

pub struct Archive {
    metadata: ArchiveMetadata,
    resource: ResourceTables,
}

macro_rules! decl_lookup {
    ($($name:ident => $t:ty),*) => {
        paste::paste! {
            $(
                pub fn [<lookup_ $name>](&self, path: impl IntoHash) -> Option<TableRef<'_, $t>> {
                    let index = self.resource.[<$name _lookup>].get(path.into_hash())?;
                    TableRef::new(self, &self.resource.$name, index)
                }

                pub fn [<lookup_ $name _mut>](&mut self, path: impl IntoHash) -> Option<TableMut<'_, $t>> {
                    let index = self.resource.[<$name _lookup>].get(path.into_hash())?;
                    TableMut::new(self, |archive| &mut archive.resource.$name, index)
                }
            )*
        }
    }
}

macro_rules! decl_access {
    ($($name:ident => $t:ty),*) => {
        paste::paste! {
            $(
                pub fn [<num_ $name>](&self) -> usize {
                    self.resource.$name.len()
                }

                pub(crate) fn [<get_ $name>](&self, index: u32) -> Option<TableRef<'_, $t>> {
                    TableRef::new(self, &self.resource.$name, index)
                }

                pub(crate) fn [<get_ $name _mut>](&mut self, index: u32) -> Option<TableMut<'_, $t>> {
                    TableMut::new(self, |archive| &mut archive.resource.$name, index)
                }

                pub(crate) fn [<get_ $name _slice>](&self, index: u32, count: u32) -> Option<TableSliceRef<'_, $t>> {
                    TableSliceRef::new(self, &self.resource.$name, index, count)
                }

                // pub(crate) fn [<get_internal_ $name _slice_mut>](&mut self, index: ArcIndex, count: u32) -> Option<TableSliceMut<'_, $t>> {
                //     TableSliceMut::new(self, |archive| &mut archive.resource.$name, index.0, count)
                // }
            )*
        }
    }
}

impl Archive {
    decl_lookup! {
        file_path => FilePath,
        stream_path => StreamPath,
        file_package => FilePackage
    }

    decl_access! {
        file_path => FilePath,
        file_entity => FileEntity,
        file_info => FileInfo,
        file_desc => FileDesc,
        file_data => FileData,
        file_package => FilePackage,
        file_package_child => FilePackageChild,
        file_group => FileGroup,
        stream_folder => StreamFolder,
        stream_path => StreamPath,
        stream_desc => StreamDesc,
        stream_data => StreamData
    }

    pub fn quick_serialize(&self) -> Vec<u8> {
        self.resource.quick_serialize()
    }

    pub fn serialize_tables(&self) -> Result<(&[u8], Box<[u8]>), std::io::Error> {
        self.resource
            .into_bytes(self)
            .map(|bytes| (self.resource.raw_data.as_ref(), bytes))
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, std::io::Error> {
        // SAFETY: Confirms that the metadata is proper by checking the magic after reading it
        let metadata = unsafe {
            let metadata = reader.read_binary::<ArchiveMetadata>()?;
            if metadata.magic != ArchiveMetadata::MAGIC {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Expected magic {:#x} found {:#x}",
                        ArchiveMetadata::MAGIC,
                        metadata.magic
                    ),
                ));
            }

            metadata
        };

        reader.seek(SeekFrom::Start(metadata.resource_table_offset))?;

        let decompressed_section = reader.read_compressed_data()?;
        let resource = ResourceTables::from_bytes(decompressed_section)?;

        Ok(Self { metadata, resource })
    }
}
