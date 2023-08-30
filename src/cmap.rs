use std::{io::{Read, Seek}, fmt};

#[derive(Debug, Clone)]
pub(crate) struct CMAP {
  pub(crate) version: u16,
  pub(crate) num_tables: u16,
  pub(crate) encoding_records: Vec<Box<EncodingRecord>>,
  pub(crate) buffer: Vec<u8>,
}

impl fmt::Display for CMAP {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl CMAP {
  pub(crate) fn new<R:Read + Seek>(file:R, offset: u32, length: u32) -> Self {
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
    format!("platform_id: {}, encoding_id: {}, offset: {}", self.platform_id, self.encoding_id, self.subtable_offset)
  }

  pub(crate) fn get_platform(&self) -> String {
    match self.platform_id {
      0 => "Unicode".to_string(), // Unicode
      1 => "Macintosh".to_string(), // Macintosh
      2 => "ISO".to_string(),  // ISO, deprecated
      3 => "Windows".to_string(), // Windows
      4 => "Custom".to_string(), // Custom
      _ => "Unknown".to_string(),
    }
  }

  pub(crate) fn get_encoding(&self) -> String {
    match self.platform_id {
      0 => {
        match self.encoding_id {
          0 => "Unicode1.0".to_string(), // Unicode 1.0 semantics, deprecated
          1 => "Unicode1.1".to_string(), // Unicode 1.1 semantics, deprecated
          2 => "ISO/IEC 10646".to_string(), // ISO/IEC 110646, deprecated
          3 => "Unicode2.0 Bmp".to_string(),  // Unicode 2.0 and onwards, BMP only
          4 => "Unicode2.0 Full".to_string(), // Unicode 2.0 and onwards, full repertoire
          5 => "Unicode Variation Sequences".to_string(), // 異字体セレクタ
          6 => "Unicode Full Repertoire".to_string(),
          _ => "Unknown".to_string(),
        }
      }
      1 => {
        match self.encoding_id {
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
        }
      },
      2 => {
        match self.encoding_id {
          0 => "7Bit Ascii".to_string(),
          1 => "Iso10646".to_string(),
          2 => "Iso8859-1".to_string(),
          _ => "Unknown".to_string(),
        }
      },
      3 => {
        match self.encoding_id {
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
        }
      },
      4 => {
        "unknown".to_string()
      },
      _ => "Unknown".to_string(),
    }
  }

}


#[derive(Debug, Clone)]
pub(crate) struct CmapEncoding {
  pub(crate) encoding_record: Box<EncodingRecord>,
  pub(crate) cmap_subtable: Box<CmapSubtable>,
}


#[derive(Debug, Clone)]
pub(crate) enum CmapSubtable {
    Format0(ByteEncoding),
    Format2(CmapHighByteEncoding),
    Format4(SegmentMappingToDelta),
    Format6(TrimmedTableMapping),
    Format8(Mixed16and32Coverage),
    Format10(TrimmedArray),
    Format12(SegmentedCoverage),
    Format13(ManyToOneRangeMapping),
    Format14(UnicodeVariationSeauences),
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
  pub(crate) end_code: Vec<u16>,
  pub(crate) reserved_pad: u16,
  pub(crate) start_code: Vec<u16>,
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


#[derive(Debug, Clone)]
// format 13 Many-to-one range mappings
pub(crate) struct ManyToOneRangeMapping {
  pub(crate) format: u16,
  pub(crate) reserved: u16,
  pub(crate) length: u32,
  pub(crate) language: u32,
  pub(crate) num_groups: u32,
  pub(crate) ranges: Vec<ConstantMapGroup>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstantMapGroup {
  pub(crate) start_char_code: u32,
  pub(crate) end_char_code: u32,
  pub(crate) glyph_id: u32,
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

#[derive(Debug, Clone)]
pub(crate) struct DefaultUVS {
  pub(crate) num_unicode_value_ranges: u32,
  pub(crate) unicode_value_ranges: Vec<UnicodeValueRange>,
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

#[derive(Debug, Clone)]
pub(crate) struct UVSMapping {
    unicode_value: u32,
    glyph_id: u32,
}

// load_cmap_table(buffer.clone(), record.offset, record.length) -> CMAP
fn load_cmap_table<R:Read + Seek>(mut file:R,offset: u32 , length: u32) -> CMAP {
    file.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
    let mut buffer = vec![0; length as usize];
    file.read_exact(&mut buffer).unwrap();
    let version = u16::from_be_bytes([buffer[0], buffer[1]]);
    let num_tables = u16::from_be_bytes([buffer[2], buffer[3]]);
    let mut encoding_records = Vec::new();
    let mut offset = 4;
    for _ in 0..num_tables {
        let platform_id = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        let encoding_id = u16::from_be_bytes([buffer[offset + 2], buffer[offset + 3]]);
        let subtable_offset = u32::from_be_bytes([buffer[offset + 4], buffer[offset + 5], buffer[offset + 6], buffer[offset + 7]]);
        offset += 8;
        encoding_records.push(Box::new(EncodingRecord {
            platform_id,
            encoding_id,
            subtable_offset,
        }));
    }

    CMAP {
        version,
        num_tables,
        encoding_records: encoding_records,
        buffer: buffer.to_vec()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EncodingRecordPriority {
  pub(crate) records: Vec<Box<EncodingRecord>>,
  pub(crate) uvs: Vec<Box<EncodingRecord>>,
  pub(crate) substitute: Vec<Box<EncodingRecord>>,
}

pub(crate) fn select_encoding(encoding_records: &Vec<Box<EncodingRecord>>) ->EncodingRecordPriority {
    let mut uvs = Vec::new();
    let mut substitute = Vec::new();
    let mut priolities = Vec::new();
    for record in encoding_records.iter() {
      match record.platform_id {
        0 => { // Unicode
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
            },
            3 => {
              priolities.push(5);
            },
            4 => {
              priolities.push(1);
            },
            5 => {
              uvs.push(record.clone());
              priolities.push(101);
            },
            6 => {
              substitute.push(record.clone());
              priolities.push(101);
            },
            _ => {
              priolities.push(100);
            },
          }
        },
        1 => { // 4. Platform ID = 1 (Macintosh)
          priolities.push(3);
        }
        2 => {  // ISO, deprecated
          priolities.push(100);
        },
        3 => { // Windows
          // 5. Platform ID = 3 (Microsoft) and Encoding ID  = 1 (Unicode BMP only)
          // 2. Platform ID = 3 (Microsoft) and Encoding ID  = 10 (Unicode full repertoire)
          match record.encoding_id {
            10 => {
              priolities.push(2);
            },
            1 => {
              priolities.push(5);
            },
            _ => {
              priolities.push(100);
            },
          }
        },
        4 => { // Custom
          priolities.push(100);
        },
        _ => {
          priolities.push(100);
        },
      }
    }
    let mut priolity = Vec::new();
    // sort by priority
    for (i, priority) in priolities.iter().enumerate() {
      priolity.push((i, priority.clone()));
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

pub(crate) fn get_subtable(encoding_record: &Box<EncodingRecord>, buffer: &[u8]) -> CmapSubtable {
  let offset = encoding_record.subtable_offset as usize;
  let buffer = &buffer[offset..];
  let format = u16::from_be_bytes([buffer[0], buffer[1]]);
  match format {
    0 => { // Format0
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
    },
    2 => { // Format2
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
    },
    4 => { // SegmentMappingToDelta
      // Todo!
      let length = u16::from_be_bytes([buffer[2], buffer[3]]);
      let language = u16::from_be_bytes([buffer[4], buffer[5]]);
      let seg_count_x2 = u16::from_be_bytes([buffer[6], buffer[7]]);
      let seg_count = seg_count_x2 / 2;
      let search_range = u16::from_be_bytes([buffer[8], buffer[9]]);
      let entry_selector = u16::from_be_bytes([buffer[10], buffer[11]]);
      let range_shift = u16::from_be_bytes([buffer[12], buffer[13]]);
      let mut end_code = Vec::new();
      let mut offset = 14;
      for _ in 0..seg_count {
        let code = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        end_code.push(code);
      }
      let reserved_pad = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
      offset += 2;
      let mut start_code = Vec::new();
      for _ in 0..seg_count {
        let code = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        start_code.push(code);
      }      
      let mut id_delta = Vec::new();
      for _ in 0..seg_count {
        let delta = i16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        id_delta.push(delta);
      }
      let mut id_range_offset = Vec::new();
      for _ in 0..seg_count {
        let range_offset = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        offset += 2;
        id_range_offset.push(range_offset);
      }
      let mut glyph_id_array = Vec::new();
      while offset < length as usize {
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
        end_code,
        reserved_pad,
        start_code,
        id_delta,
        id_range_offset,
        glyph_id_array,
      })
    },
    6 => { // Format 6: Trimmed table mapping
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
    },
    8 => { // Format 8 mixed 16-bit and 32-bit coverage
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
      let num_groups = u32::from_be_bytes([buffer[offset], buffer[offset + 1], buffer[offset + 2], buffer[offset + 3]]);
      let mut groups = Vec::new();
      offset += 4;
      for _ in 0..num_groups {
        let start_char_code = u32::from_be_bytes([buffer[offset], buffer[offset + 1], buffer[offset + 2], buffer[offset + 3]]);
        let end_char_code = u32::from_be_bytes([buffer[offset + 4], buffer[offset + 5], buffer[offset + 6], buffer[offset + 7]]);
        let start_glyph_id = u32::from_be_bytes([buffer[offset + 8], buffer[offset + 9], buffer[offset + 10], buffer[offset + 11]]);
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
    },
    10 => { // Format 10 Trimed array
      let reserved = u16::from_be_bytes([buffer[2], buffer[3]]);
      let length: u32 = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
      let language: u32 = u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
      let start_char_code: u32 = u32::from_be_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
      let num_chars: u32 = u32::from_be_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
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
    },
    12 => { // Format 12 Segmented coverage
      let reserved = u16::from_be_bytes([buffer[2], buffer[3]]);
      let length: u32 = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
      let language: u32 = u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
      let num_groups: u32 = u32::from_be_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
      let mut groups = Vec::new();
      let mut offset = 16;
      for _ in 0..num_groups {
        let start_char_code = u32::from_be_bytes([buffer[offset], buffer[offset + 1], buffer[offset + 2], buffer[offset + 3]]);
        let end_char_code = u32::from_be_bytes([buffer[offset + 4], buffer[offset + 5], buffer[offset + 6], buffer[offset + 7]]);
        let start_glyph_id = u32::from_be_bytes([buffer[offset + 8], buffer[offset + 9], buffer[offset + 10], buffer[offset + 11]]);
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
    },
    14 => { // format 14 Unicode Variation Sequences
      let length = u32::from_be_bytes([buffer[2], buffer[3], buffer[4], buffer[5]]);
      let num_var_selector_records = u32::from_be_bytes([buffer[6], buffer[7], buffer[8], buffer[9]]);
      let mut offest = 10;
      let mut var_selector_records = Vec::new();
      for _ in 0..num_var_selector_records {
        let var_selector = u32::from_be_bytes([0, buffer[offest], buffer[offest + 1], buffer[offest + 2]]);
        // 32bit 
        let default_uvs_offset = u32::from_be_bytes([buffer[offest + 3], buffer[offest + 4], buffer[offest + 5], buffer[offest + 6]]);
        let non_default_uvs_offset = u32::from_be_bytes([buffer[offest + 7], buffer[offest + 8], buffer[offest + 9], buffer[offest + 10]]);
        offest += 11;

        let default_uvs = if default_uvs_offset > 0 {
          let uvs_offset = default_uvs_offset as usize;
          let uvs_buffer = &buffer[uvs_offset..];
          let num_unicode_value_ranges = u32::from_be_bytes([uvs_buffer[0], uvs_buffer[1], uvs_buffer[2], uvs_buffer[3]]);
          let mut unicode_value_ranges = Vec::new();
          let mut uvs_offset = 4;
          for _ in 0..num_unicode_value_ranges {
            let start_unicode_value = u32::from_be_bytes([uvs_buffer[uvs_offset], uvs_buffer[uvs_offset + 1], uvs_buffer[uvs_offset + 2], uvs_buffer[uvs_offset + 3]]);
            let additional_count = uvs_buffer[uvs_offset + 4];
            uvs_offset += 5;
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
          let num_unicode_value_ranges = u32::from_be_bytes([non_default_uvs_buffer[0], non_default_uvs_buffer[1], non_default_uvs_buffer[2], non_default_uvs_buffer[3]]);
          let mut unicode_value_ranges = Vec::new();
          let mut uvs_offset = 4;
          for _ in 0..num_unicode_value_ranges {
            let unicode_value = u32::from_be_bytes([non_default_uvs_buffer[uvs_offset], non_default_uvs_buffer[uvs_offset + 1], non_default_uvs_buffer[uvs_offset + 2], non_default_uvs_buffer[uvs_offset + 3]]);
            let glyph_id = u32::from_be_bytes([non_default_uvs_buffer[uvs_offset + 4], non_default_uvs_buffer[uvs_offset + 5], non_default_uvs_buffer[uvs_offset + 6], non_default_uvs_buffer[uvs_offset + 7]]);
            uvs_offset += 8;            
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
    },
    _ => { // unknown
      CmapSubtable::Format0(ByteEncoding {
        format,
        length: 0,
        language: 0,
        glyph_id_array: Vec::new(),
      })
     },
    }
  }




pub(crate) fn get_cmap_maps(cmap: &CMAP) -> Vec<Box<CmapEncoding>> {
  let encoding_records = &cmap.encoding_records;
  let mut cmap_encodings = Vec::new();
  for enconding_record in encoding_records {
    let buffer = &cmap.buffer;
    let subtable = get_subtable(enconding_record, buffer);
    cmap_encodings.push(Box::new(CmapEncoding {
      encoding_record: enconding_record.clone(),
      cmap_subtable: Box::new(subtable),
    }));
  }
  cmap_encodings
}

pub(crate) fn get_cmap_encodings<R: Read + Seek>(file: R, offset: u32 , length: u32) -> Vec<Box<CmapEncoding>> {
  let cmap = load_cmap_table(file, offset, length);
  get_cmap_maps(&cmap)
}


