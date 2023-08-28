#[derive(Debug, Clone)]
pub(crate) struct CMAP {
  pub(crate) version: u16,
  pub(crate) num_tables: u16,
  pub(crate) encoding_records: Vec<Box<EncodingRecord>>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EncodingRecord {
  pub(crate) platform_id: u16,
  pub(crate) encoding_id: u16,
  pub(crate) offset: u32,
}


impl EncodingRecord {
  pub(crate) fn to_string(&self) -> String {
    format!("platform_id: {}, encoding_id: {}, offset: {}", self.platform_id, self.encoding_id, self.offset)
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
pub(crate) struct CmapSubtable {
    format: u16,
    length: u16,
    language: u16,
    glyph_id_array: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct CmapHighByteEncoding {
    format: u16,
    length: u16,
    language: u16,
    sub_header_keys: Vec<u16>,
    sub_headers: Vec<CmapSubheader>,
    glyph_id_array: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct CmapSubheader {
    first_code: u16,
    entry_count: u16,
    id_delta: i16,
    id_range_offset: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct SegmentMappingToDelta {
    format: u16,
    length: u16,
    language: u16,
    seg_count_x2: u16,
    search_range: u16,
    entry_selector: u16,
    range_shift: u16,
    end_code: Vec<u16>,
    reserved_pad: u16,
    start_code: Vec<u16>,
    id_delta: Vec<u16>,
    id_range_offset: Vec<u16>,
    glyph_id_array: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct TrimmedTableMapping {
    format: u16,
    length: u16,
    language: u16,
    first_code: u16,
    entry_count: u16,
    glyph_id_array: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct SequentialMapGroup {
    start_char_code: u32,
    end_char_code: u32,
    start_glyph_id: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Mixed16and32Coverage {
    format: u16,
    reserved: u16,
    length: u32,
    language: u32,
    is32: Vec<u8>,
    num_groups: u32,
    groups: Vec<SequentialMapGroup>,
}

#[derive(Debug, Clone)]
pub(crate) struct TrimmedArray {
    format: u16,
    length: u16,
    language: u16,
    first_code: u16,
    entry_count: u16,
    glyph_id_array: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct SegmentedCoverage {
    format: u16,
    reserved: u16,
    length: u32,
    language: u32,
    n_groups: u32,
    groups: Vec<SequentialMapGroup>,
}

#[derive(Debug, Clone)]
pub(crate) struct ManyToOneRangeMapping {
    format: u16,
    reserved: u16,
    length: u32,
    language: u32,
    num_chars: u32,
    ranges: Vec<ConstantMapGroup>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstantMapGroup {
    start_char_code: u32,
    end_char_code: u32,
    glyph_id: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct UnicodeVariationSeauences {
    format: u16,
    reserved: u16,
    length: u32,
    num_var_selector_records: u32,
    var_selector_records: Vec<VarSelectorRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct VarSelectorRecord {
    var_selector: u32, // from u24
    default_uvs_offset: u32,
    non_default_uvs_offset: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct DefaultUVS {
    num_unicode_value_ranges: u32,
    unicode_value_ranges: Vec<UnicodeValueRange>,
}

#[derive(Debug, Clone)]
pub(crate) struct UnicodeValueRange {
    start_unicode_value: u32,
    additional_count: u8,
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
pub(crate) fn load_cmap_table(font_buffer: &[u8],offset: u32 , length: u32) -> CMAP {
    let buffer = &font_buffer[offset as usize..(offset + length) as usize];
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
            offset: subtable_offset,
        }));
    }

    CMAP {
        version,
        num_tables,
        encoding_records: encoding_records,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EncodingRecordPriority {
  pub(crate) records: Vec<Box<EncodingRecord>>,
  pub(crate) uvs: Vec<Box<EncodingRecord>>,
  pub(crate) substitute: Vec<Box<EncodingRecord>>,
}

pub(crate) fn select_cmap(encoding_records: &Vec<Box<EncodingRecord>>) ->EncodingRecordPriority {
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

// CMAP selector priority



// 100. Platform ID = 2 (ISO) and Encoding ID  = 1 (ISO 10646)
// 100. Platform ID = 2 (ISO) and Encoding ID  = 0 (7-bit ASCII)
// 7. Platform ID = 2 (ISO) and Encoding ID  = 2 (ISO 8859-1), Platform ID = 3 (Microsoft) and Encoding ID  = 0, 2, 3, 4, 5 or 6 select language
//100. other
