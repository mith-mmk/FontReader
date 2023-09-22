use bin_rs::reader::BinaryReader;
use std::{fmt, io::Error};

// cmap is a table that maps character codes to glyph index values

#[derive(Debug, Clone)]
pub(crate) struct CMAP {
    pub(crate) version: u16,
    pub(crate) num_tables: u16,
    pub(crate) encoding_records: Box<Vec<EncodingRecord>>,
    pub(crate) buffer: Vec<u8>,
}

impl fmt::Display for CMAP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl CMAP {
    pub(crate) fn new<R: BinaryReader>(
        file: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, Error> {
        load_cmap_table(file, offset, length)
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "cmap\n".to_string();
        let version = format!("Version {}\n", self.version);
        string += &version;
        let num_tables = format!("Num Tables {}\n", self.num_tables);
        string += &num_tables;
        for (i, encoding_record) in self.encoding_records.iter().enumerate() {
            string += &format!("Encoding Record {} : ", i);
            string += &encoding_record.to_string();
            string += "\n";
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EncodingRecord {
    pub(crate) platform_id: u16,
    pub(crate) encoding_id: u16,
    pub(crate) subtable_offset: u32,
}

impl fmt::Display for EncodingRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl EncodingRecord {
    pub(crate) fn to_string(&self) -> String {
        format!(
            "platform_id: {} {}, encoding_id: {} {}, offset: {}",
            self.platform_id,
            self.get_platform(),
            self.encoding_id,
            self.get_encoding(),
            self.subtable_offset
        )
    }

    pub(crate) fn get_platform(&self) -> String {
        match self.platform_id {
            0 => "Unicode".to_string(),   // Unicode
            1 => "Macintosh".to_string(), // Macintosh
            2 => "ISO".to_string(),       // ISO, deprecated
            3 => "Windows".to_string(),   // Windows
            4 => "Custom".to_string(),    // Custom
            _ => "Unknown".to_string(),
        }
    }

    pub(crate) fn get_encoding(&self) -> String {
        match self.platform_id {
            0 => {
                match self.encoding_id {
                    0 => "Unicode1.0".to_string(),     // Unicode 1.0 semantics, deprecated
                    1 => "Unicode1.1".to_string(),     // Unicode 1.1 semantics, deprecated
                    2 => "ISO/IEC 10646".to_string(),  // ISO/IEC 110646, deprecated
                    3 => "Unicode2.0 Bmp".to_string(), // Unicode 2.0 and onwards, BMP only
                    4 => "Unicode2.0 Full".to_string(), // Unicode 2.0 and onwards, full repertoire
                    5 => "Unicode Variation Sequences".to_string(), // 異字体セレクタ
                    6 => "Unicode Full Repertoire".to_string(),
                    _ => "Unknown".to_string(),
                }
            }
            1 => match self.encoding_id {
                0 => "Roman".to_string(),
                1 => "Japanese".to_string(),
                2 => "Chinese".to_string(),
                3 => "Korean".to_string(),
                4 => "Arabic".to_string(),
                5 => "Hebrew".to_string(),
                6 => "Greek".to_string(),
                7 => "Russian".to_string(),
                8 => "RSymbol".to_string(),
                9 => "Devanagari".to_string(),
                10 => "Gurmukhi".to_string(),
                11 => "Gujarati".to_string(),
                12 => "Oriya".to_string(),
                13 => "Bengali".to_string(),
                14 => "Tamil".to_string(),
                15 => "Telugu".to_string(),
                16 => "Kannada".to_string(),
                17 => "Malayalam".to_string(),
                18 => "Sinhalese".to_string(),
                19 => "Burmese".to_string(),
                20 => "Khmer".to_string(),
                21 => "Thai".to_string(),
                22 => "Laotian".to_string(),
                23 => "Georgian".to_string(),
                24 => "Armenian".to_string(),
                25 => "Chinese".to_string(),
                26 => "Tibetan".to_string(),
                27 => "Mongolian".to_string(),
                28 => "Geez".to_string(),
                29 => "Slavic".to_string(),
                30 => "Vietnamese".to_string(),
                31 => "Sindhi".to_string(),
                32 => "Uninterpreted".to_string(),
                _ => "Unknown".to_string(),
            },
            2 => match self.encoding_id {
                0 => "7Bit Ascii".to_string(),
                1 => "Iso10646".to_string(),
                2 => "Iso8859-1".to_string(),
                _ => "Unknown".to_string(),
            },
            3 => match self.encoding_id {
                0 => "Symbol".to_string(),
                1 => "Unicode Bmp".to_string(),
                2 => "ShiftJIS".to_string(),
                3 => "PRC".to_string(),
                4 => "Big5".to_string(),
                5 => "Wansung".to_string(),
                6 => "Johab".to_string(),
                7 => "Reserved".to_string(),
                8 => "Reserved".to_string(),
                9 => "Reserved".to_string(),
                10 => "Unicode Full".to_string(),
                _ => "Unknown".to_string(),
            },
            4 => "unknown".to_string(),
            _ => "Unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CmapEncodings {
    pub(crate) cmap: Box<CMAP>,
    pub(crate) cmap_encodings: Box<Vec<CmapEncoding>>,
}

impl CmapEncodings {
    pub(crate) fn new<R: BinaryReader>(
        file: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, Error> {
        let cmap = load_cmap_table(file, offset, length)?;
        let cmap_encodings = get_cmap_maps(&cmap);

        Ok(CmapEncodings {
            cmap: Box::new(cmap),
            cmap_encodings: Box::new(cmap_encodings),
        })
    }

    pub(crate) fn get_encoding_engine(&self) -> Box<Vec<EncodingRecord>> {
        self.cmap.encoding_records.clone()
    }

    pub(crate) fn get_glyph_position_from_uvs(&self, code_number: u32, vs: u32) -> u32 {
        let cmap_encodings = &self.cmap_encodings;
        let mut current_encoding = -1;
        for i in 0..cmap_encodings.len() {
            if cmap_encodings[i].cmap_subtable.get_format() == 14 {
                current_encoding = i as isize;
                break;
            }
        }
        if current_encoding == -1 {
            return self.get_glyph_position(code_number);
        }
        let current_encoding = current_encoding as usize;
        let cmap_encoding = &cmap_encodings[current_encoding];
        let cmap_subtable = &cmap_encoding.cmap_subtable;
        let mut position = 0;

        match cmap_subtable.as_ref() {
            CmapSubtable::Format14(format14) => {
                'outer: for i in 0..format14.num_var_selector_records {
                    let var_selector_record = &format14.var_selector_records[i as usize];
                    if var_selector_record.var_selector == vs {
                        let non_default_uvs = &var_selector_record.non_default_uvs;
                        // let num_unicode_value_ranges = non_default_uvs.num_unicode_value_ranges;
                        let i = non_default_uvs.unicode_value_ranges.binary_search_by(|x| {
                            if x.unicode_value == code_number {
                                std::cmp::Ordering::Equal
                            } else if x.unicode_value < code_number {
                                std::cmp::Ordering::Less
                            } else {
                                std::cmp::Ordering::Greater
                            }
                        });
                        if let Ok(i) = i {
                            position = non_default_uvs.unicode_value_ranges[i].glyph_id as u32;
                            break 'outer;
                        }
                        /*
                        for i in 0..num_unicode_value_ranges {
                            let code = non_default_uvs.unicode_value_ranges[i as usize].unicode_value;
                            if code == code_number {
                                position =  non_default_uvs.unicode_value_ranges[i as usize].glyph_id as u32;
                                break 'outer;
                            }
                        } */
                        break;
                    }
                }
            }
            _ => {
                print!("{:?}", cmap_subtable);
            }
        }
        if position == 0 {
            position = self.get_glyph_position(code_number);
        }
        position
    }
    

    pub(crate) fn get_glyph_position(&self, code_number: u32) -> u32 {
        let cmap_encodings = &self.cmap_encodings;
        let mut current_encoding = 0;
        for i in 0..cmap_encodings.len() {
            if cmap_encodings[i].cmap_subtable.get_format() == 12 {
                current_encoding = i;
                break;
            }
            if cmap_encodings[i].cmap_subtable.get_format() == 4 {
                current_encoding = i;
            }
        }
        let cmap_encoding = &cmap_encodings[current_encoding];
        let cmap_subtable = &cmap_encoding.cmap_subtable;
        let mut position = 0;

        match cmap_subtable.as_ref() {
            CmapSubtable::Format12(format12) => {

                let i = format12.groups.binary_search_by(|x| {
                    if x.start_char_code <= code_number && code_number <= x.end_char_code {
                        std::cmp::Ordering::Equal
                    } else if x.start_char_code < code_number {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                });
                let i = if let Ok(i) = i { i } else { return 0 };

                let group = &format12.groups[i];
                position = group.start_glyph_id + (code_number - group.start_char_code);

                /* 
                for i in 0..format12.groups.len() {
                    let group = &format12.groups[i];
                    if group.start_char_code <= code_number && code_number <= group.end_char_code {
                        position = group.start_glyph_id + (code_number - group.start_char_code);
                        break;
                    }
                }
                */
            }
            CmapSubtable::Format4(format4) => {
                let code_number = code_number as u16;
                let i = format4.codes.binary_search_by(|x| {
                    if x.0 <= code_number && code_number <= x.1 {
                        std::cmp::Ordering::Equal
                    } else if x.0 < code_number {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                });

                let i = if let Ok(i) = i { i } else { return 0 };
                let id_range_offset = format4.id_range_offset[i] as u32;
                let gid = if id_range_offset == 0 {
                    ((code_number as i32 + format4.id_delta[i] as i32) & 0xffff) as u32
                } else {
                    let mut offset = format4.id_range_offset[i] as u32 / 2 + i as u32
                        - format4.seg_count_x2 as u32 / 2;
                    // reverce calculation
                    offset += code_number as u32 - format4.codes[i].0 as u32;
                    format4.glyph_id_array[offset as usize] as u32
                };
                position = gid;

                /*

                for i in 0..format4.codes.len() {
                    match format4.codes[i].0 <= code_number && code_number <= format4.codes[i].1
                    {
                        true => {
                            let id_range_offset = format4.id_range_offset[i] as u32;
                            let gid = if id_range_offset == 0 {
                                ((code_number as i32 + format4.id_delta[i] as i32) & 0xffff) as u32
                            } else {
                                let mut offset = format4.id_range_offset[i] as u32 / 2 + i as u32
                                    - format4.seg_count_x2 as u32 / 2;
                                // reverce calculation
                                offset += code_number as u32 - format4.codes[i].0 as u32;
                                format4.glyph_id_array[offset as usize] as u32
                            };
                            position = gid;
                            break;
                        }
                        false => (),
                    }
                }
                */
            }

            CmapSubtable::Format13(format13) => {
                for i in 0..format13.groups.len() {
                    let group = &format13.groups[i];
                    if group.start_char_code <= code_number && code_number <= group.end_char_code {
                        position = group.glyph_id;
                        break;
                    }
                }
            }

            _ => {
                print!("{:?}", cmap_subtable);
                panic!("Not support format");
            }
        }
        position
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CmapEncoding {
    pub(crate) encoding_record: Box<EncodingRecord>,
    pub(crate) cmap_subtable: Box<CmapSubtable>,
}

#[derive(Debug, Clone)]
pub(crate) enum CmapSubtable {
    Format0(ByteEncoding),          // do not use
    Format2(CmapHighByteEncoding),  // also use
    Format4(SegmentMappingToDelta), // use
    Format6(TrimmedTableMapping),   // do not use
    Format8(Mixed16and32Coverage),  // do not use
    Format10(TrimmedArray),         // do not use
    Format12(SegmentedCoverage),    // use
    Format13(ManyToOneRangeMapping),        // use
    Format14(UnicodeVariationSeauences),  // part of use
    FormatUnknown,
}

impl CmapSubtable {
    pub(crate) fn get_format(&self) -> u16 {
        match self {
            CmapSubtable::Format0(_) => 0,
            CmapSubtable::Format2(_) => 2,
            CmapSubtable::Format4(_) => 4,
            CmapSubtable::Format6(_) => 6,
            CmapSubtable::Format8(_) => 8,
            CmapSubtable::Format10(_) => 10,
            CmapSubtable::Format12(_) => 12,
            CmapSubtable::Format13(_) => 13,
            CmapSubtable::Format14(_) => 14,
            CmapSubtable::FormatUnknown => 0xFFFF,
        }
    }

    pub(crate) fn get_part_of_string(&self, length: usize) -> String {
        match self {
            CmapSubtable::Format0(format0) => {
                let mut string = "Format 0: Byte encoding table\n".to_string();
                let length = if length > format0.glyph_id_array.len() {
                    format0.glyph_id_array.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format0.format);
                string += &format!("length: {}\n", format0.length);
                string += &format!("language: {}\n", format0.language);
                string += "glyph_id_array:";
                for i in 0..length {
                    if i % 16 == 0 {
                        string += &format!("\n{:002} ", format0.glyph_id_array[i]);
                    }
                    string += &format!("  {:02X} ", format0.glyph_id_array[i]);
                }
                string += "\n";
                string
            }
            CmapSubtable::Format2(format2) => {
                let mut string = "Format 2: High-byte mapping through table\n".to_string();
                let length = if length > format2.sub_header_keys.len() {
                    format2.sub_header_keys.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format2.format);
                string += &format!("length: {}\n", format2.length);
                string += &format!("language: {}\n", format2.language);
                string += "sub_header_keys:";
                for i in 0..length {
                    if i % 16 == 0 {
                        string += &format!("\n{:002} ", format2.sub_header_keys[i]);
                    }
                    string += &format!("  {:02X} ", format2.sub_header_keys[i]);
                }
                string += "\n";
                string
            }
            CmapSubtable::Format4(format4) => {
                let mut string = "Format 4: Segment mapping to delta values\n".to_string();
                // SegmentMappingToDelta
                let length = if length > format4.codes.len() {
                    format4.codes.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format4.format);
                string += &format!("length: {}\n", format4.length);
                string += &format!("language: {}\n", format4.language);
                string += &format!("seg_count_x2: {}\n", format4.seg_count_x2);
                string += &format!("search_range: {}\n", format4.search_range);
                string += &format!("entry_selector: {}\n", format4.entry_selector);
                string += &format!("range_shift: {}\n", format4.range_shift);
                string += "start_code end_code\n";
                for i in 0..length {
                    if i < format4.codes.len() && i < format4.codes.len() {
                        string += &format!(
                            "{} {:04x} {:04x}\n",
                            i, format4.codes[i].0, format4.codes[i].1
                        );
                    }
                }
                string += "\n";
                string
            }
            CmapSubtable::Format6(format6) => {
                let mut string = "Format 6: Trimmed table mapping\n".to_string();
                let length = if length > format6.glyph_id_array.len() {
                    format6.glyph_id_array.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format6.format);
                string += &format!("length: {}\n", format6.length);
                string += &format!("language: {}\n", format6.language);
                string += &format!("first_code: {}\n", format6.first_code);
                string += &format!("entry_count: {}\n", format6.entry_count);
                string += "glyph_id_array:";
                for i in 0..length {
                    if i % 16 == 0 {
                        string += &format!("\n{:002} ", format6.glyph_id_array[i]);
                    }
                    string += &format!("  {:02X} ", format6.glyph_id_array[i]);
                }
                string += "\n";
                string
            }
            CmapSubtable::Format8(format8) => {
                let mut string = "Format 8: Mixed 16-bit and 32-bit coverage\n".to_string();

                let length = if length > format8.is32.len() {
                    format8.is32.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format8.format);
                string += &format!("reserved: {}\n", format8.reserved);
                string += &format!("length: {}\n", format8.length);
                string += &format!("language: {}\n", format8.language);
                string += "is32:";
                for i in 0..length {
                    if i % 16 == 0 {
                        string += &format!("\n{:002} ", format8.is32[i]);
                    }
                    string += &format!("  {:02X} ", format8.is32[i]);
                }
                string += "\n";

                string += &format!("num_groups: {}\n", format8.num_groups);
                let lenghth = if length > format8.groups.len() {
                    format8.groups.len()
                } else {
                    length
                };
                string += "groups:\n";
                for i in 0..lenghth {
                    string += &format!("\n{} ", format8.groups[i].to_string());
                }
                string += "\n";
                string
            }
            CmapSubtable::Format10(format10) => {
                let mut string = "Format 10: Trimmed array\n".to_string();
                let length = if length > format10.glyph_id_array.len() {
                    format10.glyph_id_array.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format10.format);
                string += &format!("reserved: {}\n", format10.reserved);
                string += &format!("length: {}\n", format10.length);
                string += &format!("language: {}\n", format10.language);
                string += &format!("start_char_code: {}\n", format10.start_char_code);
                string += &format!("num_chars: {}\n", format10.num_chars);
                string += "glyph_id_array:";
                for i in 0..length {
                    if i % 16 == 0 {
                        string += &format!("\n{:002} ", format10.glyph_id_array[i]);
                    }
                    string += &format!("  {:02X} ", format10.glyph_id_array[i]);
                }
                string += "\n";
                string
            }
            CmapSubtable::Format12(format12) => {
                let mut string = "Format 12: Segmented coverage\n".to_string();
                let length = if length > format12.groups.len() {
                    format12.groups.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format12.format);
                string += &format!("reserved: {}\n", format12.reserved);
                string += &format!("length: {}\n", format12.length);
                string += &format!("language: {}\n", format12.language);
                string += &format!("num_groups: {}\n", format12.num_groups);
                string += "groups:\n";
                for i in 0..length {
                    let seg = &format12.groups[i].to_string();
                    string += &format!("{:3} {}\n", i, seg);
                }
                string += "\n";
                string
            }
            CmapSubtable::Format13(format13) => {
                let mut string = "Format 13: Many-to-one range mappings\n".to_string();
                let length = if length > format13.groups.len() {
                    format13.groups.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format13.format);
                string += &format!("reserved: {}\n", format13.reserved);
                string += &format!("length: {}\n", format13.length);
                string += &format!("language: {}\n", format13.language);
                string += &format!("num_groups: {}\n", format13.num_groups);
                string += "groups:\n";
                for i in 0..length {
                    string += &format!("{:3} {}\n", i, format13.groups[i].to_string());
                }
                string += "\n";
                string
            }
            CmapSubtable::Format14(format14) => {
                let mut string = "Format 14: Unicode Variation Sequences\n".to_string();
                let length = if length > format14.var_selector_records.len() {
                    format14.var_selector_records.len()
                } else {
                    length
                };
                string += &format!("format: {}\n", format14.format);
                string += &format!("reserved: {}\n", format14.reserved);
                string += &format!("length: {}\n", format14.length);
                string += &format!(
                    "num_var_selector_records: {}\n",
                    format14.num_var_selector_records
                );
                string += "var_selector_records:\n";
                for i in 0..length {
                    string += &format!(
                        "{:04x} {:002}\n",
                        i,
                        format14.var_selector_records[i].to_stirng()
                    );
                }
                string += "\n";
                string
            }
            CmapSubtable::FormatUnknown => "Unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
// Format 0: Byte encoding table
pub(crate) struct ByteEncoding {
    format: u16,
    length: u16,
    language: u16,
    glyph_id_array: Vec<u8>,
}

#[derive(Debug, Clone)]
// Format 2: High-byte mapping through table
pub(crate) struct CmapHighByteEncoding {
    pub(crate) format: u16,
    pub(crate) length: u16,
    pub(crate) language: u16,
    pub(crate) sub_header_keys: Vec<u16>,
    pub(crate) sub_headers: Vec<CmapSubheader>,
    pub(crate) glyph_id_array: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct CmapSubheader {
    pub(crate) first_code: u16,
    pub(crate) entry_count: u16,
    pub(crate) id_delta: i16,
    pub(crate) id_range_offset: u16,
}

#[derive(Debug, Clone)]
// format 4
pub(crate) struct SegmentMappingToDelta {
    pub(crate) format: u16,
    pub(crate) length: u16,
    pub(crate) language: u16,
    pub(crate) seg_count_x2: u16,
    pub(crate) search_range: u16,
    pub(crate) entry_selector: u16,
    pub(crate) range_shift: u16,
    pub(crate) codes:Vec<(u16, u16)>, // start_code, end_code
    // pub(crate) end_code: Vec<u16>,
    // pub(crate) start_code: Vec<u16>,
    pub(crate) id_delta: Vec<i16>,
    pub(crate) id_range_offset: Vec<u16>,
    pub(crate) glyph_id_array: Vec<u16>,
}

#[derive(Debug, Clone)]
// format 6 Trimmed table mapping
pub(crate) struct TrimmedTableMapping {
    pub(crate) format: u16,
    pub(crate) length: u16,
    pub(crate) language: u16,
    pub(crate) first_code: u16,
    pub(crate) entry_count: u16,
    pub(crate) glyph_id_array: Vec<u16>,
}

#[derive(Debug, Clone)]
// format 8 Mixed 16-bit and 32-bit coverage
pub(crate) struct Mixed16and32Coverage {
    pub(crate) format: u16,
    pub(crate) reserved: u16,
    pub(crate) length: u32,
    pub(crate) language: u32,
    pub(crate) is32: Vec<u8>,
    pub(crate) num_groups: u32,
    pub(crate) groups: Vec<SequentialMapGroup>,
}

#[derive(Debug, Clone)]
// format 10 Trimmed array
pub(crate) struct TrimmedArray {
    pub(crate) format: u16,
    pub(crate) reserved: u16,
    pub(crate) length: u32,
    pub(crate) language: u32,
    pub(crate) start_char_code: u32,
    pub(crate) num_chars: u32,
    pub(crate) glyph_id_array: Vec<u16>,
}

#[derive(Debug, Clone)]
// format 12 Segmented coverage
pub(crate) struct SegmentedCoverage {
    pub(crate) format: u16,
    pub(crate) reserved: u16,
    pub(crate) length: u32,
    pub(crate) language: u32,
    pub(crate) num_groups: u32,
    pub(crate) groups: Vec<SequentialMapGroup>,
}

#[derive(Debug, Clone)]
pub(crate) struct SequentialMapGroup {
    pub(crate) start_char_code: u32,
    pub(crate) end_char_code: u32,
    pub(crate) start_glyph_id: u32,
}

impl SequentialMapGroup {
    fn to_string(&self) -> String {
        format!(
            "start_char_code: {:04x}, end_char_code: {:04x}, start_glyph_id: {}",
            self.start_char_code, self.end_char_code, self.start_glyph_id
        )
    }
}

#[derive(Debug, Clone)]
// format 13 Many-to-one range mappings
pub(crate) struct ManyToOneRangeMapping {
    pub(crate) format: u16,
    pub(crate) reserved: u16,
    pub(crate) length: u32,
    pub(crate) language: u32,
    pub(crate) num_groups: u32,
    pub(crate) groups: Vec<ConstantMapGroup>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstantMapGroup {
    pub(crate) start_char_code: u32,
    pub(crate) end_char_code: u32,
    pub(crate) glyph_id: u32,
}

impl ConstantMapGroup {
    fn to_string(&self) -> String {
        format!(
            "sstart_char_code: {:04x}, end_char_code: {:04x}, glyph_id: {}",
            self.start_char_code, self.end_char_code, self.glyph_id
        )
    }
}

#[derive(Debug, Clone)]
// format 14 Unicode Variation Sequences
pub(crate) struct UnicodeVariationSeauences {
    pub(crate) format: u16,
    pub(crate) reserved: u16,
    pub(crate) length: u32,
    pub(crate) num_var_selector_records: u32,
    pub(crate) var_selector_records: Vec<VarSelectorRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct VarSelectorRecord {
    pub(crate) var_selector: u32, // from u24
    pub(crate) default_uvs_offset: u32,
    pub(crate) default_uvs: DefaultUVS,
    pub(crate) non_default_uvs_offset: u32,
    pub(crate) non_default_uvs: NonDefautUVS,
}

impl VarSelectorRecord {
    fn to_stirng(&self) -> String {
        format!("var_selector: {:04x}\n default_uvs_offset: {}, default_uvs: {}\n non_default_uvs_offset: {}, non_default_uvs: {}", self.var_selector, self.default_uvs_offset, self.default_uvs.to_string(), self.non_default_uvs_offset, self.non_default_uvs.to_string())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DefaultUVS {
    pub(crate) num_unicode_value_ranges: u32,
    pub(crate) unicode_value_ranges: Vec<UnicodeValueRange>,
}

impl DefaultUVS {
    fn to_string(&self) -> String {
        let mut string = format!(
            "num_unicode_value_ranges: {}",
            self.num_unicode_value_ranges
        );
        let length = if self.unicode_value_ranges.len() > 10 {
            10
        } else {
            self.unicode_value_ranges.len()
        };
        for i in 0..length {
            string += &format!(
                "{} {:06x} {}\n",
                i,
                self.unicode_value_ranges[i].start_unicode_value,
                self.unicode_value_ranges[i].additional_count
            );
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UnicodeValueRange {
    pub(crate) start_unicode_value: u32,
    pub(crate) additional_count: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct NonDefautUVS {
    num_unicode_value_ranges: u32,
    unicode_value_ranges: Vec<UVSMapping>,
}

impl NonDefautUVS {
    fn to_string(&self) -> String {
        let mut string = format!(
            "num_unicode_value_ranges: {}\n",
            self.num_unicode_value_ranges
        );
        let length = if self.unicode_value_ranges.len() > 10 {
            10
        } else {
            self.unicode_value_ranges.len()
        };
        for i in 0..length {
            string += &format!("{}\n", self.unicode_value_ranges[i].to_string());
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UVSMapping {
    unicode_value: u32,
    glyph_id: u32,
}

impl UVSMapping {
    fn to_string(&self) -> String {
        format!(
            "\nunicode_value: {:04X}, glyph_id: {}",
            self.unicode_value, self.glyph_id
        )
    }
}

// load_cmap_table(buffer.clone(), record.offset, record.length) -> CMAP
fn load_cmap_table<R: BinaryReader>(file: &mut R, offset: u32, length: u32) -> Result<CMAP, Error> {
    file.seek(std::io::SeekFrom::Start(offset as u64))?;

    let version = file.read_u16_be()?;
    let num_tables = file.read_u16_be()?;
    let mut encoding_records = Vec::new();
    for _ in 0..num_tables {
        let platform_id = file.read_u16_be()?;
        let encoding_id = file.read_u16_be()?;
        let subtable_offset = file.read_u32_be()?;
        encoding_records.push(EncodingRecord {
            platform_id,
            encoding_id,
            subtable_offset,
        });
    }
    file.seek(std::io::SeekFrom::Start(offset as u64))?;
    let buffer = file.read_bytes_as_vec(length as usize)?;

    Ok(CMAP {
        version,
        num_tables,
        encoding_records: Box::new(encoding_records),
        buffer,
    })
}

#[derive(Debug, Clone)]
pub(crate) struct EncodingRecordPriority {
    pub(crate) records: Vec<EncodingRecord>,
    pub(crate) uvs: Vec<EncodingRecord>,
    pub(crate) substitute: Vec<EncodingRecord>,
}

pub(crate) fn select_encoding(encoding_records: &Vec<EncodingRecord>) -> EncodingRecordPriority {
    let mut uvs = Vec::new();
    let mut substitute = Vec::new();
    let mut priolities = Vec::new();
    for record in encoding_records.iter() {
        match record.platform_id {
            0 => {
                // Unicode
                match record.encoding_id {
                    //100. Platform ID = 0 (Unicode) and Encoding ID  = 0 (Unicode 1.0 semantics, deprecated)
                    //100. Platform ID = 0 (Unicode) and Encoding ID  = 1 (Unicode version 1.1)
                    //100. Platform ID = 0 (Unicode) and Encoding ID  = 2 (ISO 10646)
                    // 5. Platform ID = 0 (Unicode) and Encoding ID  = 3 (Unicode BMP only)
                    // 1. Platform ID = 0 (Unicode) and Encoding ID  = 4 (Unicode version 2.0 or later)
                    // uvs format 14
                    // 1. Platform ID = 0 (Unicode) and Encoding ID  = 5 (Unicode Variation Sequences)
                    // substitude format 13 fallback to format 4
                    // 2. Platform ID = 0 (Unicode) and Encoding ID  = 6 (Unicode Full Repertoire)
                    0 | 1 | 2 => {
                        priolities.push(100);
                    }
                    3 => {
                        priolities.push(5);
                    }
                    4 => {
                        priolities.push(1);
                    }
                    5 => {
                        uvs.push(record.clone());
                        priolities.push(101);
                    }
                    6 => {
                        substitute.push(record.clone());
                        priolities.push(101);
                    }
                    _ => {
                        priolities.push(100);
                    }
                }
            }
            1 => {
                // 4. Platform ID = 1 (Macintosh)
                priolities.push(3);
            }
            2 => {
                // ISO, deprecated
                priolities.push(100);
            }
            3 => {
                // Windows
                // 5. Platform ID = 3 (Microsoft) and Encoding ID  = 1 (Unicode BMP only)
                // 2. Platform ID = 3 (Microsoft) and Encoding ID  = 10 (Unicode full repertoire)
                match record.encoding_id {
                    10 => {
                        priolities.push(2);
                    }
                    1 => {
                        priolities.push(5);
                    }
                    _ => {
                        priolities.push(100);
                    }
                }
            }
            4 => {
                // Custom
                priolities.push(100);
            }
            _ => {
                priolities.push(100);
            }
        }
    }
    let mut priolity = Vec::new();
    // sort by priority
    for (i, priority) in priolities.iter().enumerate() {
        priolity.push((i, *priority));
    }
    priolity.sort_by(|a, b| a.1.cmp(&b.1));

    let mut records_priolity = Vec::new();
    for (i, _) in priolity.iter() {
        let encoding_record = encoding_records[*i].clone();
        records_priolity.push(encoding_record);
    }

    EncodingRecordPriority {
        records: records_priolity,
        uvs,
        substitute,
    }
}

pub(crate) fn get_subtable(encoding_record: &EncodingRecord, buffer: &[u8]) -> CmapSubtable {
    let offset = encoding_record.subtable_offset as usize;
    let buffer = &buffer[offset..];
    let format = u16::from_be_bytes([buffer[0], buffer[1]]);
    match format {
        0 => {
            // Format0
            let length = u16::from_be_bytes([buffer[2], buffer[3]]);
            let language = u16::from_be_bytes([buffer[4], buffer[5]]);
            let mut glyph_id_array = Vec::new();
            for i in 6..length as usize {
                glyph_id_array.push(buffer[i]);
            }
            CmapSubtable::Format0(ByteEncoding {
                format,
                length,
                language,
                glyph_id_array,
            })
        }
        2 => {
            // Format2
            let length = u16::from_be_bytes([buffer[2], buffer[3]]);
            let language = u16::from_be_bytes([buffer[4], buffer[5]]);
            let mut sub_header_keys = Vec::new();
            // sub header keys
            let mut offset = 6;
            for _ in 0..256 {
                let sub_header_key = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                sub_header_keys.push(sub_header_key);
            }

            let mut sub_headers = Vec::new();
            // sub header
            for _ in 0..256 {
                let first_code = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                let entry_count = u16::from_be_bytes([buffer[offset + 2], buffer[offset + 3]]);
                let id_delta = i16::from_be_bytes([buffer[offset + 4], buffer[offset + 5]]);
                let id_range_offset = u16::from_be_bytes([buffer[offset + 6], buffer[offset + 7]]);
                offset += 8;
                sub_headers.push(CmapSubheader {
                    first_code,
                    entry_count,
                    id_delta,
                    id_range_offset,
                });
            }
            let mut glyph_id_array = Vec::new();
            while offset < length as usize {
                let glyph_id = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                glyph_id_array.push(glyph_id);
            }

            CmapSubtable::Format2(CmapHighByteEncoding {
                format,
                length,
                language,
                sub_header_keys,
                sub_headers,
                glyph_id_array,
            })
        }
        4 => {
            // SegmentMappingToDelta
            // Todo!
            let length = u16::from_be_bytes([buffer[2], buffer[3]]);
            let language = u16::from_be_bytes([buffer[4], buffer[5]]);
            let seg_count_x2 = u16::from_be_bytes([buffer[6], buffer[7]]);
            let seg_count = seg_count_x2 / 2;
            let search_range = u16::from_be_bytes([buffer[8], buffer[9]]);
            let entry_selector = u16::from_be_bytes([buffer[10], buffer[11]]);
            let range_shift: u16 = u16::from_be_bytes([buffer[12], buffer[13]]);
            let mut end_code = Vec::with_capacity(seg_count as usize);
            let mut offset: usize = 14;
            for _ in 0..seg_count {
                let code = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                end_code.push(code);
            }
            let _: u16 = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
            offset += 2;
            let mut codes = Vec::with_capacity(seg_count as usize);
            for i in 0..seg_count as usize {
                let code = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                codes.push((code, end_code[i]));
            }
            let mut id_delta = Vec::with_capacity(seg_count as usize);
            for _ in 0..seg_count {
                let delta = i16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                id_delta.push(delta);
            }
            let mut id_range_offset = Vec::with_capacity(seg_count as usize);
            for _ in 0..seg_count {
                let range_offset = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                id_range_offset.push(range_offset);
            }
            let mut glyph_id_array = Vec::new();
            while offset < (length - 1) as usize {
                let glyph_id = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                glyph_id_array.push(glyph_id);
            }
            CmapSubtable::Format4(SegmentMappingToDelta {
                format,
                length,
                language,
                seg_count_x2,
                search_range,
                entry_selector,
                range_shift,
                codes,
                id_delta,
                id_range_offset,
                glyph_id_array,
            })
        }
        6 => {
            // Format 6: Trimmed table mapping
            let length = u16::from_be_bytes([buffer[2], buffer[3]]);
            let language = u16::from_be_bytes([buffer[4], buffer[5]]);
            let first_code = u16::from_be_bytes([buffer[6], buffer[7]]);
            let entry_count = u16::from_be_bytes([buffer[8], buffer[9]]);
            let mut glyph_id_array = Vec::new();
            let mut offset = 10;
            for _ in 0..entry_count {
                let glyph_id = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                glyph_id_array.push(glyph_id);
            }

            CmapSubtable::Format6(TrimmedTableMapping {
                format,
                length,
                language,
                first_code,
                entry_count,
                glyph_id_array,
            })
        }
        8 => {
            // Format 8 mixed 16-bit and 32-bit coverage
            let reserved = u16::from_be_bytes([buffer[2], buffer[3]]);
            let length = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
            let language = u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
            // i32 [8192]
            let mut offset = 12;
            let mut is32 = Vec::new();
            for _ in 0..8192 {
                is32.push(buffer[offset]);
                offset += 1;
            }
            let num_groups = u32::from_be_bytes([
                buffer[offset],
                buffer[offset + 1],
                buffer[offset + 2],
                buffer[offset + 3],
            ]);
            let mut groups = Vec::new();
            offset += 4;
            for _ in 0..num_groups {
                let start_char_code = u32::from_be_bytes([
                    buffer[offset],
                    buffer[offset + 1],
                    buffer[offset + 2],
                    buffer[offset + 3],
                ]);
                let end_char_code = u32::from_be_bytes([
                    buffer[offset + 4],
                    buffer[offset + 5],
                    buffer[offset + 6],
                    buffer[offset + 7],
                ]);
                let start_glyph_id = u32::from_be_bytes([
                    buffer[offset + 8],
                    buffer[offset + 9],
                    buffer[offset + 10],
                    buffer[offset + 11],
                ]);
                offset += 12;
                groups.push(SequentialMapGroup {
                    start_char_code,
                    end_char_code,
                    start_glyph_id,
                });
            }
            CmapSubtable::Format8(Mixed16and32Coverage {
                format,
                reserved,
                length,
                language,
                is32,
                num_groups,
                groups,
            })
        }
        10 => {
            // Format 10 Trimed array
            let reserved = u16::from_be_bytes([buffer[2], buffer[3]]);
            let length: u32 = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
            let language: u32 = u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
            let start_char_code: u32 =
                u32::from_be_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
            let num_chars: u32 =
                u32::from_be_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
            let mut glyph_id_array = Vec::new();
            let mut offset = 20;
            for _ in 0..num_chars {
                let glyph_id = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                offset += 2;
                glyph_id_array.push(glyph_id);
            }
            CmapSubtable::Format10(TrimmedArray {
                format,
                reserved,
                length,
                language,
                start_char_code,
                num_chars,
                glyph_id_array,
            })
        }
        12 => {
            // Format 12 Segmented coverage
            let reserved = u16::from_be_bytes([buffer[2], buffer[3]]);
            let length: u32 = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
            let language: u32 = u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
            let num_groups: u32 =
                u32::from_be_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
            let mut groups = Vec::new();
            let mut offset = 16;
            for _ in 0..num_groups {
                let start_char_code = u32::from_be_bytes([
                    buffer[offset],
                    buffer[offset + 1],
                    buffer[offset + 2],
                    buffer[offset + 3],
                ]);
                let end_char_code = u32::from_be_bytes([
                    buffer[offset + 4],
                    buffer[offset + 5],
                    buffer[offset + 6],
                    buffer[offset + 7],
                ]);
                let start_glyph_id = u32::from_be_bytes([
                    buffer[offset + 8],
                    buffer[offset + 9],
                    buffer[offset + 10],
                    buffer[offset + 11],
                ]);
                offset += 12;
                groups.push(SequentialMapGroup {
                    start_char_code,
                    end_char_code,
                    start_glyph_id,
                });
            }
            CmapSubtable::Format12(SegmentedCoverage {
                format,
                reserved,
                length,
                language,
                num_groups,
                groups,
            })
        }
        13 => {
            // format 13 Many-to-one range mappings
            let format = u16::from_be_bytes([buffer[0], buffer[1]]);
            let reserved = u16::from_be_bytes([buffer[2], buffer[3]]);
            let length: u32 = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
            let language: u32 = u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
            let num_groups: u32 =
                u32::from_be_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
            let mut groups = Vec::new();
            let mut offset = 16;
            for _ in 0..num_groups {
                let start_char_code = u32::from_be_bytes([
                    buffer[offset],
                    buffer[offset + 1],
                    buffer[offset + 2],
                    buffer[offset + 3],
                ]);
                let end_char_code = u32::from_be_bytes([
                    buffer[offset + 4],
                    buffer[offset + 5],
                    buffer[offset + 6],
                    buffer[offset + 7],
                ]);
                let glyph_id = u32::from_be_bytes([
                    buffer[offset + 8],
                    buffer[offset + 9],
                    buffer[offset + 10],
                    buffer[offset + 11],
                ]);
                offset += 12;
                groups.push(ConstantMapGroup {
                    start_char_code,
                    end_char_code,
                    glyph_id,
                });
            }
            CmapSubtable::Format13(ManyToOneRangeMapping {
                format,
                reserved,
                length,
                language,
                num_groups,
                groups,
            })
        }
        14 => {
            // format 14 Unicode Variation Sequences
            let length = u32::from_be_bytes([buffer[2], buffer[3], buffer[4], buffer[5]]);
            let num_var_selector_records =
                u32::from_be_bytes([buffer[6], buffer[7], buffer[8], buffer[9]]);
            let mut offest = 10;
            let mut var_selector_records = Vec::new();
            for _ in 0..num_var_selector_records {
                let var_selector =
                    u32::from_be_bytes([0, buffer[offest], buffer[offest + 1], buffer[offest + 2]]);
                // 32bit
                let default_uvs_offset = u32::from_be_bytes([
                    buffer[offest + 3],
                    buffer[offest + 4],
                    buffer[offest + 5],
                    buffer[offest + 6],
                ]);
                let non_default_uvs_offset = u32::from_be_bytes([
                    buffer[offest + 7],
                    buffer[offest + 8],
                    buffer[offest + 9],
                    buffer[offest + 10],
                ]);
                offest += 11;

                let default_uvs = if default_uvs_offset > 0 {
                    let uvs_offset = default_uvs_offset as usize;
                    let uvs_buffer = &buffer[uvs_offset..];
                    let num_unicode_value_ranges = u32::from_be_bytes([
                        uvs_buffer[0],
                        uvs_buffer[1],
                        uvs_buffer[2],
                        uvs_buffer[3],
                    ]);
                    let mut unicode_value_ranges = Vec::new();
                    let mut uvs_offset = 4;
                    for _ in 0..num_unicode_value_ranges {
                        let start_unicode_value = u32::from_be_bytes([
                            0,
                            uvs_buffer[uvs_offset],
                            uvs_buffer[uvs_offset + 1],
                            uvs_buffer[uvs_offset + 2],
                        ]);
                        let additional_count = uvs_buffer[uvs_offset + 3];
                        uvs_offset += 4;
                        unicode_value_ranges.push(UnicodeValueRange {
                            start_unicode_value,
                            additional_count,
                        });
                    }
                    DefaultUVS {
                        num_unicode_value_ranges,
                        unicode_value_ranges,
                    }
                } else {
                    DefaultUVS {
                        num_unicode_value_ranges: 0,
                        unicode_value_ranges: Vec::new(),
                    }
                };
                let non_default_uvs = if non_default_uvs_offset > 0 {
                    let uvs_offset = non_default_uvs_offset as usize;
                    let non_default_uvs_buffer = &buffer[uvs_offset..];
                    let num_unicode_value_ranges = u32::from_be_bytes([
                        non_default_uvs_buffer[0],
                        non_default_uvs_buffer[1],
                        non_default_uvs_buffer[2],
                        non_default_uvs_buffer[3],
                    ]);
                    let mut unicode_value_ranges = Vec::new();
                    let mut uvs_offset = 4;
                    for _ in 0..num_unicode_value_ranges {
                        let unicode_value = u32::from_be_bytes([
                            0,
                            non_default_uvs_buffer[uvs_offset],
                            non_default_uvs_buffer[uvs_offset + 1],
                            non_default_uvs_buffer[uvs_offset + 2],
                        ]);
                        let glyph_id = u16::from_be_bytes([
                            non_default_uvs_buffer[uvs_offset + 3],
                            non_default_uvs_buffer[uvs_offset + 4],
                        ]) as u32;
                        uvs_offset += 5;
                        unicode_value_ranges.push(UVSMapping {
                            unicode_value,
                            glyph_id,
                        });
                    }
                    NonDefautUVS {
                        num_unicode_value_ranges,
                        unicode_value_ranges,
                    }
                } else {
                    NonDefautUVS {
                        num_unicode_value_ranges: 0,
                        unicode_value_ranges: Vec::new(),
                    }
                };

                var_selector_records.push(VarSelectorRecord {
                    var_selector,
                    default_uvs_offset,
                    default_uvs,
                    non_default_uvs_offset,
                    non_default_uvs,
                });
            }

            CmapSubtable::Format14(UnicodeVariationSeauences {
                format,
                reserved: 0,
                length,
                num_var_selector_records,
                var_selector_records,
            })
        }
        _ => {
            // unknown
            CmapSubtable::Format0(ByteEncoding {
                format,
                length: 0,
                language: 0,
                glyph_id_array: Vec::new(),
            })
        }
    }
}

pub(crate) fn get_cmap_maps(cmap: &CMAP) -> Vec<CmapEncoding> {
    let encoding_records = &cmap.encoding_records;
    let mut cmap_encodings = Vec::new();
    for enconding_record in encoding_records.as_slice() {
        let buffer = &cmap.buffer;
        let subtable = get_subtable(enconding_record, buffer);
        cmap_encodings.push(CmapEncoding {
            encoding_record: Box::new(enconding_record.clone()),
            cmap_subtable: Box::new(subtable),
        });
    }
    cmap_encodings
}
