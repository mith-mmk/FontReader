#![allow(dead_code)]

use bin_rs::reader::BinaryReader;
use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;
use crate::limits::{resource_limit, DecodeLimits};

#[derive(Debug, Clone)]
pub struct WOFFHeader {
    pub(crate) sfnt_version: u32,
    pub(crate) signature: u32,
    pub(crate) flavor: u32,
    pub(crate) length: u32,
    pub(crate) num_tables: u16,
    pub(crate) reserved: u16,
    pub(crate) total_sfnt_size: u32,
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) meta_offset: u32,
    pub(crate) meta_length: u32,
    pub(crate) meta_orig_length: u32,
    pub(crate) priv_offset: u32,
    pub(crate) priv_length: u32,
}

impl WOFFHeader {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R) -> Result<Self, std::io::Error> {
        let mut header = Self {
            sfnt_version: 0,
            signature: 0,
            flavor: 0,
            length: 0,
            num_tables: 0,
            reserved: 0,
            total_sfnt_size: 0,
            major_version: 0,
            minor_version: 0,
            meta_offset: 0,
            meta_length: 0,
            meta_orig_length: 0,
            priv_offset: 0,
            priv_length: 0,
        };
        header.signature = reader.read_u32()?;
        header.flavor = reader.read_u32()?;
        header.length = reader.read_u32()?;
        header.num_tables = reader.read_u16()?;
        header.reserved = reader.read_u16()?;
        header.total_sfnt_size = reader.read_u32()?;
        header.major_version = reader.read_u16()?;
        header.minor_version = reader.read_u16()?;
        header.meta_offset = reader.read_u32()?;
        header.meta_length = reader.read_u32()?;
        header.meta_orig_length = reader.read_u32()?;
        header.priv_offset = reader.read_u32()?;
        header.priv_length = reader.read_u32()?;
        Ok(header)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WOFFTableRecord {
    pub(crate) tag: u32,
    pub(crate) offset: u32,
    pub(crate) comp_length: u32,
    pub(crate) orig_length: u32,
    pub(crate) orig_checksum: u32,
}

impl WOFFTableRecord {
    pub(crate) fn new() -> WOFFTableRecord {
        WOFFTableRecord {
            tag: 0,
            offset: 0,
            comp_length: 0,
            orig_length: 0,
            orig_checksum: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WOFFTable {
    pub(crate) tag: u32,
    pub(crate) data: Vec<u8>,
}
impl WOFFTable {
    pub(crate) fn new() -> WOFFTable {
        WOFFTable {
            tag: 0,
            data: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WOFF {
    pub(crate) header: WOFFHeader,
    pub(crate) table_records: Vec<WOFFTableRecord>,
    pub(crate) metadata: Box<String>,
    pub(crate) private_data: Box<Vec<u8>>,
    pub(crate) tables: Vec<WOFFTable>,
}

impl WOFF {
    pub(crate) fn from<B: BinaryReader>(
        reader: &mut B,
        header: WOFFHeader,
    ) -> Result<Self, std::io::Error> {
        Self::from_with_limits(reader, header, &DecodeLimits::default())
    }

    pub(crate) fn from_with_limits<B: BinaryReader>(
        reader: &mut B,
        header: WOFFHeader,
        limits: &DecodeLimits,
    ) -> Result<Self, std::io::Error> {
        if header.num_tables > 4096 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "WOFF table count exceeds supported limit",
            ));
        }
        let mut table_records = Vec::new();
        for _ in 0..header.num_tables {
            let mut table_record = WOFFTableRecord::new();
            table_record.tag = reader.read_u32()?;
            table_record.offset = reader.read_u32()?;
            table_record.comp_length = reader.read_u32()?;
            table_record.orig_length = reader.read_u32()?;
            table_record.orig_checksum = reader.read_u32()?;
            #[cfg(debug_assertions)]
            {
                let _ = crate::util::u32_to_string(table_record.tag);
            }
            table_records.push(table_record);
        }
        // read metadata
        reader.seek(std::io::SeekFrom::Start(header.meta_offset as u64))?;
        let metadata = if header.meta_length > 0 {
            if header.meta_orig_length as usize > limits.max_metadata_bytes {
                return Err(resource_limit(
                    "WOFF metadata",
                    header.meta_orig_length as usize,
                    limits.max_metadata_bytes,
                ));
            }
            let compress_metadata = reader.read_bytes_as_vec(header.meta_length as usize)?;
            match decompress_to_vec_zlib_with_limit(
                &compress_metadata,
                limits.max_metadata_bytes,
            ) {
                Ok(metadata_bytes) if metadata_bytes.len() == header.meta_orig_length as usize => {
                    String::from_utf8(metadata_bytes).unwrap_or_default()
                }
                _ => String::new(),
            }
        } else {
            "".to_string()
        };
        // read private data

        reader.seek(std::io::SeekFrom::Start(header.priv_offset as u64))?;
        if header.priv_length as usize > limits.max_table_bytes {
            return Err(resource_limit(
                "WOFF private data",
                header.priv_length as usize,
                limits.max_table_bytes,
            ));
        }
        let private_data = reader.read_bytes_as_vec(header.priv_length as usize)?;

        // read table data
        let mut tables = Vec::new();
        let mut total_decompressed = 0usize;
        for table_record in table_records.iter() {
            if table_record.comp_length as usize > limits.max_table_bytes {
                return Err(resource_limit(
                    "WOFF compressed table",
                    table_record.comp_length as usize,
                    limits.max_table_bytes,
                ));
            }
            if table_record.orig_length as usize > limits.max_table_bytes {
                return Err(resource_limit(
                    "WOFF table",
                    table_record.orig_length as usize,
                    limits.max_table_bytes,
                ));
            }
            reader.seek(std::io::SeekFrom::Start(table_record.offset as u64))?;
            let mut table = WOFFTable::new();
            table.tag = table_record.tag;
            let mut table_data = reader.read_bytes_as_vec(table_record.comp_length as usize)?;
            if table_record.comp_length != table_record.orig_length {
                table_data = decompress_to_vec_zlib_with_limit(
                    &table_data,
                    table_record.orig_length as usize,
                )
                .map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid WOFF table")
                })?;
            }
            if table_data.len() != table_record.orig_length as usize {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "WOFF table length does not match origLength",
                ));
            }
            let actual_checksum = Self::checksum(table_record.tag, &table_data);
            if actual_checksum != table_record.orig_checksum {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "WOFF table checksum does not match origChecksum for {:08x}: {:08x} != {:08x}",
                        table_record.tag, actual_checksum, table_record.orig_checksum
                    ),
                ));
            }
            total_decompressed = total_decompressed
                .checked_add(table_data.len())
                .ok_or_else(|| resource_limit("WOFF decompressed data", usize::MAX, limits.max_total_decompressed_bytes))?;
            if total_decompressed > limits.max_total_decompressed_bytes {
                return Err(resource_limit(
                    "WOFF decompressed data",
                    total_decompressed,
                    limits.max_total_decompressed_bytes,
                ));
            }
            table.data = table_data;
            tables.push(table);
        }

        Ok(WOFF {
            header,
            table_records,
            metadata: Box::new(metadata),
            private_data: Box::new(private_data),
            tables,
        })
    }

    fn checksum(tag: u32, data: &[u8]) -> u32 {
        data.chunks(4).enumerate().fold(0u32, |sum, (index, chunk)| {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            if tag == u32::from_be_bytes(*b"head") {
                let word_start = index * 4;
                for byte in word.iter_mut().enumerate() {
                    let position = word_start + byte.0;
                    if (8..12).contains(&position) {
                        *byte.1 = 0;
                    }
                }
            }
            sum.wrapping_add(u32::from_be_bytes(word))
        })
    }

    pub fn get_metadata(&self) -> &str {
        &self.metadata
    }

    pub fn get_private_data(&self) -> &[u8] {
        &self.private_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    #[test]
    fn rejects_excessive_table_count_before_reading_directory() {
        let header = WOFFHeader {
            sfnt_version: 0,
            signature: 0,
            flavor: 0,
            length: 44,
            num_tables: 4097,
            reserved: 0,
            total_sfnt_size: 0,
            major_version: 0,
            minor_version: 0,
            meta_offset: 0,
            meta_length: 0,
            meta_orig_length: 0,
            priv_offset: 0,
            priv_length: 0,
        };
        let mut reader = BytesReader::new(&[]);
        assert!(WOFF::from_with_limits(&mut reader, header, &DecodeLimits::default()).is_err());
    }
}
