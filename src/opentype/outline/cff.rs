// CFF is Adobe Type 1 font format, which is a compact binary format.

use std::io::SeekFrom;

// Compare this snippet from src/outline/cff.rs:
use bin_rs::reader::{BinaryReader, BytesReader};

use crate::opentype::requires::name;
//
// // CFF is Adobe Type 1 font format, which is a compact binary format.
//
/*/
type Card8 = u8;
type Card16 = u16;
type OffSize = u8;
type Offset = u32; 1-4bytes
type SID = u16;
type Card32 = u32;
*/
type SID = u16;


#[derive(Debug, Clone)]
pub(crate) struct CFF {
    pub(crate) header: Header,
    pub(crate) name: String,
    pub(crate) top_dict_index: Index, // TopDict
    pub(crate) strings: Vec<String>,
    pub(crate) global_subr_index: Index,
    pub(crate) encordings: Index,
    pub(crate) charset: Index,
    pub(crate) fd_select: Option<Index>,
    pub(crate) char_strings_index: Index,
    pub(crate) char_strings: Vec<CharString>,
    pub(crate) font_dict_index: Option<Index>,
    pub(crate) private_dict: PrivateDict,
    pub(crate) local_subr_index: Index,
}

impl CFF {
    pub(crate) fn new<R:BinaryReader>(reader: &mut R, offset: u32, length: u32) -> Result<Self, Box<dyn std::error::Error>> {
        println!("offset: {} length: {}", offset, length);
        reader.seek(SeekFrom::Start(offset as u64))?;

        let header = Header::parse(reader)?;
        let name_index = Index::parse(reader)?;
        let name = String::from_utf8(name_index.data[0].clone())?;
        let top_dict_index = Index::parse(reader)?;
        #[cfg(debug_assertions)]
        {
        let top_dict = Dict::parse(&top_dict_index.data[0]);
        println!("top_dict: {:?}", top_dict);
        }
        let string_index = Index::parse(reader)?;
        let mut strings = Vec::new();
        for i in 0..string_index.count {
            let buf = string_index.data[i as usize].clone();
            strings.push(String::from_utf8(buf)?);
        }

        let global_subr_index = Index::parse(reader)?;
        println!("global_subr_index: {:?}", global_subr_index);
        let encordings = Index::parse(reader)?;
        println!("encordings: {:?}", encordings);
        let charset = Index::parse(reader)?;
        println!("charset: {:?}", charset);
        let fd_select = Index::parse(reader)?;
        println!("fd_select: {:?}", fd_select);
        let char_strings_index = Index::parse(reader)?;
        println!("char_strings_index: {:?}", char_strings_index);
        let nglipys = char_strings_index.count;
        panic!("nglipys: {}", nglipys);
        // parce char_strings
        let font_dict_index = Index::parse(reader)?;
        // parse font_dict_index
       
        let private_dict = PrivateDict::parse(reader, top_dict_index.data[0].len() as u32)?;
        let local_subr_index = Index::parse(reader)?;
        Ok(Self {
            header,
            name,
            top_dict_index,
            strings,
            global_subr_index,
            char_strings_index,
            private_dict,
            local_subr_index,
            encordings,
            charset,
            fd_select: None,
            char_strings: Vec::new(),
            font_dict_index: None,         

        })     
    }
}



