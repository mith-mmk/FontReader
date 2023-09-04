use std::path::PathBuf;
use bin_rs::{reader::{BinaryReader, BytesReader, StreamReader}, Endian};
use crate::{opentype::OTFHeader, util::u32_to_string};
use crate::woff::woff::WOFFHeader;

// pub type F2DOT14 = i16;
pub type LONGDATETIME = i64;

//pub type Fixed = u32;
pub type FWORD = i16;
pub type UFWORD = u16;

#[cfg(target_feature = "impl")]
pub fn fixed_to_f32(value: Fixed) -> f32 {
    let integer = (value >> 16) as f32;
    let decimal = (value & 0xFFFF) as f32 / 65536.0;
    integer + decimal
}

#[cfg(target_feature = "impl")]
pub fn f2dot14_to_f32(value: F2DOT14) -> f32 {
    let integer = (value >> 14) as f32;
    let decimal = (value & 0x3FFF) as f32 / 16384.0;
    integer + decimal
}


pub fn longdatetime_to_string(value: &LONGDATETIME) -> String {
    /* LONGDATETIME Date and time represented in number of seconds since 12:00 midnight,
    January 1, 1904, UTC. The value is represented as a signed 64-bit integer. */
    let seconds = value % 60;
    let minutes = (value / 60) % 60;
    let hours = (value / 3600) % 24;
    let days = (value / 86400) % 365;    
    let years = (value / 31536000) + 1904;
    let leap_year = if years % 4 == 0 {
        if years % 100 == 0 {
            if years % 400 == 0 {
                1
            } else { 0 }
        } else { 1 }    
    } else { 0 };

    let monthes = [31, 28 + leap_year, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0;
    let mut days = days;
    for i in 0..monthes.len() {
        if days < monthes[i] {
            month = i + 1;
            break;
        } else {
            days -= monthes[i];
        }
    }  

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}Z", years, month, days, hours, minutes, seconds)
}



/*
// https://docs.microsoft.com/en-us/typography/opentype/spec/otff

type Tag = u32;
type Offset16 = u16;
type Offset32 = u32;
type uint8 = u8;
type uint16 = u16;
type uint32 = u32;
type uint64 = u64;
type int8 = i8;
type int16 = i16;
type int32 = i32;
type int64 = i64;
*/

#[derive(Debug, Clone)]
pub enum FontHeaders {
    TTF(TTFHeader),
    OTF(OTFHeader),
    WOFF(WOFFHeader),
    WOFF2(WOFF2Header),
    Unknown
}



