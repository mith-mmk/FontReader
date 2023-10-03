use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

use crate::opentype::{outline::glyf, requires};

#[derive(Debug, Clone)]

pub(crate) enum Coverage {
    Format1(CoverageFormat1),
    Format2(CoverageFormat2),
}

impl Coverage {
    pub(crate) fn contains(&self, griph_id: usize) -> Option<usize> {
        match self {
            Coverage::Format1(coverage) => {
                let glyph_ids = &coverage.glyph_ids;
                let result = glyph_ids.binary_search(&(griph_id as u16));
                match result {
                    Ok(index) => Some(index),
                    Err(_) => None,
                }
            }
            Coverage::Format2(coverage) => {
                let range_records: &Vec<RangeRecord> = &coverage.range_records;
                let result = range_records.binary_search_by(|x| {
                    if x.start_glyph_id >= (griph_id as u16) && x.end_glyph_id <= (griph_id as u16)
                    {
                        std::cmp::Ordering::Equal
                    } else if x.end_glyph_id > (griph_id as u16) {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                });
                match result {
                    Ok(index) => {
                        let reage_record = &range_records[index];
                        let index = reage_record.start_coverage_index
                            + (griph_id as u16 - reage_record.start_glyph_id);
                        Some(index as usize)
                    }
                    Err(_) => None,
                }
            }
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            Coverage::Format1(coverage) => {
                let mut string = String::new();
                string += &format!("CoverageFormat: {}\n", coverage.coverage_format);
                string += &format!("GlyphCount: {}\n", coverage.glyph_count);
                string += &format!("GlyphIds: {:?}\n", coverage.glyph_ids);
                string
            }
            Coverage::Format2(coverage) => {
                let mut string = String::new();
                string += &format!("CoverageFormat: {}\n", coverage.coverage_format);
                string += &format!("RangeCount: {}\n", coverage.range_count);
                string += &format!("RangeRecords: {:?}\n", coverage.range_records);
                string
            }
        }
    }

    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let coverage_format = reader.read_u16_be()?;
        match coverage_format {
            1 => {
                let glyph_count = reader.read_u16_be()?;
                let mut glyph_ids = Vec::new();
                for _ in 0..glyph_count {
                    glyph_ids.push(reader.read_u16_be()?);
                }
                Ok(Coverage::Format1(CoverageFormat1 {
                    coverage_format,
                    glyph_count,
                    glyph_ids,
                }))
            }
            2 => {
                let range_count = reader.read_u16_be()?;
                let mut range_records = Vec::new();
                for _ in 0..range_count {
                    let start_glyph_id = reader.read_u16_be()?;
                    let end_glyph_id = reader.read_u16_be()?;
                    let start_coverage_index = reader.read_u16_be()?;
                    range_records.push(RangeRecord {
                        start_glyph_id,
                        end_glyph_id,
                        start_coverage_index,
                    });
                }
                Ok(Coverage::Format2(CoverageFormat2 {
                    coverage_format,
                    range_count,
                    range_records,
                }))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unknown coverage format",
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CoverageFormat1 {
    pub(crate) coverage_format: u16,
    pub(crate) glyph_count: u16,
    pub(crate) glyph_ids: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct CoverageFormat2 {
    pub(crate) coverage_format: u16,
    pub(crate) range_count: u16,
    pub(crate) range_records: Vec<RangeRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct RangeRecord {
    pub(crate) start_glyph_id: u16,
    pub(crate) end_glyph_id: u16,
    pub(crate) start_coverage_index: u16,
}
