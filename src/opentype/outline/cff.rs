// CFF is Adobe Type 1 font format, which is a compact binary format.

use std::{collections::HashMap, error::Error, io::SeekFrom};

// Compare this snippet from src/outline/cff.rs:
use bin_rs::reader::BinaryReader;

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
    pub(crate) charsets: Charsets,
    pub(crate) char_string: CharString,
    // pub(crate) fd_Select: FDSelect,
    // pub(crate) fd_dict_index: FDDistIndex,
    pub(crate) private_dict: Option<PrivateDict>,
}



impl CFF {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, Box<dyn Error>> {
        println!("offset: {} length: {}", offset, length);
        reader.seek(SeekFrom::Start(offset as u64))?;

        let header = Header::parse(reader)?;
        let name_index = Index::parse(reader)?;
        let name = String::from_utf8(name_index.data[0].clone())?;
        let top_dict_index = Index::parse(reader)?;
        let top_dict = Dict::parse(&top_dict_index.data[0])?;
        let n_glyphs = top_dict.get_i32(0, 15).unwrap() as usize;
        let encording_offset = top_dict.get_i32(0, 16); // none
        let global_subr_index_offset = top_dict.get_i32(12, 29); // none
        let fd_array_offset = top_dict.get_i32(12, 36).unwrap();
        let fd_select_offset = top_dict.get_i32(12, 37).unwrap();
        let charsets_offset = top_dict.get_i32(0, 15).unwrap();
        let char_strings_offset = top_dict.get_i32(0, 17).unwrap();
        #[cfg(debug_assertions)]
        {
            println!("n_glyphs: {}", n_glyphs);
            println!("encording: {:?}", encording_offset);
            println!("global_subr_index: {:?}", global_subr_index_offset);
            println!("fd_array: {:?}", fd_array_offset);
            println!("fd_select: {:?}", fd_select_offset);
            println!("charsets: {:?}", charsets_offset);
            println!("char_strings: {:?}", char_strings_offset);
        }

        let charsets_offset = charsets_offset as u32 + offset;

        let charsets = Charsets::new(reader, charsets_offset, n_glyphs as u32)?;
        let char_strings_offset = char_strings_offset as u32 + offset;
        let char_string = CharString::new(reader, char_strings_offset as u32)?;
        println!("char_string: {:?}", char_string.data.data[0]);
        let private = top_dict.get_i32_array(0, 18);
        let private_dict = if let Some(private) = private {
            println!("private: {:?}", private);
            let private_dict_length = private[0] as u32;
            let private_dict_offset = private[1] as u32;
    
            let private_dict_offset = private_dict_offset as u32 + offset;
            let private_dict_index = Index::parse(reader)?;
            let private_dict = Dict::parse(&private_dict_index.data[0])?;
            println!("private_dict: {:?}", private_dict);
            Some(private_dict)
        } else {
            None
        };

        Ok (Self {
            header,
            name,
            top_dict_index,
            strings: Vec::new(),
            charsets,
            char_string,
            // fd_Select: FDSelect,
            // fd_dict_index: FDDistIndex,
            private_dict,
        })
    }
}


#[derive(Debug, Clone)]
pub(crate) struct Charsets {
    n_glyphs: usize,
    format: u8,
    sid: Vec<u16>,
}

impl Charsets {
    fn new<R: BinaryReader>(reader: &mut R, offset: u32, n_glyphs: u32)  -> Result<Self, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let format = reader.read_u8().unwrap();
        println!("format: {}", format);
        let mut charsets = Self {
            n_glyphs: n_glyphs as usize,
            format,
            sid: Vec::new(),
        };

        match format {
            0 => charsets.parse_format0(reader, n_glyphs)?,
            1..=2 => charsets.parse_format1(reader, n_glyphs)?,
            _ => {panic!("Illegal format: {}", format) }
        }
        Ok(charsets)
    }

    fn parse_format0<R: BinaryReader>(&mut self, reader: &mut R, n_glyphs: u32) -> Result<(), Box<dyn Error>> {
        let mut i = 1;
        for _ in 0..n_glyphs as usize -1 {
            let sid = reader.read_u16_be()?;
            self.sid.push(sid);
            i += 2;
        }
        Ok(())
    }

    fn parse_format1<R: BinaryReader>(&mut self, reader: &mut R, n_glyphs: u32) -> Result<(), Box<dyn Error>>
    {
        let mut i = 1;
        while i < n_glyphs as usize -1 {
            let mut sid = reader.read_u16_be()?;
            let n_left = 
                if self.format == 1 {
                    reader.read_u8()? as usize
                } else {
                    reader.read_u16_be()? as usize
                };
            for _ in 0..=n_left {
                self.sid.push(sid);
                i += 1;
                sid += 1;
            }
        }
        Ok(())
    }

    fn parse_format2(&mut self) {
        todo!()
    }


}

#[derive(Debug, Clone)]
pub(crate) enum Operand {
    Integer(i32),
    Real(f64),
}


