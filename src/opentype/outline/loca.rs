use std::{fmt, io::SeekFrom};

use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) struct LOCA {
    pub(crate) offsets: Box<Vec<u32>>,
}

impl fmt::Display for LOCA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl LOCA {
    pub(crate) fn new<R: BinaryReader>(
        file: &mut R,
        offest: u32,
        length: u32,
        num_glyphs: u16,
    ) -> Result<Self, std::io::Error> {
        get_loca(file, offest, length, num_glyphs)
    }

    pub(crate) fn new_by_size<R: BinaryReader>(
        file: &mut R,
        offest: u32,
        length: u32,
        index_to_loc_format: usize,
    ) -> Result<Self, std::io::Error> {
        get_loca_by_size(file, offest, length, index_to_loc_format)
    }

    pub(crate) fn to_string(&self) -> String {
        let max_length = 10;
        let mut string = "loca\n".to_string();
        let length = if self.offsets.len() < 10 {
            self.offsets.len()
        } else {
            max_length
        };
        for i in 0..length {
            if i % 16 == 0 {
                string += "\n";
            }
            let offset = format!("{:3} Offset {:08x}\n", i, self.offsets[i]);
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
) -> Result<LOCA, std::io::Error> {
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

    Ok(LOCA {
        offsets: Box::new(offsets),
    })
}

fn get_loca<R: BinaryReader>(file: &mut R, offest: u32, length: u32, num_glyphs: u16) -> Result<LOCA, std::io::Error> {
    let size = length / num_glyphs as u32;
    file.seek(SeekFrom::Start(offest as u64))?;
    let index_to_loc_format = if size != 4 && size != 2 {
        panic!("Invalid size of loca table");
    } else if size == 4 {
        1
    } else {
        0
    };
    get_loca_by_size(file, offest, length, index_to_loc_format)
}
