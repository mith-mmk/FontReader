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

pub(crate) fn operand_encoding(b: &[u8]) -> Option<i32> {
  if b.len() == 0 {
    return None;
  }
  let b0 = b[0];
  if b0 >= 32 && b0 <= 246 {
    return Some(b0 as i32 - 139);
  }
  if b0 >= 247 && b0 <= 250 {
    if b.len() < 2 {
      return None;
    }
    let b1 = b[1];
    return Some((b0 as i32 - 247) * 256 + b1 as i32 + 108);
  }
  if b0 >= 251 && b0 <= 254 {
    if b.len() < 2 {
      return None;
    }
    let b1 = b[1];
    return Some(-(b0 as i32 - 251) * 256 - b1 as i32 - 108);
  }
  if b0 == 28 {
    if b.len() < 3 {
      return None;
    }
    let value = i16::from_be_bytes([b[1], b[2]]) as i32;
    return Some (value );
  }
  if b0 == 29 {
    if b.len() < 5 {
      return None;
    }
    let value = i32::from_be_bytes([b[1], b[2], b[3], b[4]]);
    return Some (value);
  }
  None
}


