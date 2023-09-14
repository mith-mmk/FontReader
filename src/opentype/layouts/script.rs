use std::io::SeekFrom;
use super::*;
use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) struct Script {
    pub(crate) script_tag: u32, // https://learn.microsoft.com/ja-jp/typography/opentype/spec/scripttags
    pub(crate) script_offset: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedScript {
    pub(crate) script_tag: u32,
    pub(crate) default_language_system_offset: u16,
    pub(crate) language_systems: Box<Vec<LanguageSystemRecord>>,
}

impl ParsedScript {
  pub(crate) fn parse<R: BinaryReader>(reader: &mut R, script: &Script) -> Self {
      let offset = script.script_offset;
      reader.seek(SeekFrom::Start(offset as u64)).unwrap();
      let default_language_system_offset = reader.read_u16_be().unwrap();
      let language_system_count = reader.read_u16_be().unwrap();
      let mut language_systems = Vec::new();
      for _ in 0..language_system_count {
          let language_system_tag = reader.read_u32_be().unwrap();
          let language_system_offset = reader.read_u16_be().unwrap(); // todo!
          let lookup_order_offset = reader.read_u16_be().unwrap();
          let required_feature_index = reader.read_u16_be().unwrap();
          let feature_index_count = reader.read_u16_be().unwrap();
          let mut feature_indexes = Vec::new();
          for _ in 0..feature_index_count {
              feature_indexes.push(reader.read_u16_be().unwrap());
          }
          language_systems.push(LanguageSystemRecord {
              language_system_tag,
              language_system: LanguageSystem {
                  lookup_order_offset,
                  required_feature_index,
                  feature_index_count,
                  feature_indexes,
              },
          });
      }


      Self {
          script_tag: script.script_tag,
          default_language_system_offset,
          language_systems: Box::new(language_systems),
      }

  }

  pub(crate) fn to_string(&self) -> String {
      let mut u8s = [0;4];
      for i in 0..4 {
          u8s[3 - i] = (self.script_tag >> (i * 8)) as u8;
      }
      let tag = unsafe { std::str::from_utf8_unchecked(&u8s) };
      let mut string = format!("Script: {} {}\n", tag, self.language_systems.len());
      for language_system in self.language_systems.iter() {
          string += &format!("{}", language_system.to_string());
      }
      string
  }
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptList {
  pub(crate) script_count: u16,
  pub(crate) scripts: Box<Vec<ParsedScript>>,
}

impl ScriptList {
  pub(crate) fn new<R: BinaryReader>(reader: &mut R, script_list_offset: u64, length: u32) -> ScriptList {
      reader.seek(SeekFrom::Start(script_list_offset as u64)).unwrap();
      let script_count = reader.read_u16_be().unwrap();
      let mut scripts = Vec::new();
      for _ in 0..script_count {
          let script_tag = reader.read_u32_be().unwrap();
          let script_offset = reader.read_u16_be().unwrap();
          scripts.push(Script {
              script_tag,
              script_offset: script_offset as u64 + script_list_offset,
          });
      }
      let mut parced_scripts = Vec::new();

      for script in scripts.iter_mut() {
          let parsed_script = ParsedScript::parse(reader, script);
          parced_scripts.push(parsed_script)
      }

      Self {
          script_count,
          scripts: Box::new(parced_scripts),
      }
  }

  pub(crate) fn to_string(&self) -> String {
      let mut string = String::new();
      string += &format!("script count {} :{}\n", self.script_count, self.scripts.len());
      for script in self.scripts.iter() {
          string += &format!("{}",script.to_string());
      }
      string
  }
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptRecord {
  pub(crate) script_tag: u32,
  pub(crate) script: ParsedScript,
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptTable {
  pub(crate) default_language_system_offset: u16,
  pub(crate) language_system_count: u16,
  pub(crate) language_systems: Box<Vec<LanguageSystem>>,
}
