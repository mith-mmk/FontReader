use std::{fmt, io::SeekFrom};

use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) struct Loca {
    pub(crate) offsets: Vec<u32>,
}

impl fmt::Display for Loca {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_text())
    }
}

impl Loca {
    pub(crate) fn new_by_size<R: BinaryReader>(
        file: &mut R,
        offest: u32,
        length: u32,
        index_to_loc_format: usize,
    ) -> Result<Self, std::io::Error> {
        get_loca_by_size(file, offest, length, index_to_loc_format)
    }

    fn format_text(&self) -> String {
        let max_length = 10;
        let mut string = "loca\n".to_string();
        let length = if self.offsets.len() < 10 {
            self.offsets.len()
        } else {
            max_length
        };
        for (i, value) in self.offsets.iter().take(length).enumerate() {
            if i % 16 == 0 {
                string += "\n";
            }
            let offset = format!("{:3} Offset {:08x}\n", i, value);
            string += &offset;
        }
        string
    }
}

fn get_loca_by_size<R: BinaryReader>(
    file: &mut R,
    offest: u32,
    length: u32,
    index_to_loc_format: usize,
) -> Result<Loca, std::io::Error> {
    file.seek(SeekFrom::Start(offest as u64))?;

    let mut offsets = Vec::new();
    let mut i = 0;
    while i < length {
        let offset: u32 = if index_to_loc_format == 0 {
            i += 2;
            file.read_u16_be()? as u32 * 2
        } else {
            i += 4;
            file.read_u32_be()?
        };
        offsets.push(offset);
    }

    Ok(Loca { offsets })
}

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn loca_display_preserves_offsets() {
        let loca = Loca {
            offsets: vec![0, 0xab],
        };
        assert_eq!(
            loca.to_string(),
            "loca\n\n  0 Offset 00000000\n  1 Offset 000000ab\n"
        );
        assert_eq!(format!("{loca}"), loca.to_string());
    }
}