pub(crate) fn operand_encoding(b: &[u8]) -> Option<(f64,usize)> {
    if b.is_empty() {
        return None;
    }
    let b0 = b[0];
    if (32..=246).contains(&b0) {
        return Some(((b0 as i32 - 139) as f64, 1));
    }
    if (247..=250).contains(&b0) {
        if b.len() < 2 {
            return None;
        }
        let b1 = b[1];
        return Some((((b0 as i32 - 247) * 256 + b1 as i32 + 108) as f64, 2));
    }
    if (251..=254).contains(&b0) {
        if b.len() < 2 {
            return None;
        }
        let b1 = b[1];
        return Some(((-(b0 as i32 - 251) * 256 - b1 as i32 - 108) as f64, 2));
    }
    if b0 == 28 {
        if b.len() < 3 {
            return None;
        }
        let value = i16::from_be_bytes([b[1], b[2]]) as i32;
        return Some(((value) as f64, 3));
    }
    if b0 == 29 {
        if b.len() < 5 {
            return None;
        }
        let value = i32::from_be_bytes([b[1], b[2], b[3], b[4]]);
        return Some(((value) as f64, 5));
    }
    if b0 == 30 {
        let mut r = Vec::new();

        for i in 1..b.len() {
            let b = b[i];
            let r0 = b >> 4;
            let r1 = b & 0x0f;
            match r0 {
                0 => r.push('0'),
                1 => r.push('1'),
                2 => r.push('2'),
                3 => r.push('3'),
                4 => r.push('4'),
                5 => r.push('5'),
                6 => r.push('6'),
                7 => r.push('7'),
                8 => r.push('8'),
                9 => r.push('9'),
                0xa => r.push('.'),
                0xb => r.push('E'),
                0xc => {
                    r.push('E');
                    r.push('-');
                }
                0xd => {}
                0xe => r.push('-'),
                0xf => {
                    break;
                }
                _ => {}
            }
            match r1 {
                0 => r.push('0'),
                1 => r.push('1'),
                2 => r.push('2'),
                3 => r.push('3'),
                4 => r.push('4'),
                5 => r.push('5'),
                6 => r.push('6'),
                7 => r.push('7'),
                8 => r.push('8'),
                9 => r.push('9'),
                0xa => r.push('.'),
                0xb => r.push('E'),
                0xc => {
                    r.push('E');
                    r.push('-');
                }
                0xd => {}
                0xe => r.push('-'),
                0xf => {
                    break;
                }
                _ => {}
            }
        }
        let str = r.iter().collect::<String>();
        println!("str: {}", str);
        match str.parse::<f64>() {
            Ok(f64value) => return Some((f64value, r.len() + 1)),
            Err(_) => return None,
        }
    }
    None
}



#[derive(Debug, Clone)]
pub(crate) struct Header {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) hdr_size: u8,
    pub(crate) off_size: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct Index {
    pub(crate) count: u16,
    pub(crate) off_size: u8,
    pub(crate) offsets: Vec<u32>,
    pub(crate) data: Vec<Vec<u8>>,
}

pub(crate) struct TopDict {
    pub(crate) version: Option<SID>,
    pub(crate) notice: Option<SID>,
    pub(crate) copy_right: Option<SID>,
    pub(crate) full_name: Option<SID>,
    pub(crate) family_name: Option<SID>,
    pub(crate) weight: Option<SID>,
    pub(crate) is_fixed_pitch: Option<bool>,
    pub(crate) italic_angle: Option<f64>,
    pub(crate) underline_position: Option<i16>,
    pub(crate) underline_thickness: Option<i16>,
    pub(crate) paint_type: Option<u16>,
    pub(crate) char_string_type: Option<u16>,
    pub(crate) font_matrix: Option<[f64; 6]>,
    pub(crate) unique_id: Option<u32>,
    pub(crate) font_bounding_box: Option<[i16; 4]>,
    pub(crate) stroke_width: Option<f64>,
    pub(crate) xuid: Option<Vec<u8>>,
    pub(crate) charset: Option<u16>,
    pub(crate) encoding: Option<u16>,
    pub(crate) char_strings: Option<u16>,
    pub(crate) private: Option<u16>,
    pub(crate) synthetic_base: Option<u16>,
    pub(crate) post_script: Option<String>,
    pub(crate) base_font_name: Option<String>,
    pub(crate) base_font_blend: Option<Vec<f64>>,
}

impl TopDict {
    pub(crate) fn enpty() -> Self {
        Self {
            version: None,
            notice: None,
            copy_right: None,
            full_name: None,
            family_name: None,
            weight: None,
            is_fixed_pitch: None,
            italic_angle: None,
            underline_position: None,
            underline_thickness: None,
            paint_type: None,
            char_string_type: None,
            font_matrix: None,
            unique_id: None,
            font_bounding_box: None,
            stroke_width: None,
            xuid: None,
            charset: None,
            encoding: None,
            char_strings: None,
            private: None,
            synthetic_base: None,
            post_script: None,
            base_font_name: None,
            base_font_blend: None,
        }

    }
    pub(crate) fn new(buffer: &Vec<u8>) -> Self {
        let mut top_dict = Self::enpty();
        
        top_dict
    }

}






