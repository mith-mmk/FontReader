// CFF is Adobe Type 1 font format, which is a compact binary format.

// Compare this snippet from src/outline/cff.rs:
use std::fmt;
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
