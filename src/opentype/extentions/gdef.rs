use crate::opentype::layouts::*;
use bin_rs::reader::BinaryReader;
use std::io::SeekFrom;

#[derive(Debug, Clone)]
pub(crate) struct GDEF {
    pub(crate) major_versionn: u16,
    pub(crate) minor_version: u16,
    pub(crate) glyph_class_def: Option<ClassDef>,
    pub(crate) attach_list: Option<AttachPointList>,
    pub(crate) lig_caret_list: Option<LigatureCaretList>,
    pub(crate) mark_attach_class_def: Option<ClassDef>,
    // 1.2
    pub(crate) mark_glyph_sets_def: Option<MarkGlyphSetsDef>,
    // 1.3

    // pub(crate) item_var_store: Option<VariationStore>,
}

impl GDEF {
    pub fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        length: usize,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let major_versionn = reader.read_u16_be()?;
        let minor_version = reader.read_u16_be()?;
        let glyph_class_def_offset = reader.read_u16_be()?;
        let _attach_list_offset = reader.read_u16_be()?;
        let _lig_caret_list_offset = reader.read_u16_be()?;
        let _mark_attach_class_def_offset = reader.read_u16_be()?;
        let _mark_glyph_sets_def_offset = if minor_version >= 2 {
            reader.read_u16_be()?
        } else {
            0
        };
        let _item_var_store_offset = if minor_version >= 3 {
            reader.read_u16_be()?
        } else {
            0
        };
        let glyph_class_def = if glyph_class_def_offset != 0 {
            Some(ClassDef::new(reader, offset + glyph_class_def_offset as u64)?)
        } else {
            None
        };
        
        let attach_list = if _attach_list_offset != 0 {
            let attach_list = AttachPointList::new(reader, offset + _attach_list_offset as u64)?;
            Some(attach_list)
        } else {
            None
        };

        let lig_caret_list = if _lig_caret_list_offset != 0 {
            let lig_caret_list =
                LigatureCaretList::new(reader, offset + _lig_caret_list_offset as u64)?;
            Some(lig_caret_list)
        } else {
            None
        };

        let mark_attach_class_def = if _mark_attach_class_def_offset != 0 {
            let mark_glyph_sets_def =
                ClassDef::new(reader, offset + _mark_attach_class_def_offset as u64)?;
            Some(mark_glyph_sets_def)
        } else {
            None
        };

        let mark_glyph_sets_def = if _mark_glyph_sets_def_offset != 0 {
            Some(MarkGlyphSetsDef::new(
                reader,
                offset + _mark_glyph_sets_def_offset as u64,
                length as u32,
            )?)
        } else {
            None
        };

        Ok(GDEF {
            major_versionn,
            minor_version,
            glyph_class_def,
            attach_list,
            lig_caret_list,
            mark_attach_class_def,
            mark_glyph_sets_def,
            // item_var_store: None,
        })
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = String::new();
        string += &format!("major_version: {}\n", self.major_versionn);
        string += &format!("minor_version: {}\n", self.minor_version);

