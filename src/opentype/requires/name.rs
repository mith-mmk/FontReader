// name is a table that contains font name information.

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum NameID {
    CopyRightNotice = 0,
    FontFamilyName = 1,
    FontSubfamilyName = 2,
    UniqueFontIdentifier = 3,
    FullFontName = 4,
    VersionString = 5,
    PostScriptName = 6,
    Trademark = 7,
    ManufacturerName = 8,
    Designer = 9,
    Description = 10,
    VendorURL = 11,
    DesignerURL = 12,
    LicenseDescription = 13,
    LicenseInfoURL = 14,
    Reserved = 15,
    TypographicFamilyName = 16,
    TypographicSubfamilyName = 17,
    CompatibleFullName = 18,
    SampleText = 19,
    PostScriptCIDFindfontName = 20,
    WWSFamilyName = 21,
    WWSSubfamilyName = 22,
    LightBackgroundPalette = 23,
    DarkBackgroundPalette = 24,
    VariationsPostScriptNamePrefix = 25,
    OTHER,
}

impl NameID {
    pub fn iter() -> [NameID; 27] {
        [
            Self::CopyRightNotice,
            Self::FontFamilyName,
            Self::FontSubfamilyName,
            Self::UniqueFontIdentifier,
            Self::FullFontName,
            Self::VersionString,
            Self::PostScriptName,
            Self::Trademark,
            Self::ManufacturerName,
            Self::Designer,
            Self::Description,
            Self::VendorURL,
            Self::DesignerURL,
            Self::LicenseDescription,
            Self::LicenseInfoURL,
            Self::Reserved,
            Self::TypographicFamilyName,
            Self::TypographicSubfamilyName,
            Self::CompatibleFullName,
            Self::SampleText,
            Self::PostScriptCIDFindfontName,
            Self::WWSFamilyName,
            Self::WWSSubfamilyName,
            Self::LightBackgroundPalette,
            Self::DarkBackgroundPalette,
            Self::VariationsPostScriptNamePrefix,
            Self::OTHER,
        ]
    }
}

use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    io::SeekFrom,
};

use bin_rs::reader::BinaryReader;
#[cfg(feature = "iconv")]
use iconv::Iconv;

use crate::opentype::platforms::{get_locale_to_language_id, PlatformID};

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

#[derive(Debug, Clone, Default)]
pub(crate) struct NameTable {
    // platform ID, Language ID, String
    pub(crate) default_namelist: HashMap<u16, String>,
    pub(crate) namelist: HashMap<(u16, u16), HashMap<u16, String>>,
}

impl NameTable {
    pub fn new(name: &NAME) -> Self {
        let name_records = name.name_records.clone();
        let mut default_namelist = HashMap::new();
        let mut namelist: HashMap<(u16, u16), HashMap<u16, String>> = HashMap::new();
        for name_record in name_records.iter() {
            let platform_id = name_record.platform_id;
            let language_id = name_record.language_id;
            let name_id = name_record.name_id;
            let string = name_record.string.clone();
            match platform_id {
                1 => {
                    if language_id == 0 {
                        default_namelist.insert(name_id, string);
                    } else {
                        let key = (platform_id, language_id);
                        if let Some(names) = namelist.get_mut(&key) {
                            names.insert(name_id, string);
                        } else {
                            let mut names = HashMap::new();
                            names.insert(name_id, string);
                            namelist.insert(key, names);
                        }
                    }
                }
                3 => {
                    if language_id == 0x409 {
                        default_namelist.insert(name_id, string);
                    } else {
                        let key = (platform_id, language_id);
                        if let Some(names) = namelist.get_mut(&key) {
                            names.insert(name_id, string);
                        } else {
                            let mut names = HashMap::new();
                            names.insert(name_id, string);
                            namelist.insert(key, names);
                        }
                    }
                }
                _ => {
                    default_namelist.insert(name_id, string);
                }
            }
        }
        Self {
            default_namelist,
            namelist,
        }
    }

    pub fn get_name_list(&self, locale: &String, platform_id: PlatformID) -> HashMap<u16, String> {
        let language_id = get_locale_to_language_id(locale, platform_id);
        let language_id = if let Some(language_id) = language_id {
            language_id
        } else {
            0
        };
        let key = (platform_id as u16, language_id);
        #[cfg(debug_assertions)]
        {
            println!("locale: {}", locale);
            println!("platform_id: {:?}", platform_id);
            println!("language_id: {:?}", language_id);
            println!("key: {:?}", key);
        }

        match self.namelist.get(&key) {
            Some(names) => names.clone(),
            None => self.default_namelist.clone(),
        }
    }

