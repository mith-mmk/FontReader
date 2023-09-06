pub struct WOFF2 {
  pub(crate) header: WOFF2Header,
  pub(crate) table_directory: TableDirectory,
  pub(crate) collection_directory: CollectionDirectory,
  pub(crate) compressed_font_data: CompressedFontData,
  pub(crate) extended_metadata: ExtendedMetadata,
  pub(crate) private_data: PrivateData,
}

fn read_uint_base128(data: &[u8]) -> Option<u32> {
  let mut accum: u32 = 0;
  for (i, &data_byte) in data.iter().enumerate() {
    if (i == 0 && data_byte == 0x80) {return None};
    if (accum & 0xFE000000) {return None};
    accum = (accum << 7) | (data_byte & 0x7F);
    if (data_byte & 0x80) == 0 {return Some(accum)};
  }
  None
}

pub struct WOFF2Header {
  pub(crate) signature: u32,
  pub(crate) flavor: u32,
  pub(crate) length: u32,
  pub(crate) num_tables: u16,
  pub(crate) total_sfnt_size: u32,
  pub(crate) total_compressed_size: u32,
  pub(crate) major_version: u16,
  pub(crate) minor_version: u16,
  pub(crate) meta_offset: u32,
  pub(crate) meta_length: u32,
  pub(crate) meta_orig_length: u32,
  pub(crate) priv_offset: u32,
  pub(crate) priv_length: u32,
}

pub struct TableDirectory {
  pub(crate) flags: u8,
  pub(crate) tag: u32,
  pub(crate) orig_length: u32,
  pub(crate) transform_length: u32,
  pub(crate) transform_version: u32,
}

pub struct TransformedGlyfTable {
  pub(crate) reserved: u16,
  pub(crate) option_flags: u16,
  pub(crate) num_glyphs: u16,
  pub(crate) index_format: u16,
  pub(crate) nContourStreamSize: u32,
  pub(crate) nPointsStreamSize: u32,
  pub(crate) flagStreamSize: u32,
  pub(crate) glyphStreamSize: u32,
  pub(crate) compositeStreamSize: u32,
  pub(crate) bboxStreamSize: u32,
  pub(crate) instructionStreamSize: u32,
  pub(crate) nContourStream: Vec<i16>,
  pub(crate) nPointsStream: Vec<u32>,
  pub(crate) flagStream: Vec<u8>,
  pub(crate) glyphStream: Vec<u8>,
  pub(crate) compositeStream: Vec<u8>,
  pub(crate) bboxBitmap: Vec<u8>,
  pub(crate) instructionStream: Vec<u8>,
  pub(crate) overlapSimpleBitmap: Vec<u8>,
}