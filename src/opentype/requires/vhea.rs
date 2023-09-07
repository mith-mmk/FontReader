// vhea is Vertical Metrics Header Table (optional)
// https://docs.microsoft.com/en-us/typography/opentype/spec/vhea

use bin_rs::reader::BinaryReader;
use std::{fmt, io::SeekFrom};

use crate::fontheader::{FWORD, UFWORD};

#[derive(Debug, Clone)]
pub(crate) struct VHEA {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) ascender: FWORD,
    pub(crate) descender: FWORD,
    pub(crate) line_gap: FWORD,
    pub(crate) advance_height_max: UFWORD,
    pub(crate) min_top_side_bearing: FWORD,
    pub(crate) min_bottom_side_bearing: FWORD,
    pub(crate) y_max_extent: FWORD,
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

impl fmt::Display for VHEA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl VHEA {
    pub(crate) fn new<R: BinaryReader>(file: &mut R, offest: u32, length: u32) -> Self {
        get_vhea(file, offest, length)
    }

    pub(crate) fn get_accender(&self) -> i16 {
        self.ascender as i16
    }

    pub(crate) fn get_descender(&self) -> i16 {
        self.descender as i16
    }

    pub(crate) fn get_line_gap(&self) -> i16 {
        self.line_gap as i16
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
        let advance_height_max = format!("Advance Height Max {}\n", self.advance_height_max);
        string += &advance_height_max;
        let min_top_side_bearing = format!("Min Top Side Bearing {}\n", self.min_top_side_bearing);
        string += &min_top_side_bearing;
        let min_bottom_side_bearing =
            format!("Min Buttom Side Bearing {}\n", self.min_bottom_side_bearing);
        string += &min_bottom_side_bearing;
        let y_max_extent = format!("xMax Extent {}\n", self.y_max_extent);
        string += &y_max_extent;
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

fn get_vhea<R: BinaryReader>(file: &mut R, offest: u32, length: u32) -> VHEA {
    let mut file = file;
    file.seek(SeekFrom::Start(offest as u64)).unwrap();
    let major_version = file.read_u16_be().unwrap();
    let minor_version = file.read_u16_be().unwrap();
    let ascender = file.read_i16_be().unwrap();
    let descender = file.read_i16_be().unwrap();
    let line_gap = file.read_i16_be().unwrap();
    let advance_height_max = file.read_u16_be().unwrap();
    let min_top_side_bearing = file.read_i16_be().unwrap();
    let min_bottom_side_bearing = file.read_i16_be().unwrap();
    let y_max_extent = file.read_i16_be().unwrap();
    let caret_slope_rise = file.read_i16_be().unwrap();
    let caret_slope_run = file.read_i16_be().unwrap();
    let caret_offset = file.read_i16_be().unwrap();
    let reserved1 = file.read_i16_be().unwrap();
    let reserved2 = file.read_i16_be().unwrap();
    let reserved3 = file.read_i16_be().unwrap();
    let reserved4 = file.read_i16_be().unwrap();
    let metric_data_format = file.read_i16_be().unwrap();
    let number_of_hmetrics = file.read_u16_be().unwrap();

    VHEA {
        major_version,
        minor_version,
        ascender,
        descender,
        line_gap,
        advance_height_max,
        min_top_side_bearing,
        min_bottom_side_bearing,
        y_max_extent,
        caret_slope_rise,
        caret_slope_run,
        caret_offset,
        reserved1,
        reserved2,
        reserved3,
        reserved4,
        metric_data_format,
        number_of_hmetrics,
    }
}
