use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use hash40::Hash40;

use crate::{
    archive::{containers::Bucket, file_package::SubPackageRef, resource::serialization::SerState},
    hash::HashWithData,
    io::WriteBinExt,
    BinaryRepr,
};

use super::{
    containers::{BucketLookup, IndexLookup, Table},
    file_data::FileData,
    file_desc::FileDesc,
    file_entity::FileEntity,
    file_group::FileGroup,
    file_info::FileInfo,
    file_package::{FilePackage, FilePackageChild},
    file_path::FilePath,
    stream_data::StreamData,
    stream_desc::StreamDesc,
    stream_folder::StreamFolder,
    stream_path::StreamPath,
    Archive,
};

pub(crate) mod serialization;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ResourceTableHeader {
    resource_data_size: u32,
    file_path_count: u32,
    file_entity_count: u32,

    file_package_count: u32,
    file_data_group_count: u32,
    file_package_child_count: u32,
    file_package_info_count: u32,
    file_package_desc_count: u32,
    file_package_data_count: u32,

    file_info_group_count: u32,
    file_group_info_count: u32,

    padding: [u8; 0xC],

    locale_count: u8,
    region_count: u8,

    padding2: [u8; 0x2],

    version_patch: u8,
    version_minor: u8,
    version_major: u16,

    versioned_file_group_count: u32,
    versioned_file_count: u32,
    padding3: [u8; 0x4],
    versioned_file_info_count: u32,
    versioned_file_desc_count: u32,
    versioned_file_data_count: u32,

    local_region_hash_to_region: [[u32; 3]; 14],

    stream_folder_count: u32,
    stream_path_count: u32,
    stream_desc_count: u32,
    stream_data_count: u32,
}

impl BinaryRepr for ResourceTableHeader {}

pub(crate) struct ResourceTables {
    pub raw_data: Box<[u8]>,
    pub stream_folder: Table<StreamFolder>,
    pub stream_path_lookup: IndexLookup,
    pub stream_path: Table<StreamPath>,
    pub stream_desc: Table<StreamDesc>,
    pub stream_data: Table<StreamData>,

    pub file_path_lookup: BucketLookup,
    pub file_path: Table<FilePath>,
    pub file_entity: Table<FileEntity>,
    pub file_package_lookup: IndexLookup,
    pub file_package: Table<FilePackage>,
    pub file_group: Table<FileGroup>,
    pub file_package_child: Table<FilePackageChild>,
    pub file_info: Table<FileInfo>,
    pub file_desc: Table<FileDesc>,
    pub file_data: Table<FileData>,
}

fn write_table<T: BinaryRepr + Copy>(
    table: &Table<T>,
    indexes: impl Iterator<Item = u32>,
    mut reinternalize: impl FnMut(&mut T),
    data: &mut Vec<u8>,
) -> std::io::Result<()> {
    for mut value in indexes
        .map(|index| table.get(index).expect("invalid index"))
        .copied()
    {
        reinternalize(&mut value);
        data.write_binary(&value)?;
    }

    Ok(())
}

fn write_lookup<T: 'static>(
    lookup: impl Iterator<Item = (Hash40, u32)>,
    state: &SerState,
    data: &mut Vec<u8>,
) -> std::io::Result<()> {
    for (hash, index) in lookup {
        data.write_binary(&HashWithData::new(hash, state.get::<T>(index)))?;
    }

    Ok(())
}

fn quick_serialize_table<T: BinaryRepr + Copy>(table: &Table<T>, buffer: &mut Vec<u8>) {
    buffer.extend_from_slice(T::cast_slice_bytes(table.fixed()));
    buffer.extend_from_slice(T::cast_slice_bytes(table.dynamic()));
}

fn quick_serialize_lookup(lookup: impl Iterator<Item = (Hash40, u32)>, buffer: &mut Vec<u8>) {
    for (hash, index) in lookup {
        buffer.extend_from_slice(HashWithData::new(hash, index).cast_bytes());
    }
}

