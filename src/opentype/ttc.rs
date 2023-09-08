use std::io::SeekFrom;

use crate::opentype::OTFHeader;
use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub struct TTCHeader {
    pub(crate) sfnt_version: u32,
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) num_fonts: u32,
    pub(crate) table_directory: Box<Vec<u32>>,
    // Version2
    pub(crate) ul_dsig_tag: u32,
    pub(crate) ul_dsig_length: u32,
    pub(crate) ul_dsig_offset: u32,
    pub(crate) font_collection: Box<Vec<OTFHeader>>,
}

impl TTCHeader {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R) -> Self {
        let mut header = TTCHeader {
            sfnt_version: 0,
            major_version: 0,
            minor_version: 0,
            num_fonts: 0,
            table_directory: Box::<Vec<u32>>::default(),
            ul_dsig_tag: 0,
            ul_dsig_length: 0,
            ul_dsig_offset: 0,
            font_collection: Box::<Vec<OTFHeader>>::default(),
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
        let mut font_collection = Vec::new();

        for i in 0..header.num_fonts {
            reader
                .seek(SeekFrom::Start(header.table_directory[i as usize] as u64))
                .unwrap();
            font_collection.push(OTFHeader::new(reader));
        }
        header.font_collection = Box::new(font_collection);
        header
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "TTCHeader\n".to_string();
        string += &format!("sfnt_version: {}\n", self.sfnt_version);
        string += &format!("major_version: {}\n", self.major_version);
        string += &format!("minor_version: {}\n", self.minor_version);
        string += &format!("num_fonts: {}\n", self.num_fonts);
        for (i, table) in self.table_directory.iter().enumerate() {
            string += &format!("table[{}]: {:08X}\n", i, table);
        }
        if self.major_version == 2 {
            string += &format!("ul_dsig_tag: {:08X}\n", self.ul_dsig_tag);
            string += &format!("ul_dsig_length: {}\n", self.ul_dsig_length);
            string += &format!("ul_dsig_offset: {:08X}\n", self.ul_dsig_offset);
        }
        for (i, font) in self.font_collection.iter().enumerate() {
            string += &format!("font[{}]:\n {}\n", i, font.to_string());
        }
        string
    }
}
