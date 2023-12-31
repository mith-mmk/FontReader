use bin_rs::reader::BinaryReader;
use std::io::Error;
use std::{fmt, io::SeekFrom};

// hhea table Font horizontal metrics header

use crate::fontheader::{FWORD, UFWORD};

#[derive(Debug, Clone)]
pub(crate) struct HHEA {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) ascender: FWORD,
    pub(crate) descender: FWORD,
    pub(crate) line_gap: FWORD,
    pub(crate) advance_width_max: UFWORD,
    pub(crate) min_left_side_bearing: FWORD,
    pub(crate) min_right_side_bearing: FWORD,
    pub(crate) x_max_extent: FWORD,
    pub(crate) caret_slope_rise: i16,
    pub(crate) caret_slope_run: i16,
    pub(crate) caret_offset: i16,
    pub(crate) reserved1: i16,
    pub(crate) reserved2: i16,
    pub(crate) reserved3: i16,
    pub(crate) reserved4: i16,
    pub(crate) metric_data_format: i16,
    pub(crate) number_of_hmetrics: u16,
}

impl fmt::Display for HHEA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl HHEA {
    pub(crate) fn new<R: BinaryReader>(
        file: &mut R,
        offest: u32,
        length: u32,
    ) -> Result<Self, Error> {
        get_hhea(file, offest, length)
    }

    pub(crate) fn get_accender(&self) -> i16 {
        self.ascender
    }

    pub(crate) fn get_descender(&self) -> i16 {
        self.descender
    }

    pub(crate) fn get_line_gap(&self) -> i16 {
        self.line_gap
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "hhea\n".to_string();
        let version = format!("Version {}.{}\n", self.major_version, self.minor_version);
        string += &version;
        let ascender = format!("Ascender {}\n", self.ascender);
        string += &ascender;
        let descender = format!("Descender {}\n", self.descender);
        string += &descender;
        let line_gap = format!("Line Gap {}\n", self.line_gap);
        string += &line_gap;
        let advance_width_max = format!("Advance Width Max {}\n", self.advance_width_max);
        string += &advance_width_max;
        let min_left_side_bearing =
            format!("Min Left Side Bearing {}\n", self.min_left_side_bearing);
        string += &min_left_side_bearing;
        let min_right_side_bearing =
            format!("Min Right Side Bearing {}\n", self.min_right_side_bearing);
        string += &min_right_side_bearing;
        let x_max_extent = format!("xMax Extent {}\n", self.x_max_extent);
        string += &x_max_extent;
        let caret_slope_rise = format!("Caret Slope Rise {}\n", self.caret_slope_rise);
        string += &caret_slope_rise;
        let caret_slope_run = format!("Caret Slope Run {}\n", self.caret_slope_run);
        string += &caret_slope_run;
        let caret_offset = format!("Caret Offset {}\n", self.caret_offset);
        string += &caret_offset;
        let reserved1 = format!("Reserved1 {}\n", self.reserved1);
        string += &reserved1;
        let reserved2 = format!("Reserved2 {}\n", self.reserved2);
        string += &reserved2;
        let reserved3 = format!("Reserved3 {}\n", self.reserved3);
        string += &reserved3;
        let reserved4 = format!("Reserved4 {}\n", self.reserved4);
        string += &reserved4;
        let metric_data_format = format!("Metric Data Format {}\n", self.metric_data_format);
        string += &metric_data_format;
        let number_of_hmetrics = format!("Number of HMetrics {}\n", self.number_of_hmetrics);
        string += &number_of_hmetrics;
        string
    }
}

fn get_hhea<R: BinaryReader>(file: &mut R, offest: u32, _length: u32) -> Result<HHEA, Error> {
    let file = file;
    file.seek(SeekFrom::Start(offest as u64))?;
    let major_version = file.read_u16_be()?;
    let minor_version = file.read_u16_be()?;
    let ascender = file.read_i16_be()?;
    let descender = file.read_i16_be()?;
    let line_gap = file.read_i16_be()?;
    let advance_width_max = file.read_u16_be()?;
    let min_left_side_bearing = file.read_i16_be()?;
    let min_right_side_bearing = file.read_i16_be()?;
    let x_max_extent = file.read_i16_be()?;
    let caret_slope_rise = file.read_i16_be()?;
    let caret_slope_run = file.read_i16_be()?;
    let caret_offset = file.read_i16_be()?;
    let reserved1 = file.read_i16_be()?;
    let reserved2 = file.read_i16_be()?;
    let reserved3 = file.read_i16_be()?;
    let reserved4 = file.read_i16_be()?;
    let metric_data_format = file.read_i16_be()?;
    let number_of_hmetrics = file.read_u16_be()?;
    Ok(HHEA {
        major_version,
        minor_version,
        ascender,
        descender,
        line_gap,
        advance_width_max,
        min_left_side_bearing,
        min_right_side_bearing,
        x_max_extent,
        caret_slope_rise,
        caret_slope_run,
        caret_offset,
        reserved1,
        reserved2,
        reserved3,
        reserved4,
        metric_data_format,
        number_of_hmetrics,
    })
}
