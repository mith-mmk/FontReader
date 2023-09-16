// CFF is Adobe Type 1 font format, which is a compact binary format.

use std::{io::SeekFrom, collections::HashMap, error::Error};

// Compare this snippet from src/outline/cff.rs:
use bin_rs::reader::{BinaryReader, BytesReader};


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
    pub(crate) fn new<R:BinaryReader>(reader: &mut R, offset: u32, length: u32) -> Result<Self, Box<dyn Error>> {
        println!("offset: {} length: {}", offset, length);
        reader.seek(SeekFrom::Start(offset as u64))?;

        let header = Header::parse(reader)?;
        let name_index = Index::parse(reader)?;
        let name = String::from_utf8(name_index.data[0].clone())?;
        let top_dict_index = Index::parse(reader)?;
        let top_dict = Dict::parse(&top_dict_index.data[0])?;
        let fd_array_offset = top_dict.get_i32(12, 36);
        let fd_select_offset = top_dict.get_i32(12, 37);
        let charsets_offset = top_dict.get_i32(0, 15);
        let char_strings_offset = top_dict.get_i32(0, 17);
        #[cfg(debug_assertions)]
        {
            println!("fd_array: {:?}", fd_array_offset);
            println!("fd_select: {:?}", fd_select_offset);
            println!("charsets: {:?}", charsets_offset);
            println!("char_strings: {:?}", char_strings_offset);
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
       
        let private_dict = Index::parse(reader)?;
        // parse private_dict
        let private_dict = Dict::parse(&private_dict.data[0])?;
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

#[derive(Debug, Clone)]
pub(crate) enum Operand {
    Integer(i32),
    Real(f64),
    SID(SID),
    BCD(Vec<u8>),
    None,
}


pub(crate) fn operand_encoding(b: &[u8]) -> Result<(Operand,usize),Box<dyn Error>> {
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
        return Ok((Operand::Integer((b0 as i32 - 247) * 256 + b1 as i32 + 108), 2));
    }
    if (251..=254).contains(&b0) {
        if b.len() < 2 {
            return Err("buffer shotage".into());
        }
        let b1 = b[1];
        return Ok((Operand::Integer(-(b0 as i32 - 251) * 256 - b1 as i32 - 108) , 2));
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
            Err(_) => return Err("Illegal value".into())
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
    pub(crate) entries: HashMap<u16, Vec<Operand>>
}

impl Dict {
    pub(crate) fn parse(buffer: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut entries = HashMap::new();
        let mut i = 0;
        let mut operator = 0;
        let mut operands = Vec::new();
        while i < buffer.len() {
            if buffer.len() <= i  {
                break;
            }
            let b = buffer[i];
            if b == 12 {
                operator = (b as u16) << 8 | buffer[i + 1] as u16;            
                entries.insert(operator, operands);
                operands = Vec::new();
                i += 2;

            } else if b <=21 {
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


        Ok(Self {
            entries,
        })
    }
    
    pub(crate) fn get_sid(&self, key1: u8, key2: u8) -> Result<i32, Box<dyn Error>> {
        self.get_i32(key1, key2)
    }

    pub(crate) fn get_i32(&self, key1: u8, key2: u8) -> Result<i32, Box<dyn Error>> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                if operands.len() != 1 {
                    return Err("Illegal operands".into());
                }
                match operands[0] {
                    Operand::Integer(value) => Ok(value),
                    Operand::Real(value) => Ok(value as i32),
                    _ => Err("Illegal operands".into())
                }
            }
            None => Err("not found".into())
        }
    }


    pub fn get_f64(&self, key1: u8, key2: u8) -> Result<f64, Box<dyn Error>> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                if operands.len() != 1 {
                    return Err("Illegal operands".into());
                }
                match operands[0] {
                    Operand::Integer(value) => Ok(value as f64),
                    Operand::Real(value) => Ok(value),
                    _ => Err("Illegal operands".into())
                }
            }
            None => Err("not found".into())
        }
    }

    pub(crate) fn get_i32_array(&self, key1: u8, key2: u8) -> Result<Vec<i32>, Box<dyn Error>> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                let mut r = Vec::new();
                for operand in operands {
                    match operand {
                        Operand::Integer(value) => r.push(*value),
                        Operand::Real(value) => r.push(*value as i32),
                        _ => return Err("Illegal operands".into())
                    }
                }
                Ok(r)
            }
            None => Err("not found".into())
        }
    }

    pub(crate) fn get_f64_array(&self, key1: u8, key2: u8) -> Result<Vec<f64>, Box<dyn Error>> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                let mut r = Vec::new();
                for operand in operands {
                    match operand {
                        Operand::Integer(value) => r.push(*value as f64),
                        Operand::Real(value) => r.push(*value),
                        _ => return Err("Illegal operands".into())
                    }
                }
                Ok(r)
            }
            None => Err("not found".into())
        }
    }

}


#[derive(Debug, Clone)]
pub(crate) struct CharString {
    pub(crate) data: Vec<u8>,
    pub(crate) instructions: Vec<u8>,
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
        Ok(Self {
            count,
            data,
        })
    }
}



