use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone)]
pub(crate) enum ClassDefinition {
    Format1(ClassDefinitionFormat1),
    Format2(ClassDefinitionFormat2),
}

impl ClassDefinition {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R, offset: u64, _: u32) -> Self {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let class_format = reader.read_u16_be().unwrap();
        match class_format {
            1 => {
                let start_glyph_id = reader.read_u16_be().unwrap();
                let glyph_count = reader.read_u16_be().unwrap();
                let mut class_values = Vec::new();
                for _ in 0..glyph_count {
                    class_values.push(reader.read_u16_be().unwrap());
                }
                Self::Format1(ClassDefinitionFormat1 {
                    class_format,
                    start_glyph_id,
                    glyph_count,
                    class_values,
                })
            }
            2 => {
                let range_count = reader.read_u16_be().unwrap();
                let mut range_records = Vec::new();
                for _ in 0..range_count {
                    let start_glyph_id = reader.read_u16_be().unwrap();
                    let end_glyph_id = reader.read_u16_be().unwrap();
                    let class = reader.read_u16_be().unwrap();
                    range_records.push(ClassRangeRecord {
                        start_glyph_id,
                        end_glyph_id,
                        class,
                    });
                }
                Self::Format2(ClassDefinitionFormat2 {
                    class_format,
                    range_count,
                    range_records,
                })
            }
            _ => {
                panic!("Unknown ClassDefinition format: {}", class_format);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClassDefinitionFormat1 {
    pub(crate) class_format: u16,
    pub(crate) start_glyph_id: u16,
    pub(crate) glyph_count: u16,
    pub(crate) class_values: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct ClassDefinitionFormat2 {
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
