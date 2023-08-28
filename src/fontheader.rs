use std::borrow::BorrowMut;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;
use crate::cmap::{self, EncodingRecord};

type Fixed = u32;
type FWORD = i16;
type UFWORD = u16;
type F2DOT14 = i16;
type LONGDATETIME = i64;
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

fn u32_to_string(value: u32) -> String {
    let bytes = value.to_be_bytes();
    // bytes to string
    let mut string = String::new();
    for byte in bytes.iter() {
        string.push(char::from(*byte));
    }
    string
}

impl FontHeaders {
  pub fn to_string(&self) -> String {
        match self {
            FontHeaders::TTF(header) => {
                let mut string = String::new();
                string.push_str("TTF: ");
                // ascii string
                string.push_str(u32_to_string(header.sfnt_version).as_str());
                string.push_str(&" major version: ".to_string());
                string.push_str(&header.major_version.to_string());
                string.push_str(&" minor version: ".to_string());
                string.push_str(&header.minor_version.to_string());
                string.push_str(&" num fonts: ".to_string());
                string.push_str(&header.num_fonts.to_string());
                string.push_str(&" table directory: ".to_string());
                for table in header.table_directory.iter() {
                    string.push_str(&table.to_string());
                    string.push_str(&" ".to_string());
                }
                if header.major_version >= 2 {
                    string.push_str(&" ul_dsig_sfnt_version: ".to_string());
                    string.push_str(&header.ul_dsig_sfnt_version.to_string());
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
                string.push_str(&" table records: ".to_string());
                #[cfg(debug_assertions)]
                for table in header.table_records.iter() {
                    string.push_str(&table.clone().to_string());
                    string.push_str(&" ".to_string());
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
pub struct TableRecord {
    table_tag: u32,
    check_sum: u32,
    offset: u32,
    length: u32,
}

impl TableRecord {
    fn to_string(self) -> String {
        let mut string = String::new();
        string.push_str(&"table tag: ".to_string());
        string.push_str(u32_to_string(self.table_tag).as_str());
        string.push_str(&" check sum: ".to_string());
        // hex digit
        string.push_str(&format!("{:08X}", self.check_sum));
        string.push_str(&" offset: ".to_string());
        string.push_str(&format!("{:08X}", self.offset));
        string.push_str(&" length: ".to_string());
        string.push_str(&format!("{:08X}", self.length));
        string
    }
}


#[derive(Debug, Clone)]
pub struct TTFHeader {
    sfnt_version: u32,
    major_version: u16,
    minor_version: u16,
    num_fonts: u32,
    table_directory: Box<Vec<u32>>,
    // Version2
    ul_dsig_sfnt_version: u32,
    ul_dsig_length: u32,
    ul_dsig_offset: u32,
}


#[derive(Debug, Clone)]
pub struct OTFHeader {
    sfnt_version: u32,
    num_tables: u16,
    search_range: u16,
    entry_selector: u16,
    range_shift: u16,
    table_records: Box<Vec<TableRecord>>,
}



#[derive(Debug, Clone)]
pub struct WOFFHeader {
    sfnt_version: u32,
    signature: u32,
    flavor: u32,
    length: u32,
    num_tables: u16,
    reserved: u16,
    total_sfnt_size: u32,
    major_version: u16,
    minor_version: u16,
    meta_offset: u32,
    meta_length: u32,
    meta_orig_length: u32,
    priv_offset: u32,
    priv_length: u32,
}

#[derive(Debug, Clone)]
pub struct WOFF2Header {
    sfnt_version: u32,
    signature: u32,
    flavor: u32,
    length: u32,
    num_tables: u16,
    reserved: u16,
    total_sfnt_size: u32,
    major_version: u16,
    minor_version: u16,
    meta_offset: u32,
    meta_length: u32,
    meta_orig_length: u32,
    priv_offset: u32,
    priv_length: u32,
}

/*
uint32
CalcTableChecksum(uint32 *Table, uint32 Length)
{
uint32 Sum = 0L;
uint32 *Endptr = Table+((Length+3) & ~3) / sizeof(uint32);
while (Table < EndPtr)
    Sum += *Table++;
return Sum;
} */

fn check_sum(table: Vec<u8>) -> u32 {
    let mut sum = 0;
    for i in 0..table.len()/4 {
        let offset = i * 4;
        let mut bytes = [0; 4];
        for j in 0..4 {
            bytes[j] = table[offset + j];
        }
        let number: u32 = u32::from_be_bytes(bytes);      
        sum += number;
    }
    let remain = table.len() % 4;
    if remain > 0 {
        let mut bytes = [0; 4];
        for i in 0..remain {
            bytes[i] = table[table.len() - remain + i];
        }
        let number: u32 = u32::from_be_bytes(bytes);
        sum += number;
    }
    sum
}

enum FontTable {
// required
    CMAP(cmap::CMAP),
    HEAD(HEAD),
    HHEA(HHEA),
    HMTX,
    MAXP,
    NAME,
    OS2,
    POST,
// tables related to TrueType outlines
    CVT,
    FPGM,
    GLYF,
    LOCA,
    PREP,
    GASP,
// tables related to CFF outlines
    CFF,
    CFF2,
    VORG,
// SVG
    SVG,
// Bitmap Glyphs
    EBDT,
    EBLC,
    EBSV,
// Color Bitmap Glyphs
    CBDT,
    CBLC,
    COLR,
    CPAL,
// Advanced Typographics
    BASE,
    GDEF,
    GPOS,
    GSUB,
    JSTF,
    MATH,
    MERG,
    PROP,
    ZWJ,
// OpenType Font Variant
    AVAR,
    CVAR,
    FVAR,
    GVAR,
    HVAR,
    MVAR,
    STAT,
    VVAR,
// Color Fonts
    // COLR,
    // CPAL,
    // CBDT,
    // CBLC,
    SBIX,
    // SVG,
// Other OpenType Tables
    DISG,
    HDMX,
    KERN,
    LTSH,
//    MERG,
    META,
//    STAT,
    PCLT,
    VDMX,
    VHEA,
    VMTX,
    UNKNOWN
}




struct HEAD {
    major_version: u16,
    minor_version: u16,
    font_revision: u32,
    check_sum_adjustment: u32,
    magic_number: u32,
    flags: u16,
    units_per_em: u16,
    created: LONGDATETIME,
    modified: LONGDATETIME,
    x_min: i16,
    y_min: i16,
    x_max: i16,
    y_max: i16,
    mac_style: u16,
    lowest_rec_ppem: u16,
    font_direction_hint: i16,
    index_to_loc_format: i16,
    glyph_data_format: i16,
}

struct HHEA {
    major_version: u16,
    minor_version: u16,
    ascender: FWORD,
    descender: FWORD,
    line_gap: FWORD,
    advance_width_max: UFWORD,
    min_left_side_bearing: FWORD,
    min_right_side_bearing: FWORD,
    x_max_extent: FWORD,
    caret_slope_rise: i16,
    caret_slope_run: i16,
    caret_offset: i16,
    reserved1: i16,
    reserved2: i16,
    reserved3: i16,
    reserved4: i16,
    metric_data_format: i16,
    number_of_hmetrics: u16,
}

struct HMTX {
    advance_width: Box<Vec<u16>>,
    left_side_bearing: Box<Vec<i16>>,
} 


pub fn get_font_type(fontdata: &[u8]) -> FontHeaders {
    let mut buffer = [0; 4];
    let mut file = std::io::Cursor::new(fontdata);    
    file.read(&mut buffer).unwrap();
    let sfnt_version:u32 = u32::from_be_bytes(buffer);
    let font_type = match &buffer {
        b"ttcf" => {
            let mut major_version = [0; 2];
            file.read(&mut major_version).unwrap();
            let mut minor_version = [0; 2];
            let version = u16::from_be_bytes(major_version);
            file.read(&mut minor_version).unwrap();
            let mut num_fonts = [0; 4];
            file.read(&mut num_fonts).unwrap();
            let mut table_directory = Vec::new();
            for _ in 0..u32::from_be_bytes(num_fonts) {
                let mut offset = [0; 4];
                file.read(&mut offset).unwrap();
                table_directory.push(u32::from_be_bytes(offset));
            }
            let mut ul_dsig_sfnt_version = [0; 4];
            let mut ul_dsig_length: [u8; 4] = [0; 4];
            let mut ul_dsig_offset = [0; 4];
            if version >= 2 {
                file.read(&mut ul_dsig_sfnt_version).unwrap();
                file.read(&mut ul_dsig_length).unwrap();
                file.read(&mut ul_dsig_offset).unwrap();
            }
            let fontheader = {
                TTFHeader {
                    sfnt_version : sfnt_version,
                    major_version: version,
                    minor_version: u16::from_be_bytes(minor_version),
                    num_fonts: u32::from_be_bytes(num_fonts),
                    table_directory: Box::new(table_directory),
                    ul_dsig_sfnt_version: u32::from_be_bytes(ul_dsig_sfnt_version),
                    ul_dsig_length: u32::from_be_bytes(ul_dsig_length),
                    ul_dsig_offset: u32::from_be_bytes(ul_dsig_offset),
                }
            };  
            FontHeaders::TTF(fontheader)
        },
        // if 0x00010000 -> OTF

        b"\x00\x01\x00\x00" | b"OTTO" => {
            let mut num_tables = [0; 2];
            file.read(&mut num_tables).unwrap();
            let mut search_range = [0; 2];
            file.read(&mut search_range).unwrap();
            let mut entry_selector = [0; 2];
            file.read(&mut entry_selector).unwrap();
            let mut range_shift = [0; 2];
            file.read(&mut range_shift).unwrap();
            let mut table_directory = [0; 16];
            file.read(&mut table_directory).unwrap();
            let mut table_data = [0; 4];
            file.read(&mut table_data).unwrap();
            let mut checksum = [0; 4];
            file.read(&mut checksum).unwrap();
            let mut offset = [0; 4];
            file.read(&mut offset).unwrap();
            let mut length = [0; 4];
            file.read(&mut length).unwrap();
            let mut table_records = Vec::new();
            for _ in 0..u16::from_be_bytes(num_tables) {
                let mut table_tag = [0; 4];
                file.read(&mut table_tag).unwrap();
                let mut check_sum = [0; 4];
                file.read(&mut check_sum).unwrap();
                let mut offset = [0; 4];
                file.read(&mut offset).unwrap();
                let mut length = [0; 4];
                file.read(&mut length).unwrap();
                // debug
                let table_record = TableRecord {
                    table_tag: u32::from_be_bytes(table_tag),
                    check_sum: u32::from_be_bytes(check_sum),
                    offset: u32::from_be_bytes(offset),
                    length: u32::from_be_bytes(length),
                };
                table_records.push(table_record);
            }

            let fontheader = OTFHeader {
                sfnt_version : sfnt_version,
                num_tables: u16::from_be_bytes(num_tables),
                search_range: u16::from_be_bytes(search_range),
                entry_selector: u16::from_be_bytes(entry_selector),
                range_shift: u16::from_be_bytes(range_shift),
                table_records: Box::new(table_records),
            };
            FontHeaders::OTF(fontheader)
        },
        // 0
        b"wOFF" => {
            let mut signature = [0; 4];
            file.read(&mut signature).unwrap();
            let mut flavor = [0; 4];
            file.read(&mut flavor).unwrap();
            let mut length = [0; 4];
            file.read(&mut length).unwrap();
            let mut num_tables = [0; 2];
            file.read(&mut num_tables).unwrap();
            let mut reserved = [0; 2];
            file.read(&mut reserved).unwrap();
            let mut total_sfnt_size = [0; 4];
            file.read(&mut total_sfnt_size).unwrap();
            let mut major_version = [0; 2];
            file.read(&mut major_version).unwrap();
            let mut minor_version = [0; 2];
            file.read(&mut minor_version).unwrap();
            let mut meta_offset = [0; 4];
            file.read(&mut meta_offset).unwrap();
            let mut meta_length = [0; 4];
            file.read(&mut meta_length).unwrap();
            let mut meta_orig_length = [0; 4];
            file.read(&mut meta_orig_length).unwrap();
            let mut priv_offset = [0; 4];
            file.read(&mut priv_offset).unwrap();
            let mut priv_length = [0; 4];
            file.read(&mut priv_length).unwrap();
            let fontheader = WOFFHeader {
                sfnt_version : sfnt_version,
                signature: u32::from_be_bytes(signature),
                flavor: u32::from_be_bytes(flavor),
                length: u32::from_be_bytes(length),
                num_tables: u16::from_be_bytes(num_tables),
                reserved: u16::from_be_bytes(reserved),
                total_sfnt_size: u32::from_be_bytes(total_sfnt_size),
                major_version: u16::from_be_bytes(major_version),
                minor_version: u16::from_be_bytes(minor_version),
                meta_offset: u32::from_be_bytes(meta_offset),
                meta_length: u32::from_be_bytes(meta_length),
                meta_orig_length: u32::from_be_bytes(meta_orig_length),
                priv_offset: u32::from_be_bytes(priv_offset),
                priv_length: u32::from_be_bytes(priv_length),
            };
            FontHeaders::WOFF(fontheader)
        },
        b"wOF2" => {          
            let mut signature = [0; 4];
            file.read(&mut signature).unwrap();
            let mut flavor = [0; 4];
            file.read(&mut flavor).unwrap();
            let mut length = [0; 4];
            file.read(&mut length).unwrap();
            let mut num_tables = [0; 2];
            file.read(&mut num_tables).unwrap();
            let mut reserved = [0; 2];
            file.read(&mut reserved).unwrap();
            let mut total_sfnt_size = [0; 4];
            file.read(&mut total_sfnt_size).unwrap();
            let mut major_version = [0; 2];
            file.read(&mut major_version).unwrap();
            let mut minor_version = [0; 2];
            file.read(&mut minor_version).unwrap();
            let mut meta_offset = [0; 4];
            file.read(&mut meta_offset).unwrap();
            let mut meta_length = [0; 4];
            file.read(&mut meta_length).unwrap();
            let mut meta_orig_length = [0; 4];
            file.read(&mut meta_orig_length).unwrap();
            let mut priv_offset = [0; 4];
            file.read(&mut priv_offset).unwrap();
            let mut priv_length = [0; 4];
            file.read(&mut priv_length).unwrap();
            let fontheader: WOFF2Header = WOFF2Header {
                sfnt_version : sfnt_version,
                signature: u32::from_be_bytes(signature),
                flavor: u32::from_be_bytes(flavor),
                length: u32::from_be_bytes(length),
                num_tables: u16::from_be_bytes(num_tables),
                reserved: u16::from_be_bytes(reserved),
                total_sfnt_size: u32::from_be_bytes(total_sfnt_size),
                major_version: u16::from_be_bytes(major_version),
                minor_version: u16::from_be_bytes(minor_version),
                meta_offset: u32::from_be_bytes(meta_offset),
                meta_length: u32::from_be_bytes(meta_length),
                meta_orig_length: u32::from_be_bytes(meta_orig_length),
                priv_offset: u32::from_be_bytes(priv_offset),
                priv_length: u32::from_be_bytes(priv_length),
            };
            FontHeaders::WOFF2(fontheader)           
        },
        _ => FontHeaders::Unknown,
    };
    font_type
}

#[derive(Debug, Clone)]
pub(crate) struct Font {
    pub(crate) font_type: FontHeaders,
    pub(crate) cmap: Box<Vec<cmap::CMAP>>,
}

pub fn font_load(filename: &PathBuf) {
    let buffer = fs::read(filename).unwrap();
    let mut font;
    match get_font_type(&buffer) {
        FontHeaders::OTF(header) => {
            let mut cmaps = Vec::new();
            header.table_records.into_iter().for_each(|record| {
                let tag: [u8;4] = record.table_tag.to_be_bytes();
                print!("{:?} ", tag);
                match &tag {
                    b"cmap" => {
                        cmaps.push(cmap::load_cmap_table(&buffer, record.offset, record.length));
                    }
                    _ => {
                        // println!("Unknown");
                    }                        
                }
            });
            font = Font {
                font_type: get_font_type(&buffer),
                cmap: Box::new(cmaps),
            };
            println!("{:?}", font.font_type);

        },
        _ => {
            println!("Unknown");
            return
        }
    }
    font.cmap.into_iter().for_each(|cmap| {
        println!("Version {} Tables {}", cmap.version, cmap.num_tables);
        for encoding in cmap.encoding_records.iter() {
            println!("{}", encoding.to_string());
            let platform = encoding.get_platform();
            let encoding = encoding.get_encoding();
            println!("Platform {:?} Encoding {:?}", platform, encoding);
        }
        let encoding_records: Vec<EncodingRecord> = cmap.encoding_records.iter()
            .map(|encoding| {
                encoding.clone()
            }).collect();
        let p = cmap::select_cmap(encoding_records);
        let encodings = p.records;
        println!("main");
        encodings.iter().for_each(|encoding| {
            println!("Platform {:?} Encoding {:?}", encoding.get_platform(), encoding.get_encoding());
        });
        println!("substitute");
        let encodings = p.substitute;
        encodings.iter().for_each(|encoding| {
            println!("Platform {:?} Encoding {:?}", encoding.get_platform(), encoding.get_encoding());
        });
        println!("uvs");

        let encodings = p.uvs;
        encodings.iter().for_each(|encoding| {
            println!("Platform {:?} Encoding {:?}", encoding.get_platform(), encoding.get_encoding());
        });
        

    });
   
}
