

use std::{fmt::{Display, Formatter, self}, io::SeekFrom};

use bin_rs::reader::BinaryReader;
#[cfg(feature="iconv")]
use iconv::Iconv;

enum EncodingEngine {
  UTF16BE,
  ASCII,
  ShiftJIS,
  PRC,
  Big5,
  Wansung,
  Johab,
  Unknown,
}

#[derive(Debug, Clone)]
pub(crate) struct NAME {
  pub(crate) version: u16,
  pub(crate) count: u16,
  pub(crate) storage_offset: u16,
  pub(crate) name_records: Box<Vec<NameRecord>>,
  pub(crate) name_string: Box<Vec<String>>,
  // above V0
  // under V1
  pub(crate) lang_tag_count: u16,
  pub(crate) lang_tag_records: Box<Vec<LangTagRecord>>,
  pub(crate) lang_tag_string: Box<Vec<String>>,
}

impl Display for NAME {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl NAME {
  pub(crate) fn new<R:BinaryReader>(file: &mut R, offest: u32, length: u32) -> Self {
    get_names(file, offest, length)
  }

  pub(crate) fn to_string(&self) -> String {
    let mut string = "name\n".to_string();
    let version = format!("Version {}\n", self.version);
    string += &version;
    let count = format!("Count {}\n", self.count);
    string += &count;
    for (i, name_string) in self.name_string.iter().enumerate() {
      let name_record = &self.name_records[i];
      let platform_id = format!("Platform ID {}\n", name_record.platform_id);
      string += &platform_id;
      let encoding_id = format!("Encoding ID {}\n", name_record.encoding_id);
      string += &encoding_id;
      let language_id = format!("Language ID {}\n", name_record.language_id);
      string += &language_id;
      let name_id = format!("Name ID {}\n", name_record.name_id);
      string += &name_id;
      let length = format!("Length {}\n", name_record.length);
      string += &length;
      let string_offset = format!("String Offset {}\n", name_record.string_offset);
      string += &string_offset;
      string += &format!("Name String {} : ", i);
      string += &name_string;
      string += "\n";
    }

    let lang_count = format!("Lang Count {}\n", self.lang_tag_count);
    string += &lang_count;

    for lang_tag_string in self.lang_tag_string.iter() {
      string += &lang_tag_string;
      string += "\n";
    }


    string   
  }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LangTagRecord {
  pub(crate) length: u16,
  pub(crate) offset: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct NameRecord{
  pub(crate) platform_id: u16,
  pub(crate) encoding_id: u16,
  pub(crate) language_id: u16,
  pub(crate) name_id: u16,
  pub(crate) length: u16,
  pub(crate) string_offset: u16,
}

fn get_names<R: BinaryReader>(file: &mut R, offest: u32, length: u32) -> NAME {
  file.seek(SeekFrom::Start(offest as u64)).unwrap();
  let version = file.read_u16().unwrap();
  let count = file.read_u16().unwrap();
  let storage_offset = file.read_u16().unwrap();
  let mut name_records = Vec::new();
  for _ in 0..count {
    let platform_id = file.read_u16().unwrap();
    let encoding_id = file.read_u16().unwrap();
    let language_id = file.read_u16().unwrap();
    let name_id = file.read_u16().unwrap();
    let length = file.read_u16().unwrap();
    let string_offset = file.read_u16().unwrap();
    name_records.push(NameRecord {
      platform_id,
      encoding_id,
      language_id,
      name_id,
      length,
      string_offset,
    });
  }
  let mut lang_tag_count = 0;
  let mut lang_tag_record = Vec::new();
  if version > 0 {
    lang_tag_count = file.read_u16().unwrap();
    for _ in 0..lang_tag_count {
      let length = file.read_u16().unwrap();
      let offset = file.read_u16().unwrap();
      lang_tag_record.push(LangTagRecord {
        length,
        offset
      });
    }
  }
  let current_position = file.offset().unwrap();
  // platform id = 0,3,4  utf-16be
  // platform id = 2       ASCII 
  // platform id = 1 0 = ASCII 1 == UTF-16BE

  let mut name_string = Vec::new();
  for i in 0..count as usize {
    let encoding_engine = match name_records[i].platform_id {
      0 | 3 | 4 => EncodingEngine::UTF16BE,
      2 => EncodingEngine::ASCII,
      1 => match name_records[i].encoding_id {
        0 => EncodingEngine::ASCII,
        _ => EncodingEngine::UTF16BE,
      },
      _ => EncodingEngine::Unknown,
    };


    let string_offset = name_records[i].string_offset as u64 + current_position ;
    file.seek(SeekFrom::Start(string_offset as u64)).unwrap();
    match encoding_engine {
      EncodingEngine::UTF16BE => {
        let string = file.read_utf16_string(name_records[i].length as usize).unwrap();
        name_string.push(string);    
      }
      EncodingEngine::ASCII => {
        let string = file.read_ascii_string(name_records[i].length as usize).unwrap();
        name_string.push(string.to_string());
      }
      _ => {
        #[cfg(feature="iconv")]
        {
        let mut u8 = Vec::new();
        let u8 = file.read_bytes_as_vec(name_records[i].length as usize).unwrap();
        let string = if encoding_engine == EncodingEngine::ShiftJIS {
          iconv::decode(&u8, "CP932").unwrap()
        } else if encoding_engine == EncodingEngine::Big5 {
          iconv::decode(&u8, "BIG5").unwrap()
        } else if encoding_engine == EncodingEngine::Wansung {
          iconv::decode(&u8, "EUC-KR").unwrap()
        } else if encoding_engine == EncodingEngine::Johab {
          iconv::decode(&u8, "JOHAB").unwrap()
        } else {
          "not support".to_string()
        };
        name_string.push(string);
        }
        #[cfg(not(feature="iconv"))]
          name_string.push("this encoding is not support".to_string());
      }
    }
  }

  let mut lang_tag_string = Vec::new();
  for i in 0..lang_tag_count {
    let string_offset = lang_tag_record[i as usize].offset + current_position as u16;
    file.seek(SeekFrom::Start(string_offset as u64)).unwrap();
    let string = file.read_utf16_string(lang_tag_record[i as usize].length as usize).unwrap();
    lang_tag_record[i as usize].offset = string_offset;
    lang_tag_record[i as usize].length = lang_tag_record[i as usize].length;
    lang_tag_string.push(string);
  }

  NAME {
    version,
    count,
    storage_offset,
    name_records: Box::new(name_records),
    name_string: Box::new(name_string),
    lang_tag_count,
    lang_tag_records: Box::new(lang_tag_record),
    lang_tag_string: Box::new(lang_tag_string),
  }
}
