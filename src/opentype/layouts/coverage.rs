#[derive(Debug, Clone)]

pub(crate) enum Coverage {
    Format1(CoverageFormat1),
    Format2(CoverageFormat2),
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