        string += "glyph class def 1 = Base, 2 = Lig, 3 = Mark, 4 = Component\n";
        if let Some(glyph_class_def) = &self.glyph_class_def {
            string += &format!("glyph_class_def: {}\n", glyph_class_def.to_string());
        }
        if let Some(attach_list) = &self.attach_list {
            string += &format!("attach_list: {}\n", attach_list.to_string());
        }
        if let Some(lig_caret_list) = &self.lig_caret_list {
            string += &format!("lig_caret_list: {}\n", lig_caret_list.to_string());
        }
        if let Some(mark_attach_class_def) = &self.mark_attach_class_def {
            string += &format!(
                "mark_attach_class_def: {}\n",
                mark_attach_class_def.to_string()
            );
        }
        if let Some(mark_glyph_sets_def) = &self.mark_glyph_sets_def {
            string += &format!("mark_glyph_sets_def: {}\n", mark_glyph_sets_def.to_string());
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AttachPointList {
    pub(crate) coverage: Coverage,
    pub(crate) attach_point: Vec<AttachPoint>,
}

impl AttachPointList {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let coverage_offset = reader.read_u16_be()?;
        let glyph_count = reader.read_u16_be()?;
        let mut attach_point_offsets = Vec::with_capacity(glyph_count as usize);
        for _ in 0..glyph_count {
            attach_point_offsets.push(reader.read_u16_be()?);
        }
        let coverage = Coverage::new(reader, offset + coverage_offset as u64)?;
        let mut attach_point = Vec::with_capacity(glyph_count as usize);
        for offset in attach_point_offsets {
            attach_point.push(AttachPoint::new(reader, offset as u64)?);
        }
        Ok(AttachPointList {
            coverage,
            attach_point,
        })
    }

    fn to_string(&self) -> String {
        let mut string = String::new();
        string += &format!("coverage: {}\n", self.coverage.to_string());
        for attach_point in self.attach_point.iter() {
            string += &format!("attach_point: {:?}\n", attach_point.point_indices);
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AttachPoint {
    pub(crate) point_indices: Vec<u16>,
}

impl AttachPoint {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let point_count = reader.read_u16_be()?;
        let mut point_indices = Vec::with_capacity(point_count as usize);
        for _ in 0..point_count {
            point_indices.push(reader.read_u16_be()?);
        }
        Ok(AttachPoint { point_indices })
    }

    fn to_string(&self) -> String {
        let mut string = String::new();
        string += &format!("point_indices: {:?}\n", self.point_indices);
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LigatureCaretList {
    pub(crate) coverage: Coverage,
    pub(crate) lig_glyph: Vec<LigatureGlyph>,
}

impl LigatureCaretList {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let coverage_offset = reader.read_u16_be()?;
        let lig_glyph_count = reader.read_u16_be()?;
        let mut lig_glyph_offsets = Vec::with_capacity(lig_glyph_count as usize);
        for _ in 0..lig_glyph_count {
            lig_glyph_offsets.push(reader.read_u16_be()?);
        }
        let mut lig_glyph = Vec::with_capacity(lig_glyph_count as usize);
        for lig_glyph_offset in lig_glyph_offsets {
            let offset = lig_glyph_offset as u64 + offset;
            lig_glyph.push(LigatureGlyph::new(reader, offset as u64)?);
        }
        let coverage = Coverage::new(reader, offset + coverage_offset as u64)?;
        Ok(Self {
            coverage,
            lig_glyph,
        })
    }

    fn to_string(&self) -> String {
        let mut string = String::new();
        string += &format!("coverage: {}\n", self.coverage.to_string());
        for lig_glyph in self.lig_glyph.iter() {
            string += &format!("lig_glyph: {:?}\n", lig_glyph.caret_value);
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LigatureGlyph {
    pub(crate) caret_value: Vec<CaretValue>,
}

impl LigatureGlyph {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let caret_count = reader.read_u16_be()?;
        let mut caret_value_offsets = Vec::with_capacity(caret_count as usize);
        for _ in 0..caret_count {
            caret_value_offsets.push(reader.read_u16_be()?);
        }

        let mut caret_value = Vec::with_capacity(caret_count as usize);
        for caret_value_offset in caret_value_offsets {
            let offset = caret_value_offset as u64 + offset;
            reader.seek(SeekFrom::Start(offset))?;
            let format = reader.read_u16_be()?;
            match format {
                1 => {
                    let coordinate = reader.read_i16_be()?;
                    caret_value.push(CaretValue::Format1(CaretValueFormat1 { coordinate }));
                }
                2 => {
                    let caret_value_point = reader.read_u16_be()?;
                    caret_value.push(CaretValue::Format2(CaretValueFormat2 { caret_value_point }));
                }
                3 => {
                    let caret_value_point = reader.read_u16_be()?;
                    let device_table_offset = reader.read_u16_be()?;
                    let device_table = 
                        if device_table_offset == 0 {
                            None
                        } else {
                            Some(DeviceTable::new(reader, offset + device_table_offset as u64)?)
                        };
                    caret_value.push(CaretValue::Format3(CaretValueFormat3 {
                        caret_value_point,
                        device_table,
                    }));
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid caret value format",
                    ));
                }
            }
        }
        Ok(Self { caret_value })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CaretValue {
    Format1(CaretValueFormat1),
    Format2(CaretValueFormat2),
    Format3(CaretValueFormat3),
}

#[derive(Debug, Clone)]
pub(crate) struct CaretValueFormat1 {
    pub(crate) coordinate: i16,
}

#[derive(Debug, Clone)]
pub(crate) struct CaretValueFormat2 {
    pub(crate) caret_value_point: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct CaretValueFormat3 {
    pub(crate) caret_value_point: u16,
    pub(crate) device_table: Option<DeviceTable>,
}



pub(crate) struct VariationStore {}

#[derive(Debug, Clone)]
pub(crate) struct MarkGlyphSetsDef {
    pub(crate) mark_set_table_format: u16,
    pub(crate) mark_set_count: u16,
    pub(crate) coverages: Vec<Coverage>,
}

impl MarkGlyphSetsDef {
    fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        _length: u32,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let mark_set_table_format = reader.read_u16_be()?;
        let mark_set_count = reader.read_u16_be()?;
        let mut coverage_offsets = Vec::with_capacity(mark_set_count as usize);
        for _ in 0..mark_set_count {
            coverage_offsets.push(reader.read_u16_be()?);
        }
        let mut coverages = Vec::with_capacity(mark_set_count as usize);
        for coverage_offset in coverage_offsets.iter() {
            let offset = *coverage_offset as u64 + offset;
            coverages.push(Coverage::new(reader, offset as u64)?);
        }

        Ok(Self {
            mark_set_table_format,
            mark_set_count,
            coverages,
        })
    }

    fn to_string(&self) -> String {
        let mut string = String::new();
        string += &format!("mark_set_table_format: {}\n", self.mark_set_table_format);
        string += &format!("mark_set_count: {}\n", self.mark_set_count);
        for coverage in self.coverages.iter() {
            string += &format!("coverage: {}\n", coverage.to_string());
        }
        string
    }
}
