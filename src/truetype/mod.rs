use std::io::SeekFrom;

use crate::{opentype::OTFHeader, util::u32_to_string};
use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub struct TTFHeader {
    pub(crate) sfnt_version: u32,
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) num_fonts: u32,
    pub(crate) table_directory: Box<Vec<u32>>,
    // Version2
    pub(crate) ul_dsig_tag: u32,
    pub(crate) ul_dsig_length: u32,
    pub(crate) ul_dsig_offset: u32,
}

impl TTFHeader {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R) -> Self {
        let mut header = TTFHeader {
            sfnt_version: 0,
            major_version: 0,
            minor_version: 0,
            num_fonts: 0,
            table_directory: Box::new(Vec::new()),
            ul_dsig_tag: 0,
            ul_dsig_length: 0,
            ul_dsig_offset: 0,
        };
        header.sfnt_version = reader.read_u32_be().unwrap();
        header.major_version = reader.read_u16_be().unwrap();
        header.minor_version = reader.read_u16_be().unwrap();
        header.num_fonts = reader.read_u32_be().unwrap();
        for _ in 0..header.num_fonts {
            header.table_directory.push(reader.read_u32_be().unwrap());
        }
        // Version2
        if header.major_version == 2 {
            header.ul_dsig_tag = reader.read_u32_be().unwrap();
            header.ul_dsig_length = reader.read_u32_be().unwrap();
            header.ul_dsig_offset = reader.read_u32_be().unwrap();
        }
        #[cfg(debug_assertions)]
        {
            println!("sfnt_version: {}", header.sfnt_version);
            println!("major_version: {}", header.major_version);
            println!("minor_version: {}", header.minor_version);
            println!("num_fonts: {}", header.num_fonts);
            for (i, table) in header.table_directory.iter().enumerate() {
                println!("table[{}]: {:08X}", i, table);
            }
            if header.major_version == 2 {
                println!("ul_dsig_tag: {:08X}", header.ul_dsig_tag);
                println!("ul_dsig_length: {}", header.ul_dsig_length);
                println!("ul_dsig_offset: {:08X}", header.ul_dsig_offset);
            }
        }
        header
    }
}

#[derive(Debug, Clone)]
pub struct TrueType {
    pub(crate) header: TTFHeader,
    pub(crate) tables: Box<Vec<OTFHeader>>,
}

impl TrueType {
    pub fn new<R: BinaryReader>(reader: &mut R) -> Self {
        let header = TTFHeader::new(reader);
        Self::from(reader, header)
    }

    pub fn from<R: BinaryReader>(reader: &mut R, header: TTFHeader) -> Self {
        let mut tables = Vec::new();

        for i in 0..header.num_fonts {
            reader
                .seek(SeekFrom::Start(header.table_directory[i as usize] as u64))
                .unwrap();
            tables.push(OTFHeader::new(reader));
        }

        Self {
            header,
            tables: Box::new(tables),
        }
    }

    fn to_string(&self) -> String {
        let mut string = "TrueType\n".to_string();
        string += &format!("sfnt_version: {}\n", self.header.sfnt_version);
        string += &format!("major_version: {}\n", self.header.major_version);
        string += &format!("minor_version: {}\n", self.header.minor_version);
        string += &format!("num_fonts: {}\n", self.header.num_fonts);
        string += &format!("ul_dsig_tag: {}\n", self.header.ul_dsig_tag);
        string += &format!("ul_dsig_length: {}\n", self.header.ul_dsig_length);
        string += &format!("ul_dsig_offset: {}\n", self.header.ul_dsig_offset);
        for (i, table) in self.tables.iter().enumerate() {
            string += &format!("table[{}]: {}\n", i, table.to_string());
        }
        string
    }
}
