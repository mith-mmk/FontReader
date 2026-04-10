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
    pub(crate) fn new<R: BinaryReader>(reader: &mut R) -> Result<Self, std::io::Error> {
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
        header.sfnt_version = reader.read_u32_be()?;
        header.major_version = reader.read_u16_be()?;
        header.minor_version = reader.read_u16_be()?;
        header.num_fonts = reader.read_u32_be()?;
        for _ in 0..header.num_fonts {
            header.table_directory.push(reader.read_u32_be()?);
        }
        // Version2
        if header.major_version == 2 {
            header.ul_dsig_tag = reader.read_u32_be()?;
            header.ul_dsig_length = reader.read_u32_be()?;
            header.ul_dsig_offset = reader.read_u32_be()?;
        }
        let mut font_collection = Vec::new();

        for i in 0..header.num_fonts {
            reader.seek(SeekFrom::Start(header.table_directory[i as usize] as u64))?;
            font_collection.push(OTFHeader::new(reader)?);
        }
        header.font_collection = Box::new(font_collection);
        Ok(header)
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "TTCHeader\n".to_string();
        string += &format!("sfnt_version: {}\n", self.sfnt_version);
        string += &format!("major_version: {}\n", self.major_version);
        string += &format!("minor_version: {}\n", self.minor_version);
        string += &format!("num_fonts: {}\n", self.num_fonts);
        /*/
        for (i, table) in self.table_directory.iter().enumerate() {
            string += &format!("table[{}]: {:08X}\n", i, table);
        }*/
        if self.major_version == 2 {
            string += &format!("ul_dsig_tag: {:08X}\n", self.ul_dsig_tag);
            string += &format!("ul_dsig_length: {}\n", self.ul_dsig_length);
            string += &format!("ul_dsig_offset: {:08X}\n", self.ul_dsig_offset);
        }
        for (i, font) in self.font_collection.iter().enumerate() {
            string += &format!("\nfont[{}]:\n {}\n", i, font.to_stirng());
        }
        string
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    #[test]
    fn ttc_header_returns_error_on_truncated_input() {
        let mut reader = BytesReader::new(b"ttc");
        assert!(TTCHeader::new(&mut reader).is_err());
    }
}
