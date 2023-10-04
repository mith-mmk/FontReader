use super::*;
use bin_rs::reader::BinaryReader;
use std::io::SeekFrom;

#[derive(Debug, Clone)]
pub(crate) struct Script {
    pub(crate) script_tag: u32, // https://learn.microsoft.com/ja-jp/typography/opentype/spec/scripttags
    pub(crate) script_offset: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedScript {
    pub(crate) script_tag: u32,
    pub(crate) language_systems: Box<Vec<LanguageSystemRecord>>,
}

impl ParsedScript {
    pub(crate) fn parse<R: BinaryReader>(
        reader: &mut R,
        script: &Script,
    ) -> Result<Self, std::io::Error> {
        let offset = script.script_offset;
        reader.seek(SeekFrom::Start(offset as u64))?;
        let default_language_system_offset = reader.read_u16_be()?;
        let language_system_count = reader.read_u16_be()?;
        let mut language_systems = Vec::new();
        let mut language_system_tags = Vec::new();
        let mut language_system_offsets = Vec::new();
        for _ in 0..language_system_count {
            language_system_tags.push(reader.read_u32_be()?);
            language_system_offsets.push(reader.read_u16_be()?);
        }
        if default_language_system_offset > 0 {
            language_system_tags.insert(0, 0);
            language_system_offsets.insert(0, default_language_system_offset);
        }

        for (i, language_system_tag) in language_system_tags.iter().enumerate() {
            let language_system_tag = *language_system_tag;
            let language_system_offset = language_system_offsets[i];
            reader.seek(SeekFrom::Start(offset + language_system_offset as u64))?;

            let lookup_order_offset = reader.read_u16_be()?;
            let required_feature_index = reader.read_u16_be()?;
            let feature_index_count = reader.read_u16_be()?;
            let mut feature_indexes = Vec::new();
            for _ in 0..feature_index_count {
                feature_indexes.push(reader.read_u16_be()?);
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

        Ok(Self {
            script_tag: script.script_tag,
            language_systems: Box::new(language_systems),
        })
    }

    pub(crate) fn to_string(&self) -> String {
        let mut u8s = [0; 4];
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
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        _length: u32,
    ) -> Result<ScriptList, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let script_count = reader.read_u16_be()?;
        let mut scripts = Vec::new();
        for _ in 0..script_count {
            let script_tag = reader.read_u32_be()?;
            let script_offset = reader.read_u16_be()?;
            scripts.push(Script {
                script_tag,
                script_offset: script_offset as u64 + offset,
            });
        }
        let mut parced_scripts = Vec::new();

        for script in scripts.iter_mut() {
            let parsed_script = ParsedScript::parse(reader, script)?;
            parced_scripts.push(parsed_script)
        }

        Ok(Self {
            script_count,
            scripts: Box::new(parced_scripts),
        })
    }

    pub(crate) fn get_script(&self, script_tag: &[u8; 4]) -> Option<&ParsedScript> {
        let script_tag = u32::from_be_bytes(*script_tag);
        for script in self.scripts.iter() {
            if script.script_tag == script_tag {
                return Some(script);
            }
        }
        None
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = String::new();
        string += &format!("script count {}\n", self.script_count);
        for script in self.scripts.iter() {
            string += &format!("{}", script.to_string());
        }
        string
    }
}
