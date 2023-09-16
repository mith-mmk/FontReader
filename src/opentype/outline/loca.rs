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
    ) -> Self {
        get_loca(file, offest, length, num_glyphs)
    }

    pub(crate) fn new_by_size<R: BinaryReader>(
        file: &mut R,
        offest: u32,
        length: u32,
        index_to_loc_format  : usize,
    ) -> Self {
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

fn get_loca_by_size<R: BinaryReader>(file: &mut R, offest: u32, length: u32, index_to_loc_format  : usize) -> LOCA {
    file.seek(SeekFrom::Start(offest as u64)).unwrap();

    let mut offsets = Vec::new();
    let mut i = 0;
    while i < length {
        let offset: u32 = if index_to_loc_format == 0 {
            i += 2;
            file.read_u16_be().unwrap() as u32 * 2
        } else {
            i += 4;
            file.read_u32_be().unwrap()
        };
        offsets.push(offset);
    }

    LOCA {
        offsets: Box::new(offsets),
    }
}

fn get_loca<R: BinaryReader>(file: &mut R, offest: u32, length: u32, num_glyphs: u16) -> LOCA {
    let size = length / num_glyphs as u32;
    file.seek(SeekFrom::Start(offest as u64)).unwrap();
    if size != 4 && size != 2 {
        panic!("Invalid size of loca table");
    }

    let mut offsets = Vec::new();
    for _ in 0..num_glyphs + 1 {
        let offset: u32 = if size == 2 {
            file.read_u16_be().unwrap() as u32 * 2
        } else {
            file.read_u32_be().unwrap()
        };
        offsets.push(offset);
    }

    LOCA {
        offsets: Box::new(offsets),
    }
}
