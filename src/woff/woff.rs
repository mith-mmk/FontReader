use bin_rs::reader::BinaryReader;
use miniz_oxide::inflate::decompress_to_vec_zlib;

#[derive(Debug, Clone)]
pub struct WOFFHeader {
  pub(crate) sfnt_version: u32,
  pub(crate) signature: u32,
  pub(crate) flavor: u32,
  pub(crate) length: u32,
  pub(crate) num_tables: u16,
  pub(crate) reserved: u16,
  pub(crate) total_sfnt_size: u32,
  pub(crate) major_version: u16,
  pub(crate) minor_version: u16,
  pub(crate) meta_offset: u32,
  pub(crate) meta_length: u32,
  pub(crate) meta_orig_length: u32,
  pub(crate) priv_offset: u32,
  pub(crate) priv_length: u32,
}

impl WOFFHeader {
  pub(crate) fn new<R: BinaryReader>(reader: &mut R) -> Self {
    let mut header = Self {
      sfnt_version: 0,
      signature: 0,
      flavor: 0,
      length: 0,
      num_tables: 0,
      reserved: 0,
      total_sfnt_size: 0,
      major_version: 0,
      minor_version: 0,
      meta_offset: 0,
      meta_length: 0,
      meta_orig_length: 0,
      priv_offset: 0,
      priv_length: 0,
    };
    header.signature = reader.read_u32().unwrap();
    header.flavor = reader.read_u32().unwrap();
    header.length = reader.read_u32().unwrap();
    header.num_tables = reader.read_u16().unwrap();
    header.reserved = reader.read_u16().unwrap();
    header.total_sfnt_size = reader.read_u32().unwrap();
    header.major_version = reader.read_u16().unwrap();
    header.minor_version = reader.read_u16().unwrap();
    header.meta_offset = reader.read_u32().unwrap();
    header.meta_length = reader.read_u32().unwrap();
    header.meta_orig_length = reader.read_u32().unwrap();
    header.priv_offset = reader.read_u32().unwrap();
    header.priv_length = reader.read_u32().unwrap();
    header
  }
}

#[derive(Debug, Clone)]
pub(crate) struct WOFFTableRecord {
    pub(crate) tag: u32,
    pub(crate) offset: u32,
    pub(crate) comp_length: u32,
    pub(crate) orig_length: u32,
    pub(crate) orig_checksum: u32,
}

impl WOFFTableRecord {
    pub(crate) fn new() -> WOFFTableRecord {
        WOFFTableRecord {
            tag: 0,
            offset: 0,
            comp_length: 0,
            orig_length: 0,
            orig_checksum: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WOFFTable {
    pub(crate) tag: u32,
    pub(crate) data: Vec<u8>,
}
impl WOFFTable {
    pub(crate) fn new() -> WOFFTable {
        WOFFTable {
            tag: 0,
            data: Vec::new(),
        }
    }
}


#[derive(Debug, Clone)]
pub(crate) struct WOFF {
    pub(crate) header: WOFFHeader,
    pub(crate) table_records: Vec<WOFFTableRecord>,
    pub(crate) metadata: Box<String>,
    pub(crate) private_data: Box<Vec<u8>>,
    pub(crate) tables: Vec<WOFFTable>,
}

impl WOFF{
    pub(crate) fn from<B:BinaryReader>(reader: &mut B, header: WOFFHeader) -> Self {
      let mut table_records = Vec::new();
      for _ in 0..header.num_tables {
          let mut table_record = WOFFTableRecord::new();
          table_record.tag = reader.read_u32().unwrap();
          table_record.offset = reader.read_u32().unwrap();
          table_record.comp_length = reader.read_u32().unwrap();
          table_record.orig_length = reader.read_u32().unwrap();
          table_record.orig_checksum = reader.read_u32().unwrap();
          let tag_str = crate::util::u32_to_string(table_record.tag);
          #[cfg(debug_assertions)]
          {
            print!("tag: {} {:08X} ", tag_str, table_record.tag);
            print!("offset: {:08X} ", table_record.offset);
            print!("comp_length: {} ", table_record.comp_length);
            print!("orig_length: {} ", table_record.orig_length);
            print!("orig_checksum: {:08X} ", table_record.orig_checksum);
            println!();
          }
          table_records.push(table_record);
      }
      // read metadata
      reader.seek(std::io::SeekFrom::Start(header.meta_offset as u64)).unwrap();
      let metadata =  if header.meta_length > 0 {
        let compress_metadata = reader.read_bytes_as_vec(header.meta_length as usize).unwrap();
        let metadata_bytes = decompress_to_vec_zlib(&compress_metadata).unwrap();
        String::from_utf8(metadata_bytes).unwrap()
      } else {
        "".to_string()
      };
      #[cfg(debug_assertions)]
      println!("metadata: {}", metadata);

      // read private data

      reader.seek(std::io::SeekFrom::Start(header.priv_offset as u64)).unwrap();
      let private_data = reader.read_bytes_as_vec(header.priv_length as usize).unwrap();

      // read table data
      let mut tables = Vec::new();
      for table_record in table_records.iter() {
        reader.seek(std::io::SeekFrom::Start(table_record.offset as u64)).unwrap();
        let mut table = WOFFTable::new();
        table.tag = table_record.tag;
        let mut table_data = reader.read_bytes_as_vec(table_record.comp_length as usize).unwrap();
        if table_record.comp_length != table_record.orig_length {
           table_data = decompress_to_vec_zlib(&table_data).unwrap();
        }
        table.data = table_data;
        tables.push(table);
      }

      WOFF {
        header,
        table_records,
        metadata: Box::new(metadata),
        private_data: Box::new(private_data),
        tables,
      }
    }

    pub(crate) fn new<R:BinaryReader>(reader: &mut R) -> Self{
      let header = WOFFHeader::new(reader);
      Self::from(reader, header)
    }

    pub fn get_metadata(&self) -> &str {
        &self.metadata
    }

    pub fn get_private_data(&self) -> &[u8] {
        &self.private_data
    }

}