    pub fn get_name(&self, name_id: NameID, locale: &String, platform_id: PlatformID) -> String {
        let language_id = get_locale_to_language_id(locale, platform_id);
        let language_id = if let Some(language_id) = language_id {
            language_id
        } else {
            0
        };

        let key = (platform_id as u16, language_id);
        match self.namelist.get(&key) {
            Some(names) => {
                let name_id = name_id as u16;
                if let Some(name) = names.get(&name_id) {
                    name.clone()
                } else if let Some(name) = self.default_namelist.get(&name_id) {
                    name.clone()
                } else {
                    "".to_string()
                }
            }
            None => {
                let name_id = name_id as u16;
                if let Some(name) = self.default_namelist.get(&name_id) {
                    name.clone()
                } else {
                    "".to_string()
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NAME {
    pub(crate) version: u16,
    pub(crate) count: u16,
    pub(crate) storage_offset: u16,
    pub(crate) name_records: Box<Vec<NameRecord>>,
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
    pub(crate) fn new<R: BinaryReader>(file: &mut R, offest: u32, length: u32) -> Self {
        get_names(file, offest, length)
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "name\n".to_string();
        let version = format!("Version {}\n", self.version);
        string += &version;
        let count = format!("Count {}\n", self.count);
        string += &count;
        for name_record in self.name_records.iter() {
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
            string += &format!("Name String {} : ", name_record.string);
            string += "\n";
        }

        let lang_count = format!("Lang Count {}\n", self.lang_tag_count);
        string += &lang_count;

        for lang_tag_string in self.lang_tag_string.iter() {
            string += lang_tag_string;
            string += "\n";
        }

        string
    }

    pub fn get_family_name(&self) -> String {
        let mut family_name = "".to_string();
        for name_record in self.name_records.iter() {
            if name_record.name_id == 1 {
                family_name = name_record.string.clone();
                break;
            }
        }
        family_name
    }

    pub fn get_subfamily_name(&self) -> String {
        let mut subfamily_name = "".to_string();
        for name_record in self.name_records.iter() {
            if name_record.name_id == 2 {
                subfamily_name = name_record.string.clone();
                break;
            }
        }
        subfamily_name
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LangTagRecord {
    pub(crate) length: u16,
    pub(crate) offset: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct NameRecord {
    pub(crate) platform_id: u16,
    pub(crate) encoding_id: u16,
    pub(crate) language_id: u16,
    pub(crate) name_id: u16,
    pub(crate) length: u16,
    pub(crate) string_offset: u16,
    pub(crate) string: String,
}

fn get_names<R: BinaryReader>(file: &mut R, offest: u32, _length: u32) -> NAME {
    file.seek(SeekFrom::Start(offest as u64)).unwrap();
    let current_position = file.offset().unwrap();
    let version = file.read_u16_be().unwrap();
    let count = file.read_u16_be().unwrap();
    let storage_offset = file.read_u16_be().unwrap();
    let mut name_records = Vec::new();
    for _ in 0..count {
        let platform_id = file.read_u16_be().unwrap();
        let encoding_id = file.read_u16_be().unwrap();
        let language_id = file.read_u16_be().unwrap();
        let name_id = file.read_u16_be().unwrap();
        let length = file.read_u16_be().unwrap();
        let string_offset = file.read_u16_be().unwrap();
        name_records.push(NameRecord {
            platform_id,
            encoding_id,
            language_id,
            name_id,
            length,
            string_offset,
            string: "".to_string(),
        });
    }
    let mut lang_tag_count = 0;
    let mut lang_tag_record = Vec::new();
    if version > 0 {
        lang_tag_count = file.read_u16_be().unwrap();
        for _ in 0..lang_tag_count {
            let length = file.read_u16_be().unwrap();
            let offset = file.read_u16_be().unwrap();
            lang_tag_record.push(LangTagRecord { length, offset });
        }
    }
    let current_position = current_position + storage_offset as u64;
    // platform id = 0,3,4  utf-16be
    // platform id = 2       ASCII
    // platform id = 1 0 = ASCII 1 == UTF-16BE

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

        let string_offset = name_records[i].string_offset as u64 + current_position;
        file.seek(SeekFrom::Start(string_offset)).unwrap();
        match encoding_engine {
            EncodingEngine::UTF16BE => {
                let mut utf16s = Vec::new();
                for _ in 0..name_records[i].length / 2 {
                    let utf16 = file.read_u16_be().unwrap();
                    utf16s.push(utf16);
                }
                name_records[i].string = String::from_utf16(&utf16s).unwrap();
            }
            EncodingEngine::ASCII => {
                let ascii = file
                    .read_bytes_as_vec(name_records[i].length as usize)
                    .unwrap();
                let mut utf16s = Vec::new();
                for i in 0..ascii.len() {
                    utf16s.push(ascii[i] as u16);
                }
                name_records[i].string = String::from_utf16(&utf16s).unwrap();
            }
            _ => {
                name_records[i].string = "this encoding is not support".to_string();
            }
        }
    }

    let mut lang_tag_string = Vec::new();
    for i in 0..lang_tag_count {
        let string_offset = lang_tag_record[i as usize].offset + current_position as u16;
        file.seek(SeekFrom::Start(string_offset as u64)).unwrap();
        let string = file
            .read_utf16be_string(lang_tag_record[i as usize].length as usize)
            .unwrap();
        lang_tag_record[i as usize].offset = string_offset;
        lang_tag_record[i as usize].length = lang_tag_record[i as usize].length;
        lang_tag_string.push(string);
    }

    NAME {
        version,
        count,
        storage_offset,
        name_records: Box::new(name_records),
        lang_tag_count,
        lang_tag_records: Box::new(lang_tag_record),
        lang_tag_string: Box::new(lang_tag_string),
    }
}