impl FontHeaders {
  pub fn to_string(&self) -> String {
        match self {
            FontHeaders::TTF(header) => {
                let mut string = String::new();
                string.push_str("TTF:");
                // ascii string
                string.push_str(u32_to_string(header.sfnt_version).as_str());
                string.push_str(&" major version: ".to_string());
                string.push_str(&header.major_version.to_string());
                string.push_str(&" minor version: ".to_string());
                string.push_str(&header.minor_version.to_string());
                string.push_str(&" num fonts: ".to_string());
                string.push_str(&header.num_fonts.to_string());
                string.push_str(&" table directory:\n".to_string());
                for table in header.table_directory.iter() {
                    string.push_str(&table.to_string());
                    string.push_str(&"\n".to_string());
                }
                if header.major_version >= 2 {
                    string.push_str(&" ul_dsig_tag: ".to_string());
                    string.push_str(&header.ul_dsig_tag.to_string());
                    string.push_str(&" ul_dsig_length: ".to_string());
                    string.push_str(&header.ul_dsig_length.to_string());
                    string.push_str(&" ul_dsig_offset: ".to_string());
                    string.push_str(&header.ul_dsig_offset.to_string());
                }
                string

            },
            FontHeaders::OTF(header) => {
                let mut string = String::new();
                string.push_str("OTF: ");
                string.push_str(u32_to_string(header.sfnt_version).as_str());
                let bytes = header.sfnt_version.to_be_bytes();
                let mut sfnt_version = String::new();
                for byte in bytes.iter() {
                    sfnt_version.push_str(&format!("{:02X}", byte));
                }
                string.push_str(&sfnt_version);
                string.push_str(&" num tables: ".to_string());
                string.push_str(&header.num_tables.to_string());
                string.push_str(&" search range: ".to_string());
                string.push_str(&header.search_range.to_string());
                string.push_str(&" entry selector: ".to_string());
                string.push_str(&header.entry_selector.to_string());
                string.push_str(&" range shift: ".to_string());
                string.push_str(&header.range_shift.to_string());
                string.push_str(&" table records:\n".to_string());
                #[cfg(debug_assertions)]
                for table in header.table_records.iter() {
                    string.push_str(&table.clone().to_string());
                    string.push_str(&"\n".to_string());
                }
                string
            },
            FontHeaders::WOFF(header) => {
                let mut string = String::new();
                string.push_str("WOFF: ");
                string.push_str(u32_to_string(header.sfnt_version).as_str());
                let bytes = header.signature.to_be_bytes();
                let mut signature = String::new();
                for byte in bytes.iter() {
                    signature.push_str(&format!("{:02X}", byte));
                }
                string.push_str(&signature);
                string.push_str(&"\n flavor: ".to_string());
                let bytes = header.flavor.to_be_bytes();
                let mut flavor = String::new();
                for byte in bytes.iter() {
                    flavor.push_str(&format!("{:02X}", byte));
                }
                string.push_str(&flavor);
                string.push_str(&"\n length: ".to_string());
                string.push_str(&header.length.to_string());
                string.push_str(&"\n num tables: ".to_string());
                string.push_str(&header.num_tables.to_string());
                string.push_str(&"\n reserved: ".to_string());
                string.push_str(&header.reserved.to_string());
                string.push_str(&"\n total sfnt size: ".to_string());
                string.push_str(&header.total_sfnt_size.to_string());
                string.push_str(&"\n major version: ".to_string());
                string.push_str(&header.major_version.to_string());
                string.push_str(&"\n minor version: ".to_string());
                string.push_str(&header.minor_version.to_string());
                string.push_str(&"\n meta offset: ".to_string());
                string.push_str(&header.meta_offset.to_string());
                string.push_str(&"\n meta length: ".to_string());
                string.push_str(&header.meta_length.to_string());
                string.push_str(&"\n meta orig length: ".to_string());
                string.push_str(&header.meta_orig_length.to_string());
                string.push_str(&"\n priv offset: ".to_string());
                string.push_str(&header.priv_offset.to_string());
                string.push_str(&"\n priv length: ".to_string());
                string.push_str(&header.priv_length.to_string());
                string
            },
            FontHeaders::WOFF2(header) => {
                let mut string = String::new();
                string.push_str("WOFF2: ");
                string.push_str(u32_to_string(header.sfnt_version).as_str());
                let bytes = header.signature.to_be_bytes();
                let mut signature = String::new();
                for byte in bytes.iter() {
                    signature.push_str(&format!("{:02X}", byte));
                }
                string.push_str(&signature);
                string.push_str(&" flavor: ".to_string());
                let bytes = header.flavor.to_be_bytes();
                let mut flavor = String::new();
                for byte in bytes.iter() {
                    flavor.push_str(&format!("{:02X}", byte));
                }
                string.push_str(&flavor);
                string.push_str(&" length: ".to_string());
                string.push_str(&header.length.to_string());
                string.push_str(&" num tables: ".to_string());
                string.push_str(&header.num_tables.to_string());
                string.push_str(&" reserved: ".to_string());
                string.push_str(&header.reserved.to_string());
                string.push_str(&" total sfnt size: ".to_string());
                string.push_str(&header.total_sfnt_size.to_string());
                string.push_str(&" major version: ".to_string());
                string.push_str(&header.major_version.to_string());
                string.push_str(&" minor version: ".to_string());
                string.push_str(&header.minor_version.to_string());
                string.push_str(&" meta offset: ".to_string());
                string.push_str(&header.meta_offset.to_string());
                string.push_str(&" meta length: ".to_string());
                string.push_str(&header.meta_length.to_string());
                string.push_str(&" meta orig length: ".to_string());
                string.push_str(&header.meta_orig_length.to_string());
                string.push_str(&" priv offset: ".to_string());
                string.push_str(&header.priv_offset.to_string());
                string.push_str(&" priv length: ".to_string());
                string.push_str(&header.priv_length.to_string());
                string
            },
            FontHeaders::Unknown => {
                format!("Unknown")
            },
        }
    }       
}



#[derive(Debug, Clone)]
pub struct TTFHeader {
    pub(crate) sfnt_version: u32,
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) num_fonts: u32,
    pub(crate) table_directory: Box<Vec<u32>>,
    // Version2
    pub(crate) ul_dsig_tag: u32,
    pub(crate) ul_dsig_length: u32,
    pub(crate) ul_dsig_offset: u32,
}