impl ResourceTables {
    pub fn quick_serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::with_capacity(
            self.file_data.len() * std::mem::size_of::<FileData>()
                + self.file_desc.len() * std::mem::size_of::<FileDesc>()
                + self.file_info.len() * std::mem::size_of::<FileInfo>()
                + self.file_entity.len() * std::mem::size_of::<FileEntity>()
                + self.file_path.len() * std::mem::size_of::<FilePath>()
                + self.file_package.len() * std::mem::size_of::<FilePackage>()
                + self.file_package_child.len() * std::mem::size_of::<FilePackageChild>()
                + self.file_group.len() * std::mem::size_of::<FileGroup>()
                + self.file_path.len() * std::mem::size_of::<HashWithData>()
                + 0x400 * std::mem::size_of::<Bucket>()
                + self.file_package.len() * std::mem::size_of::<HashWithData>()
                + self.stream_folder.len() * std::mem::size_of::<StreamFolder>()
                + self.stream_path.len() * std::mem::size_of::<StreamPath>()
                + self.stream_path.len() * std::mem::size_of::<HashWithData>()
                + self.stream_desc.len() * std::mem::size_of::<StreamDesc>()
                + self.stream_data.len() * std::mem::size_of::<StreamData>(),
        );

        quick_serialize_table(&self.stream_folder, &mut buffer);
        quick_serialize_lookup(self.stream_path_lookup.iter(), &mut buffer);
        quick_serialize_table(&self.stream_path, &mut buffer);
        quick_serialize_table(&self.stream_desc, &mut buffer);
        quick_serialize_lookup(self.file_path_lookup.iter(), &mut buffer);

        let _ = buffer.write_u32::<LittleEndian>(self.file_path_lookup.len() as u32);
        let _ = buffer.write_u32::<LittleEndian>(self.file_path_lookup.bucket_count() as u32);

        for bucket in self.file_path_lookup.buckets() {
            let _ = buffer.write_binary(&bucket);
        }

        quick_serialize_table(&self.file_path, &mut buffer);
        quick_serialize_table(&self.file_entity, &mut buffer);
        quick_serialize_lookup(self.file_package_lookup.iter(), &mut buffer);
        quick_serialize_table(&self.file_package, &mut buffer);
        quick_serialize_table(&self.file_group, &mut buffer);
        quick_serialize_table(&self.file_package_child, &mut buffer);
        quick_serialize_table(&self.file_info, &mut buffer);
        quick_serialize_table(&self.file_desc, &mut buffer);
        quick_serialize_table(&self.file_data, &mut buffer);

