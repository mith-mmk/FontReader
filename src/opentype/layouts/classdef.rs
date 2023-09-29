use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) enum ClassDef {
    Format1(ClassDefFormat1),
    Format2(ClassDefFormat2),
}

impl ClassDef {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let class_format = reader.read_u16_be()?;
        if class_format == 1 {
            let start_glyph_id = reader.read_u16_be()?;
            let glyph_count = reader.read_u16_be()?;
            let mut class_value_array = Vec::new();
            for _ in 0..glyph_count {
                class_value_array.push(reader.read_u16_be()?);
            }

            Ok(ClassDef::Format1(ClassDefFormat1 {
                class_format,
                start_glyph_id,
                glyph_count,
                class_value_array,
            }))
        } else if class_format == 2 {
            let range_count = reader.read_u16_be()?;
            let mut range_records = Vec::new();
            for _ in 0..range_count {
                let start_glyph_id = reader.read_u16_be()?;
                let end_glyph_id = reader.read_u16_be()?;
                let class = reader.read_u16_be()?;
                range_records.push(ClassRangeRecord {
                    start_glyph_id,
                    end_glyph_id,
                    class,
                });
            }
            Ok(ClassDef::Format2(ClassDefFormat2 {
                class_format,
                range_count,
                range_records,
            }))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unknown class format",
            ))
        }
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "Class Deffinition\n".to_string();
        match self {
            ClassDef::Format1(class_def) => {
                string += &format!("class_format {}\n", class_def.class_format);
                string += &format!("start_glyph_id {}\n", class_def.start_glyph_id);
                string += &format!("glyph_count {}\n", class_def.glyph_count);
                string += &format!("class_value_array {:?}\n", class_def.class_value_array);
            }
            ClassDef::Format2(class_def) => {
                string += &format!("class_format {}\n", class_def.class_format);
                string += &format!("range_count {}\n", class_def.range_count);
                string += &format!("range_records {:?}\n", class_def.range_records);
            }
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClassDefFormat1 {
    pub(crate) class_format: u16,
    pub(crate) start_glyph_id: u16,
    pub(crate) glyph_count: u16,
    pub(crate) class_value_array: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct ClassDefFormat2 {
    pub(crate) class_format: u16,
    pub(crate) range_count: u16,
    pub(crate) range_records: Vec<ClassRangeRecord>,
}

#[derive(Debug, Clone)]
/* */
pub(crate) struct ClassRangeRecord {
    pub(crate) start_glyph_id: u16,
    pub(crate) end_glyph_id: u16,
    pub(crate) class: u16,
}