#[derive(Debug, Clone)]
pub(crate) struct PrivateDict {
    pub(crate) data: Vec<u8>,
    pub(crate) dict: Dict,
}

#[derive(Debug, Clone)]
pub(crate) struct Dict {
    pub(crate) entries: Vec<Entry>,
}

impl Dict {
    pub(crate) fn parse(buffer: &[u8]) -> Self {
        let mut entries = Vec::new();
        let mut i = 0;
        loop {
            if buffer.len() <= i  {
                break;
            }
            let b = buffer[i];
            if b == 12 {
                let operator = (b as u16) << 8 | buffer[i + 1] as u16;
                let operands = Vec::new();
                entries.push(Entry { operator, operands });
                i += 2;
            } else if b <=21 {
                let operator = b as u16;
                let operands = Vec::new();
                entries.push(Entry { operator, operands });
                i += 1;
            } else if b >= 22 && b <= 27 {
                let operator = (b as u16) << 8 | buffer[i + 1] as u16;
                let operands = Vec::new();
                entries.push(Entry { operator, operands });
                i += 2;
            } else if b >= 28 && b <= 31 {
                let operator = (b as u16) << 8 | buffer[i + 1] as u16;
                let operands = Vec::new();
                entries.push(Entry { operator, operands });
                i += 2;
            } else if b >= 32 && b <= 246 {
                let operator = 28;
                let operands = Vec::new();
                entries.push(Entry { operator, operands });
                i += 1;
            } else if b >= 247 && b <= 250 {
                let operator = 29;
                let operands = Vec::new();
                entries.push(Entry { operator, operands });
                i += 2;
            } else if b >= 251 && b <= 254 {
                let operator = 30;
                let operands = Vec::new();
                entries.push(Entry { operator, operands });
                i += 2;
            } else if b == 255 {
                let operator = 30;
                let operands = Vec::new();
                entries.push(Entry { operator, operands });
                i += 5;
            } else {
                break;
            }

        }

        Self {
            entries,
        }
    }
    

}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) operator: u16,
    pub(crate) operands: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct CharString {
    pub(crate) data: Vec<u8>,
    pub(crate) instructions: Vec<u8>,
}

impl Header {
    pub(crate) fn parse<R: BinaryReader>(r: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        let major = r.read_u8()?;
        let minor = r.read_u8()?;
        let hdr_size = r.read_u8()?;
        let off_size = r.read_u8()?;
        Ok(Self {
            major,
            minor,
            hdr_size,
            off_size,
        })
    }
}

impl Index {
    pub(crate) fn parse<R: BinaryReader>(r: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
        let count = r.read_u16_be()?;
        if count == 0 {
            return Ok(Self {
                count,
                off_size: 0,
                offsets: Vec::new(),
                data: Vec::new(),
            });
        }
        let off_size = r.read_u8()?;
        println!("count: {} off_size: {}", count, off_size);

        let mut offsets = Vec::new();
        for _ in 0..count + 1 {
            println!("offsets.len(): {}", offsets.len());
            match off_size {
                1 => offsets.push(r.read_u8()? as u32),
                2 => offsets.push(r.read_u16_be()? as u32),
                3 => {
                    let b0 = r.read_u8()?;
                    let b1 = r.read_u16_be()?;
                    offsets.push(((b0 as u32) << 16) + (b1 as u32));
                }
                4 => offsets.push(r.read_u32_be()?),
                _ => {}
            }
        }
        println!("offsets: {:?}", offsets);

        let mut data = Vec::new();
        for i in 0..count {
            let start = offsets[i as usize] as usize;
            let end = offsets[i as usize + 1] as usize;
            println!("start: {} end: {}", start, end);
            let buf = r.read_bytes_as_vec(end - start)?;
            data.push(buf);
        }
        Ok(Self {
            count,
            off_size,
            offsets,
            data,
        })
    }
}

impl PrivateDict {
    pub(crate) fn parse<R: BinaryReader>(
        r: &mut R,
        size: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buf = r.read_bytes_as_vec(size as usize)?;
        let dict = Dict::parse(&buf);
        Ok(Self { data: buf, dict })
    } 
}


