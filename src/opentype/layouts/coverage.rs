#[derive(Debug, Clone)]

pub(crate) enum Coverage {
    Format1(CoverageFormat1),
    Format2(CoverageFormat2),
}

impl Coverage {
    pub(crate) fn contains(&self, griph_id: usize) -> Option<usize> {
        match self {
            Coverage::Format1(coverage) => {
                for (i, glyph_id) in coverage.glyph_ids.iter().enumerate() {
                    if *glyph_id == griph_id as u16 {
                        return Some(i);
                    }
                }
                return None;
            }
            Coverage::Format2(coverage) => {
                for range_record in coverage.range_records.iter() {
                    if range_record.start_glyph_id <= griph_id as u16
                        && range_record.end_glyph_id >= griph_id as u16
                    {
                        return Some(
                            (range_record.start_coverage_index
                                + (griph_id as u16 - range_record.start_glyph_id))
                                as usize,
                        );
                    }
                }
                return None;
            }
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
