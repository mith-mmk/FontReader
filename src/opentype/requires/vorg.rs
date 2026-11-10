//! Optional CFF vertical origins (OpenType VORG version 1.0).
use bin_rs::reader::BinaryReader;
use std::io::{Error, ErrorKind, SeekFrom};

#[derive(Debug, Clone)]
pub(crate) struct Vorg {
    default: i16,
    records: Vec<(u16, i16)>,
}
impl Vorg {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, Error> {
        let invalid = || Error::new(ErrorKind::InvalidData, "invalid VORG table");
        if length < 8 {
            return Err(invalid());
        }
        reader.seek(SeekFrom::Start(offset as u64))?;
        if reader.read_u16_be()? != 1 || reader.read_u16_be()? != 0 {
            return Err(invalid());
        }
        let default = reader.read_i16_be()?;
        let count = reader.read_u16_be()? as usize;
        if 8 + count * 4 > length as usize {
            return Err(invalid());
        }
        let mut records = Vec::with_capacity(count);
        for _ in 0..count {
            let id = reader.read_u16_be()?;
            let origin = reader.read_i16_be()?;
            if records.last().is_some_and(|&(previous, _)| previous >= id) {
                return Err(invalid());
            }
            records.push((id, origin));
        }
        Ok(Self { default, records })
    }
    pub(crate) fn origin(&self, glyph: u16) -> i16 {
        self.records
            .binary_search_by_key(&glyph, |&(id, _)| id)
            .map(|index| self.records[index].1)
            .unwrap_or(self.default)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;
    fn parse(bytes: &[u8], length: u32) -> Result<Vorg, Error> {
        Vorg::new(&mut BytesReader::new(bytes), 0, length)
    }
    #[test]
    fn validates_boundaries_version_order_and_signed_values() {
        let valid = [0, 1, 0, 0, 3, 132, 0, 1, 0, 2, 255, 156];
        let vorg = parse(&valid, 12).unwrap();
        assert_eq!(vorg.origin(1), 900);
        assert_eq!(vorg.origin(2), -100);
        for len in 0..12 {
            assert!(parse(&valid[..len], len as u32).is_err());
        }
        assert!(parse(&valid, 8).is_err());
        assert!(parse(&valid[..11], 12).is_err());
        let mut version = valid;
        version[3] = 1;
        assert!(parse(&version, 12).is_err());
        let mut records = valid.to_vec();
        records[7] = 2;
        records.extend_from_slice(&[0, 2, 0, 0]);
        assert!(parse(&records, 16).is_err());
        records[13] = 1;
        assert!(parse(&records, 16).is_err());
        assert_eq!(
            parse(&[0, 1, 0, 0, 255, 156, 0, 0], 8).unwrap().origin(5),
            -100
        );
    }
}