pub(crate) fn operand_encoding(b: &[u8]) -> Result<(Operand, usize), Box<dyn Error>> {
    if b.is_empty() {
        return Err("empty".into());
    }
    let b0 = b[0];
    if (32..=246).contains(&b0) {
        return Ok((Operand::Integer(b0 as i32 - 139), 1));
    }
    if (247..=250).contains(&b0) {
        if b.len() < 2 {
            return Err("buffer shotage".into());
        }
        let b1 = b[1];
        return Ok((
            Operand::Integer((b0 as i32 - 247) * 256 + b1 as i32 + 108),
            2,
        ));
    }
    if (251..=254).contains(&b0) {
        if b.len() < 2 {
            return Err("buffer shotage".into());
        }
        let b1 = b[1];
        return Ok((
            Operand::Integer(-(b0 as i32 - 251) * 256 - b1 as i32 - 108),
            2,
        ));
    }
    if b0 == 28 {
        if b.len() < 3 {
            return Err("buffer shotage".into());
        }
        let value = i16::from_be_bytes([b[1], b[2]]) as i32;
        return Ok((Operand::Integer(value), 3));
    }
    if b0 == 29 {
        if b.len() < 5 {
            return Err("buffer shotage".into());
        }
        let value = i32::from_be_bytes([b[1], b[2], b[3], b[4]]);
        return Ok((Operand::Integer(value), 5));
    }
    if b0 == 30 {
        let mut r = Vec::new();
        let mut x = 1;
        for i in 1..b.len() {
            let b = b[i];
            x += 1;
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
        match str.parse::<f64>() {
            Ok(f64value) => return Ok((Operand::Real(f64value), x)),
            Err(_) => return Err("Illegal value".into()),
        }
    }
    Err("Illegal value".into())
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
    pub(crate) data: Vec<Vec<u8>>,
}

type PrivateDict = Dict;

#[derive(Debug, Clone)]
pub(crate) struct Dict {
    pub(crate) entries: HashMap<u16, Vec<Operand>>,
}

impl Dict {
    pub(crate) fn parse(buffer: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut entries = HashMap::new();
        let mut i = 0;
        let mut operator = 0;
        let mut operands = Vec::new();
        while i < buffer.len() {
            if buffer.len() <= i {
                break;
            }
            let b = buffer[i];
            if b == 12 {
                operator = (b as u16) << 8 | buffer[i + 1] as u16;
                entries.insert(operator, operands);
                operands = Vec::new();
                i += 2;
            } else if b <= 21 {
                operator = b as u16;
                entries.insert(operator, operands);
                operands = Vec::new();
                i += 1;
            }
            if i >= buffer.len() {
                break;
            }

            let (operand, len) = operand_encoding(&buffer[i..])?;
            operands.push(operand);

            i += len;
        }

        Ok(Self { entries })
    }

    pub(crate) fn get_sid(&self, key1: u8, key2: u8) -> Option<i32> {
        self.get_i32(key1, key2)
    }

    pub(crate) fn get_i32(&self, key1: u8, key2: u8) -> Option<i32> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                if operands.len() != 1 {
                    return None;
                }
                match operands[0] {
                    Operand::Integer(value) => Some(value),
                    Operand::Real(value) => Some(value as i32),
                    _ => None,
                }
            }
            None => None,
        }
    }

    pub fn get_f64(&self, key1: u8, key2: u8) -> Option<f64> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                if operands.len() != 1 {
                    return None;
                }
                match operands[0] {
                    Operand::Integer(value) => Some(value as f64),
                    Operand::Real(value) => Some(value),
                    _ => None,
                }
            }
            None => None,
        }
    }

    pub(crate) fn get_i32_array(&self, key1: u8, key2: u8) -> Option<Vec<i32>> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                let mut r = Vec::new();
                for operand in operands {
                    match operand {
                        Operand::Integer(value) => r.push(*value),
                        Operand::Real(value) => r.push(*value as i32),
                        _ => return None,
                    }
                }
                Some(r)
            }
            None => None,
        }
    }

    pub(crate) fn get_f64_array(&self, key1: u8, key2: u8) -> Option<Vec<f64>> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                let mut r = Vec::new();
                for operand in operands {
                    match operand {
                        Operand::Integer(value) => r.push(*value as f64),
                        Operand::Real(value) => r.push(*value),
                        _ => return None,
                    }
                }
                Some(r)
            }
            None => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CharString {
    pub(crate) data: Index,
}

impl CharString {
    pub(crate) fn new<R:BinaryReader>(reader: &mut R, offset: u32) -> Result<Self, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let index = Index::parse(reader)?;
        Ok(Self {
            data: index
        })
    }


}


impl Header {
    pub(crate) fn parse<R: BinaryReader>(r: &mut R) -> Result<Self, Box<dyn Error>> {
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
    pub(crate) fn parse<R: BinaryReader>(r: &mut R) -> Result<Self, Box<dyn Error>> {
        let count = r.read_u16_be()?;
        if count == 0 {
            return Ok(Self {
                count,
                data: Vec::new(),
            });
        }
        let off_size = r.read_u8()?;
        println!("count: {} off_size: {}", count, off_size);

        let mut offsets = Vec::new();
        for _ in 0..count + 1 {
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

        let mut data = Vec::new();
        for i in 0..count {
            let start = offsets[i as usize] as usize;
            let end = offsets[i as usize + 1] as usize;
            let buf = r.read_bytes_as_vec(end - start)?;
            data.push(buf);
        }
        Ok(Self { count, data })
    }
}
