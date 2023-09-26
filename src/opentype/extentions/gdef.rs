use crate::opentype::layouts::*;
use bin_rs::reader::BinaryReader;
use std::io::SeekFrom;

pub(crate) struct GDEF {
    pub(crate) major_versionn: u16,
    pub(crate) minor_version: u16,
    pub(crate) glyph_class_def: ClassDefinition,
    pub(crate) attach_list: Option<AttachList>,
    pub(crate) lig_caret_list: Option<LigatureCaretList>,
    pub(crate) mark_attach_class_def: Option<ClassDefinition>,
    // 1.2
    pub(crate) mark_glyph_sets_def: Option<MarkGlyphSetsDef>,
    // 1.3
    pub(crate) item_var_store: Option<VariationStore>,
}

impl GDEF {
    pub fn new<R: BinaryReader>(reader: &mut R, offset: u64, length: usize) -> Self {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let major_versionn = reader.read_u16_be().unwrap();
        let minor_version = reader.read_u16_be().unwrap();
        let glyph_class_def_offset = reader.read_u16_be().unwrap();
        let _attach_list_offset = reader.read_u16_be().unwrap();
        let _lig_caret_list_offset = reader.read_u16_be().unwrap();
        let _mark_attach_class_def_offset = reader.read_u16_be().unwrap();
        let _mark_glyph_sets_def_offset = if minor_version >= 2 {
            reader.read_u16_be().unwrap()
        } else {
            0
        };
        let _item_var_store_offset = if minor_version >= 3 {
            reader.read_u16_be().unwrap()
        } else {
            0
        };
        let glyph_class_def = ClassDefinition::new(
            reader,
            offset + glyph_class_def_offset as u64,
            length as u32,
        );
        GDEF {
            major_versionn,
            minor_version,
            glyph_class_def,
            attach_list: None,
            lig_caret_list: None,
            mark_attach_class_def: None,
            mark_glyph_sets_def: None,
            item_var_store: None,
        }
    }
}

pub(crate) struct AttachList {
    pub(crate) coverage: Coverage,
    pub(crate) attach_point: Vec<AttachPoint>,
}

pub(crate) struct AttachPoint {
    pub(crate) point_indices: Vec<u16>,
}

pub(crate) struct LigatureCaretList {
    pub(crate) coverage: Coverage,
    pub(crate) lig_glyph: Vec<LigatureGlyph>,
}

pub(crate) struct LigatureGlyph {
    pub(crate) caret_value: Vec<CaretValue>,
}

pub(crate) enum CaretValue {
    Format1(CaretValueFormat1),
    Format2(CaretValueFormat2),
    Format3(CaretValueFormat3),
}

pub(crate) struct CaretValueFormat1 {
    pub(crate) coordinate: i16,
}

pub(crate) struct CaretValueFormat2 {
    pub(crate) caret_value_point: u16,
}

pub(crate) struct CaretValueFormat3 {
    pub(crate) caret_value_point: u16,
    pub(crate) device_table: Option<DeviceTable>,
}

// Device and VariationIndex Tables from layouts::device.rs
pub(crate) struct DeviceTable {}

pub(crate) struct MarkGlyphSetsDef {}

pub(crate) struct VariationStore {}