#[derive(Debug, Clone)]
pub struct WOFF2Header {
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

pub fn get_font_type_from_file(filename: &PathBuf) -> FontHeaders {
    let file = std::fs::File::open(filename).unwrap();
    let reader = std::io::BufReader::new(file);
    let mut file = StreamReader::new(reader);
    get_font_type(&mut file)
}

pub fn get_font_type<B: BinaryReader>(file: &mut B) -> FontHeaders {
    let mut buffer = [0; 4];
    file.set_endian(Endian::BigEndian);
    file.read_bytes(&mut buffer).unwrap();
    let sfnt_version:u32 = u32::from_be_bytes(buffer);
    let font_type = match &buffer {
        b"ttcf" => {
            let major_version = file.read_u16_be().unwrap();
            let minor_version = file.read_u16_be().unwrap();
            let num_fonts = file.read_u32_be().unwrap();
            let mut table_directory = Vec::new();
            for _ in 0..num_fonts {
                let offset = file.read_u32_be().unwrap();
                table_directory.push(offset);
            }
            let mut ul_dsig_tag = 0;
            let mut ul_dsig_length = 0;
            let mut ul_dsig_offset = 0;
            if major_version >= 2 {
                ul_dsig_tag = file.read_u32_be().unwrap();
                ul_dsig_length = file.read_u32_be().unwrap();
                ul_dsig_offset = file.read_u32_be().unwrap();
            }
            let fontheader = {
                TTFHeader {
                    sfnt_version,
                    major_version,
                    minor_version,
                    num_fonts: num_fonts,
                    table_directory: Box::new(table_directory),
                    ul_dsig_tag,
                    ul_dsig_length,
                    ul_dsig_offset,                    
                }
            };  
            FontHeaders::TTF(fontheader)
        },
        // if 0x00010000 -> OTF

        b"\x00\x01\x00\x00" | b"OTTO" => {
            let num_tables = file.read_u16_be().unwrap();
            let search_range = file.read_u16_be().unwrap();
            let entry_selector = file.read_u16_be().unwrap();
            let range_shift = file.read_u16_be().unwrap();
            let mut table_records = Vec::new();
            for _ in 0..num_tables {
                let table_tag = file.read_u32_be().unwrap();
                let check_sum = file.read_u32_be().unwrap();
                let offset = file.read_u32_be().unwrap();
                let length = file.read_u32_be().unwrap();
                table_records.push(crate::opentype::TableRecord {
                    table_tag,
                    check_sum,
                    offset,
                    length,
                });
            }

            let fontheader = OTFHeader {
                sfnt_version,
                num_tables,
                search_range,
                entry_selector,
                range_shift,
                table_records: Box::new(table_records), 
            };
            FontHeaders::OTF(fontheader)
        },
        // 0
        b"wOFF" => {
            let signature = file.read_u32_be().unwrap();
            let flavor = file.read_u32_be().unwrap();
            let length = file.read_u32_be().unwrap();
            let num_tables = file.read_u16_be().unwrap();
            let reserved = file.read_u16_be().unwrap();
            let total_sfnt_size = file.read_u32_be().unwrap();
            let major_version = file.read_u16_be().unwrap();
            let minor_version = file.read_u16_be().unwrap();
            let meta_offset = file.read_u32_be().unwrap();
            let meta_length = file.read_u32_be().unwrap();
            let meta_orig_length = file.read_u32_be().unwrap();
            let priv_offset = file.read_u32_be().unwrap();
            let priv_length = file.read_u32_be().unwrap();            
            FontHeaders::WOFF(WOFFHeader {
                sfnt_version,
                signature,
                flavor,
                length,
                num_tables,
                reserved,
                total_sfnt_size,
                major_version,
                minor_version,
                meta_offset,
                meta_length,
                meta_orig_length,
                priv_offset,
                priv_length,
            })
        },
        b"wOF2" => {
            let signature = file.read_u32_be().unwrap();
            let flavor = file.read_u32_be().unwrap();
            let length = file.read_u32_be().unwrap();
            let num_tables = file.read_u16_be().unwrap();
            let reserved = file.read_u16_be().unwrap();
            let total_sfnt_size = file.read_u32_be().unwrap();
            let major_version = file.read_u16_be().unwrap();
            let minor_version = file.read_u16_be().unwrap();
            let meta_offset = file.read_u32_be().unwrap();
            let meta_length = file.read_u32_be().unwrap();
            let meta_orig_length = file.read_u32_be().unwrap();
            let priv_offset = file.read_u32_be().unwrap();
            let priv_length = file.read_u32_be().unwrap();
            FontHeaders::WOFF2(WOFF2Header {
                sfnt_version,
                signature,
                flavor,
                length,
                num_tables,
                reserved,
                total_sfnt_size,
                major_version,
                minor_version,
                meta_offset,
                meta_length,
                meta_orig_length,
                priv_offset,
                priv_length,
            })
        },
        _ => FontHeaders::Unknown,
    };
    font_type
}


pub fn get_font_type_from_buffer(fontdata: &[u8]) -> FontHeaders {
    let file = &mut BytesReader::new(fontdata);
    get_font_type(file)
}


