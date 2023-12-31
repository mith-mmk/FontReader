pub mod color;
pub mod outline;
pub mod platforms;
pub mod requires;

// #[cfg(feature = "layout")]
pub mod extentions;
// #[cfg(feature = "layout")]
pub mod layouts;
mod ttc;
use std::fmt;

use crate::{fontheader::TableRecord, util::u32_to_string};
use bin_rs::reader::BinaryReader;
pub use outline::glyf::Glyph;
pub use requires::name::NameID;
pub use ttc::TTCHeader;

#[derive(Debug, Clone)]
pub struct OTFHeader {
    pub(crate) sfnt_version: u32,
    pub(crate) num_tables: u16,
    pub(crate) search_range: u16,
    pub(crate) entry_selector: u16,
    pub(crate) range_shift: u16,
    pub(crate) table_records: Box<Vec<TableRecord>>,
}

impl OTFHeader {
    pub(crate) fn new<R: BinaryReader>(file: &mut R) -> Self {
        let sfnt_version = file.read_u32_be().unwrap();
        let num_tables = file.read_u16_be().unwrap();
        let search_range = file.read_u16_be().unwrap();
        let entry_selector = file.read_u16_be().unwrap();
        let range_shift = file.read_u16_be().unwrap();
        let mut table_records = Vec::new();
        for _ in 0..num_tables {
            let table_tag = file.read_u32_be().unwrap();
            let check_sum = file.read_u32_be().unwrap();
            let offset = file.read_u32_be().unwrap();
            let length = file.read_u32_be().unwrap();
            table_records.push(TableRecord {
                table_tag,
                check_sum,
                offset,
                length,
            });
        }

        Self {
            sfnt_version,
            num_tables,
            search_range,
            entry_selector,
            range_shift,
            table_records: Box::new(table_records),
        }
    }

    pub(crate) fn to_stirng(&self) -> String {
        let mut string = "OTFHeader\n".to_string();
        string += &format!("sfnt_version: {}\n", u32_to_string(self.sfnt_version));
        string += &format!("num_tables: {}\n", self.num_tables);
        string += &format!("search_range: {}\n", self.search_range);
        string += &format!("entry_selector: {}\n", self.entry_selector);
        string += &format!("range_shift: {}\n", self.range_shift);
        for table_record in self.table_records.iter() {
            string += &format!("table_tag: {} ", u32_to_string(table_record.table_tag));
            string += &format!("check_sum: {} ", table_record.check_sum);
            string += &format!("offset: {} ", table_record.offset);
            string += &format!("length: {}\n", table_record.length);
        }
        string
    }
}

impl fmt::Display for OTFHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_stirng())
    }
}
