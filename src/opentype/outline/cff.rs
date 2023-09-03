// CFF is Adobe Type 1 font format, which is a compact binary format.

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

pub(crate) fn operand_encoding(b: &[u8]) -> Option<f64> {
  if b.len() == 0 {
    return None;
  }
  let b0 = b[0];
  if b0 >= 32 && b0 <= 246 {
    return Some((b0 as i32 - 139) as f64) ;
  }
  if b0 >= 247 && b0 <= 250 {
    if b.len() < 2 {
      return None;
    }
    let b1 = b[1];
    return Some(((b0 as i32 - 247) * 256 + b1 as i32 + 108) as f64) ;
  }
  if b0 >= 251 && b0 <= 254 {
    if b.len() < 2 {
      return None;
    }
    let b1 = b[1];
    return Some((-(b0 as i32 - 251) * 256 - b1 as i32 - 108) as f64) ;
  }
  if b0 == 28 {
    if b.len() < 3 {
      return None;
    }
    let value = i16::from_be_bytes([b[1], b[2]]) as i32;
    return Some((value ) as f64) ;
  }
  if b0 == 29 {
    if b.len() < 5 {
      return None;
    }
    let value = i32::from_be_bytes([b[1], b[2], b[3], b[4]]);
    return Some((value) as f64) ;
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
        0xc => { r.push('E'); r.push('-'); }
        0xd => {},
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
        0xc => { r.push('E'); r.push('-'); }
        0xd => {},
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
        Ok(f64value) => {
            return Some(f64value)
        },
        Err(_) => return None,
    }
  }
  None
}

#[derive(Debug, Clone)]
pub(crate) struct  CFF {
  pub(crate) header: Header,
  pub(crate) name_index: Index,
  pub(crate) top_dict_index: Index,
  pub(crate) string_index: Index,
  pub(crate) global_subr_index: Index,
  pub(crate) char_strings_index: Index,
  pub(crate) private_dict: PrivateDict,
  pub(crate) local_subr_index: Index,
  pub(crate) char_strings: Vec<CharString>,
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
  pub(crate) data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct PrivateDict {
  pub(crate) data: Vec<u8>,
  pub(crate) dict: Dict,
}

#[derive(Debug, Clone)]
pub(crate) struct Dict {
  pub(crate) data: Vec<u8>,
  pub(crate) entries: Vec<Entry>,
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
    let count = r.read_u16()?;
    let off_size = r.read_u8()?;
    let mut offsets = Vec::new();
    for _ in 0..count + 1 {
      let offset = r.read_u32()?;
      offsets.push(offset);
    }
    let mut data = Vec::new();
    for i in 0..count {
      let start = offsets[i as usize] as usize;
      let end = offsets[i as usize + 1] as usize;
      let mut buf = vec![0; end - start];
      r.read_bytes(&mut buf)?;
      data.append(&mut buf);
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
  pub(crate) fn parse<R: BinaryReader>(r: &mut R, size: u32) -> Result<Self, Box<dyn std::error::Error>> {
    let mut buf = vec![0; size as usize];
    r.read_bytes(&mut buf)?;
    let dict = Dict::parse(&mut BytesReader::new(&buf))?;
    Ok(Self {
      data: buf,
      dict,
    })
  }
}

impl Dict {
  pub(crate) fn parse<R: BinaryReader>(r: &mut R) -> Result<Self, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    let mut buf = Vec::new();
    loop {
      loop {
        let b = r.read_u8()?;
        buf.push(b);
        if b == 12 {
          break;
        }
      }
      let operator = u16::from_be_bytes([buf[0], buf[1]]);
      let operands = buf[2..].to_vec();
      entries.push(Entry {
        operator,
        operands,
      });
      if operator == 14 {
        break;
      }
    }
    Ok(Self {
      data: buf,
      entries,
    })
  }
}

impl Entry {
  pub(crate) fn operands(&self) -> Vec<f64> {
    let mut operands = Vec::new();
    let mut i = 0;
    while i < self.operands.len() {
      let b = self.operands[i];
      let mut buf = Vec::new();
      if b == 28 {
        buf.push(b);
        buf.push(self.operands[i + 1]);
        buf.push(self.operands[i + 2]);
        i += 3;
      } else if b == 29 {
        buf.push(b);
        buf.push(self.operands[i + 1]);
        buf.push(self.operands[i + 2]);
        buf.push(self.operands[i + 3]);
        buf.push(self.operands[i + 4]);
        i += 5;
      } else if b == 30 {
        buf.push(b);
        let mut j = i + 1;
        while j < self.operands.len() {
          let b = self.operands[j];
          if b == 30 {
            break;
          }
          buf.push(b);
          j += 1;
        }
        i = j + 1;
      } else {
        buf.push(b);
        i += 1;
      }
      let operand = operand_encoding(&buf);
      if let Some(operand) = operand {
        operands.push(operand);
      }
    }
    operands
  }
}