        buffer
    }

    pub fn into_bytes(&self, archive: &Archive) -> Result<Box<[u8]>, std::io::Error> {
        let mut cache = SerState::new();

        let mut info_groups = Vec::with_capacity(0x100);

        for (index, package) in self.file_package.iter() {
            cache.reserve::<FilePackage>(index);
            package.reserve(&mut cache);

            for group in package.data_group_range() {
                let group = self
                    .file_group
                    .get(group)
                    .expect("file data group is missing");
                group.reserve(&mut cache, true);
            }

            for info in package.info_range() {
                let info = self.file_info.get(info).expect("file info is missing");
                info.reserve(&mut cache);
            }

            let package = archive
                .get_file_package(index)
                .expect("file package is missing");

            if let Some(SubPackageRef::FileGroup(group)) = package.sub_package() {
                info_groups.push(group.index());
            }
        }

        let mut info_start = None;

        for group in info_groups {
            if !cache.try_reserve::<FileGroup>(group) {
                continue;
            }

            if info_start.is_none() {
                info_start = Some(group);
            }

            let group = self
                .file_group
                .get(group)
                .expect("file group index should be valid");
            group.reserve(&mut cache, false);

            for info in group.child_range() {
                let info = self
                    .file_info
                    .get(info)
                    .expect("file info index should be valid");
                info.reserve(&mut cache);

                for desc in info.descriptor_range() {
                    let desc = self
                        .file_desc
                        .get(desc)
                        .expect("file desc index should be valid");
                    desc.reserve(&mut cache);
                }
            }
        }

        let info_start = info_start.unwrap();

        for (index, _) in self.file_path.iter() {
            cache.reserve::<FilePath>(index);
        }

        for (index, _) in self.file_entity.iter() {
            cache.reserve::<FileEntity>(index);
        }

        for (index, stream_folder) in self.stream_folder.iter() {
            cache.reserve::<StreamFolder>(index);
            stream_folder.reserve(&mut cache);

            for path in stream_folder.stream_path_range() {
                let stream_path = self
                    .stream_path
                    .get(path)
                    .expect("stream path index should be invalid");
                stream_path.reserve(&mut cache);

                for desc in stream_path.descriptor_range() {
                    let desc = self
                        .stream_desc
                        .get(desc)
                        .expect("file entity index should be valid");
                    desc.reserve(&mut cache);
                }
            }
        }

        let mut buffer: Vec<u8> = Vec::with_capacity(
            self.file_data.len() * std::mem::size_of::<FileData>()
                + self.file_desc.len() * std::mem::size_of::<FileDesc>()
                + self.file_info.len() * std::mem::size_of::<FileInfo>()
                + self.file_entity.len() * std::mem::size_of::<FileEntity>()
                + self.file_path.len() * std::mem::size_of::<FilePath>()
                + self.file_package.len() * std::mem::size_of::<FilePackage>()
                + self.file_package_child.len() * std::mem::size_of::<FilePackageChild>()
                + self.file_group.len() * std::mem::size_of::<FileGroup>()
                + self.file_path.len() * std::mem::size_of::<HashWithData>()
                + 0x400 * std::mem::size_of::<Bucket>()
                + self.file_package.len() * std::mem::size_of::<HashWithData>()
                + self.stream_folder.len() * std::mem::size_of::<StreamFolder>()
                + self.stream_path.len() * std::mem::size_of::<StreamPath>()
                + self.stream_path.len() * std::mem::size_of::<HashWithData>()
                + self.stream_desc.len() * std::mem::size_of::<StreamDesc>()
                + self.stream_data.len() * std::mem::size_of::<StreamData>(),
        );

        write_table(
            &self.stream_folder,
            cache.iter::<StreamFolder>(),
            |folder| folder.reinternalize(&cache),
            &mut buffer,
        )?;
        write_lookup::<StreamPath>(self.stream_path_lookup.iter(), &cache, &mut buffer)?;
        write_table(
            &self.stream_path,
            cache.iter::<StreamPath>(),
            |path| path.reinternalize(&cache),
            &mut buffer,
        )?;
        write_table(
            &self.stream_desc,
            cache.iter::<StreamDesc>(),
            |desc| desc.reinternalize(&cache),
            &mut buffer,
        )?;
        write_table(
            &self.stream_data,
            cache.iter::<StreamData>(),
            |_| {},
            &mut buffer,
        )?;

        buffer.write_u32::<LittleEndian>(self.file_path_lookup.len() as u32)?;
        buffer.write_u32::<LittleEndian>(self.file_path_lookup.bucket_count() as u32)?;

        for bucket in self.file_path_lookup.buckets() {
            buffer.write_binary(&bucket)?;
        }

        write_lookup::<FilePath>(self.file_path_lookup.iter(), &cache, &mut buffer)?;
        write_table(
            &self.file_path,
            cache.iter::<FilePath>(),
            |path| path.reinternalize(&cache),
            &mut buffer,
        )?;
        write_table(
            &self.file_entity,
            cache.iter::<FileEntity>(),
            |entity| entity.reinternalize(&cache, self.file_package.len() as u32),
            &mut buffer,
        )?;
        write_lookup::<FilePackage>(self.file_package_lookup.iter(), &cache, &mut buffer)?;
        write_table(
            &self.file_package,
            cache.iter::<FilePackage>(),
            |package| package.reinternalize(&cache),
            &mut buffer,
        )?;
        write_table(
            &self.file_group,
            cache
                .iter::<FileGroup>()
                .take_while(|index| *index < info_start),
            |group| group.reinternalize_data(&cache, self.file_package.len() as u32),
            &mut buffer,
        )?;
        write_table(
            &self.file_group,
            cache
                .iter::<FileGroup>()
                .skip_while(|index| *index < info_start),
            |group| group.reinternalize_info(&cache),
            &mut buffer,
        )?;
        write_table(
            &self.file_package_child,
            cache.iter::<FilePackageChild>(),
            |child| child.reinternalize(&cache),
            &mut buffer,
        )?;
        write_table(
            &self.file_info,
            cache.iter::<FileInfo>(),
            |info| info.reinternalize(&cache),
            &mut buffer,
        )?;
        write_table(
            &self.file_desc,
            cache.iter::<FileDesc>(),
            |desc| desc.reinternalize(&cache),
            &mut buffer,
        )?;
        write_table(
            &self.file_data,
            cache.iter::<FileData>(),
            |_| {},
            &mut buffer,
        )?;

        Ok(buffer.into_boxed_slice())
    }

    pub fn from_bytes(mut bytes: Box<[u8]>) -> std::io::Result<Self> {
        // SAFETY: We read the resource table and then perform some checks on
        //      data that should be consistent if we have read it from the right location
        let resource_table = unsafe {
            let resource_table = *ResourceTableHeader::cast(&bytes);
            if resource_table.locale_count != 14 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Expected 14 locales, found {:#x}",
                        resource_table.locale_count
                    ),
                ));
            }

            if resource_table.region_count != 5 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Expected 5 regions, found {:#x}",
                        resource_table.region_count
                    ),
                ));
            }

            resource_table
        };

        let mut cursor_pos = std::mem::size_of::<ResourceTableHeader>();

        macro_rules! get {
            ($t:path, $size:expr) => {
                unsafe {
                    let value = <$t>::new(&mut bytes[cursor_pos..], ($size) as usize);
                    cursor_pos += value.fixed_byte_len();
                    value
                }
            };
        }

        let stream_folder = get!(Table<StreamFolder>, resource_table.stream_folder_count);
        let stream_path_lookup = get!(IndexLookup, resource_table.stream_path_count);
        let stream_path = get!(Table<StreamPath>, resource_table.stream_path_count);
        let stream_desc = get!(Table<StreamDesc>, resource_table.stream_desc_count);
        let stream_data = get!(Table<StreamData>, resource_table.stream_data_count);

        let file_path_lookup_count =
            LittleEndian::read_u32(&bytes[cursor_pos..cursor_pos + 4]) as usize;
        let file_path_bucket_count =
            LittleEndian::read_u32(&bytes[cursor_pos + 4..cursor_pos + 8]) as usize;

        cursor_pos += 8;

        let file_path_lookup = unsafe {
            let lookup = BucketLookup::new(
                &mut bytes[cursor_pos..],
                file_path_lookup_count,
                file_path_bucket_count,
            );
            cursor_pos += lookup.fixed_byte_len();
            lookup
        };

        let file_path = get!(Table<FilePath>, resource_table.file_path_count);
        let file_entity = get!(Table<FileEntity>, resource_table.file_entity_count);
        let file_package_lookup = get!(IndexLookup, resource_table.file_package_count);
        let file_package = get!(Table<FilePackage>, resource_table.file_package_count);
        let file_group = get!(
            Table<FileGroup>,
            resource_table.file_info_group_count
                + resource_table.file_data_group_count
                + resource_table.versioned_file_group_count
        );

        let file_package_child = get!(
            Table<FilePackageChild>,
            resource_table.file_package_child_count
        );

        let file_info = get!(
            Table<FileInfo>,
            resource_table.file_package_info_count
                + resource_table.file_group_info_count
                + resource_table.versioned_file_info_count
        );

        let file_desc = get!(
            Table<FileDesc>,
            resource_table.file_package_desc_count
                + resource_table.file_group_info_count
                + resource_table.versioned_file_desc_count
        );

        let file_data = get!(
            Table<FileData>,
            resource_table.file_package_data_count
                + resource_table.file_group_info_count
                + resource_table.versioned_file_data_count
        );

        Ok(Self {
            raw_data: bytes,
            stream_folder,
            stream_path_lookup,
            stream_path,
            stream_desc,
            stream_data,
            file_path_lookup,
            file_path,
            file_entity,
            file_package_lookup,
            file_package,
            file_group,
            file_package_child,
            file_info,
            file_desc,
            file_data,
        })
    }
}
