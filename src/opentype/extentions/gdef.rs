#![allow(dead_code)]

use crate::opentype::layouts::*;
use bin_rs::reader::BinaryReader;
use std::io::{Error, ErrorKind, SeekFrom};

#[derive(Debug, Clone)]
pub(crate) struct Gdef {
    pub(crate) major_versionn: u16,
    pub(crate) minor_version: u16,
    pub(crate) glyph_class_def: Option<ClassDef>,
    pub(crate) attach_list: Option<AttachPointList>,
    pub(crate) lig_caret_list: Option<LigatureCaretList>,
    pub(crate) mark_attach_class_def: Option<ClassDef>,
    // 1.2
    pub(crate) mark_glyph_sets_def: Option<MarkGlyphSetsDef>,
    // 1.3
    pub(crate) item_var_store_offset: Option<u32>,
    pub(crate) caret_variation_store: Option<
        Result<super::gdef_variation::CaretVariationStore, super::gdef_variation::VariationError>,
    >,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GlyphClass {
    Base,
    Ligature,
    Mark,
    Component,
}

impl Gdef {
    pub fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        length: usize,
    ) -> Result<Self, std::io::Error> {
        let bounds = CaretBounds::new(offset, length)?;
        bounds.span(offset, 12)?;
        reader.seek(SeekFrom::Start(offset))?;
        let major_versionn = reader.read_u16_be()?;
        let minor_version = reader.read_u16_be()?;
        let glyph_class_def_offset = reader.read_u16_be()?;
        let _attach_list_offset = reader.read_u16_be()?;
        let _lig_caret_list_offset = reader.read_u16_be()?;
        let _mark_attach_class_def_offset = reader.read_u16_be()?;
        let _mark_glyph_sets_def_offset = if minor_version >= 2 {
            bounds.span(offset, 14)?;
            reader.read_u16_be()? as u32
        } else {
            0
        };
        let _item_var_store_offset = if minor_version >= 3 {
            bounds.span(offset, 18)?;
            reader.read_u32_be()?
        } else {
            0
        };
        let glyph_class_def = if glyph_class_def_offset != 0 {
            Some(ClassDef::new(
                reader,
                offset + glyph_class_def_offset as u64,
            )?)
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
            let header_size = if minor_version >= 3 {
                18
            } else if minor_version >= 2 {
                14
            } else {
                12
            };
            let list_offset =
                bounds.relative(offset, _lig_caret_list_offset as u64, header_size, 4)?;
            let lig_caret_list = LigatureCaretList::new(reader, list_offset, bounds)?;
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

        // Optional malformed variation data must not disable legacy shaping.
        // Preserve a typed failure; never reinterpret it as a valid zero delta.
        let caret_variation_store = (_item_var_store_offset != 0).then(|| {
            use super::gdef_variation::{CaretVariationStore, VariationError, MAX_STORE_BYTES};
            let relative =
                usize::try_from(_item_var_store_offset).map_err(|_| VariationError::Malformed)?;
            let remaining = length
                .checked_sub(relative)
                .ok_or(VariationError::Malformed)?;
            if remaining > MAX_STORE_BYTES {
                return Err(VariationError::ResourceLimit);
            }
            let start = bounds
                .relative(offset, u64::from(_item_var_store_offset), 18, 8)
                .map_err(|_| VariationError::Malformed)?;
            reader
                .seek(SeekFrom::Start(start))
                .map_err(|_| VariationError::Malformed)?;
            let data = reader
                .read_bytes_as_vec(remaining)
                .map_err(|_| VariationError::Malformed)?;
            CaretVariationStore::parse(&data)
        });

        Ok(Gdef {
            major_versionn,
            minor_version,
            glyph_class_def,
            attach_list,
            lig_caret_list,
            mark_attach_class_def,
            mark_glyph_sets_def,
            item_var_store_offset: (_item_var_store_offset != 0).then_some(_item_var_store_offset),
            caret_variation_store,
        })
    }

    /// Normalized coordinates must include the caller's validated fvar/avar
    /// mapping. Point carets and device-pixel adjustments remain separate paths.
    pub(crate) fn caret_coordinate(
        &self,
        value: &CaretValue,
        coordinates: &[f32],
    ) -> Result<f32, UnsupportedCaret> {
        if let CaretValue::Format3(value) = value {
            if let Some(index) = value.device_table.as_ref().and_then(|d| d.variation_index) {
                let store = self
                    .caret_variation_store
                    .as_ref()
                    .and_then(|result| result.as_ref().ok())
                    .ok_or(UnsupportedCaret::VariationAdjustment)?;
                let delta = store
                    .delta(
                        index.delta_set_outer_index,
                        index.delta_set_inner_index,
                        coordinates,
                    )
                    .map_err(|_| UnsupportedCaret::VariationAdjustment)?;
                let coordinate = f32::from(value.coordinate) + delta;
                return if coordinate.is_finite() {
                    Ok(coordinate)
                } else {
                    Err(UnsupportedCaret::VariationAdjustment)
                };
            }
        }
        value.design_coordinate().map(f32::from)
    }

    fn dump_text(&self) -> String {
        let mut string = String::new();
        string += &format!("major_version: {}\n", self.major_versionn);
        string += &format!("minor_version: {}\n", self.minor_version);

        string += "glyph class def 1 = Base, 2 = Lig, 3 = Mark, 4 = Component\n";
        if let Some(glyph_class_def) = &self.glyph_class_def {
            string += &format!("glyph_class_def: {}\n", glyph_class_def);
        }
        if let Some(attach_list) = &self.attach_list {
            string += &format!("attach_list: {}\n", attach_list);
        }
        if let Some(lig_caret_list) = &self.lig_caret_list {
            string += &format!("lig_caret_list: {}\n", lig_caret_list);
        }
        if let Some(mark_attach_class_def) = &self.mark_attach_class_def {
            string += &format!("mark_attach_class_def: {}\n", mark_attach_class_def);
        }
        if let Some(mark_glyph_sets_def) = &self.mark_glyph_sets_def {
            string += &format!("mark_glyph_sets_def: {}\n", mark_glyph_sets_def);
        }
        string
    }

    pub(crate) fn glyph_class(&self, glyph_id: u16) -> Option<GlyphClass> {
        let class = self.glyph_class_def.as_ref()?.get_class(glyph_id);
        match class {
            1 => Some(GlyphClass::Base),
            2 => Some(GlyphClass::Ligature),
            3 => Some(GlyphClass::Mark),
            4 => Some(GlyphClass::Component),
            _ => None,
        }
    }

    pub(crate) fn is_mark_glyph(&self, glyph_id: u16) -> bool {
        self.glyph_class(glyph_id) == Some(GlyphClass::Mark)
    }

    pub(crate) fn mark_attachment_class(&self, glyph_id: u16) -> Option<u16> {
        let class = self.mark_attach_class_def.as_ref()?.get_class(glyph_id);
        if class == 0 {
            None
        } else {
            Some(class)
        }
    }

    pub(crate) fn attach_point_indices(&self, glyph_id: u16) -> Option<&[u16]> {
        let attach_list = self.attach_list.as_ref()?;
        let index = attach_list.coverage.contains(glyph_id as usize)?;
        Some(&attach_list.attach_point.get(index)?.point_indices)
    }

    /// Carets are returned in the font's Coverage / CaretValue index order.
    pub(crate) fn ligature_carets(&self, glyph_id: u16) -> Option<&[CaretValue]> {
        let list = self.lig_caret_list.as_ref()?;
        let index = list.coverage.contains(glyph_id as usize)?;
        Some(&list.lig_glyph.get(index)?.caret_value)
    }

    pub(crate) fn has_attach_points(&self, glyph_id: u16) -> bool {
        self.attach_point_indices(glyph_id)
            .map(|points| !points.is_empty())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AttachPointList {
    pub(crate) coverage: Coverage,
    pub(crate) attach_point: Vec<AttachPoint>,
}

impl AttachPointList {
    fn new<R: BinaryReader>(reader: &mut R, table_offset: u64) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(table_offset))?;
        let coverage_offset = reader.read_u16_be()?;
        let glyph_count = reader.read_u16_be()?;
        let mut attach_point_offsets = Vec::with_capacity(glyph_count as usize);
        for _ in 0..glyph_count {
            attach_point_offsets.push(reader.read_u16_be()?);
        }
        let coverage = Coverage::new(reader, table_offset + coverage_offset as u64)?;
        let mut attach_point = Vec::with_capacity(glyph_count as usize);
        for attach_point_relative_offset in attach_point_offsets {
            let attach_point_offset = table_offset
                .checked_add(attach_point_relative_offset as u64)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "GDEF AttachPoint offset overflow",
                    )
                })?;
            attach_point.push(AttachPoint::new(reader, attach_point_offset)?);
        }
        Ok(AttachPointList {
            coverage,
            attach_point,
        })
    }

    fn dump_text(&self) -> String {
        let mut string = String::new();
        string += &format!("coverage: {}\n", self.coverage);
        for attach_point in self.attach_point.iter() {
            string += &format!("attach_point: {}\n", attach_point);
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

    fn dump_text(&self) -> String {
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
    fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        bounds: CaretBounds,
    ) -> Result<Self, Error> {
        bounds.span(offset, 4)?;
        reader.seek(SeekFrom::Start(offset))?;
        let coverage_offset = reader.read_u16_be()? as u64;
        let count = reader.read_u16_be()? as usize;
        let header_size = 4 + 2 * count;
        bounds.span(offset, header_size)?;
        let mut offsets = Vec::with_capacity(count);
        for _ in 0..count {
            let relative = reader.read_u16_be()? as u64;
            offsets.push(bounds.relative(offset, relative, header_size, 2)?);
        }
        let coverage_offset = bounds.relative(offset, coverage_offset, header_size, 4)?;
        let coverage = bounded_caret_coverage(reader, coverage_offset, count, bounds)?;
        // Shared offset references must not amplify a tiny table into unbounded
        // allocations. Count work and stored Device words across all references.
        let mut budget = ((bounds.end - bounds.start) as usize)
            .saturating_mul(8)
            .min(1_000_000);
        let mut lig_glyph = Vec::with_capacity(count);
        for offset in offsets {
            // Do not sort physical offsets: array order is Coverage index order.
            lig_glyph.push(LigatureGlyph::new(reader, offset, bounds, &mut budget)?);
        }
        Ok(Self {
            coverage,
            lig_glyph,
        })
    }

    fn dump_text(&self) -> String {
        let mut string = String::new();
        string += &format!("coverage: {}\n", self.coverage);
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
    fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        bounds: CaretBounds,
        budget: &mut usize,
    ) -> Result<Self, Error> {
        bounds.span(offset, 2)?;
        reader.seek(SeekFrom::Start(offset))?;
        let count = reader.read_u16_be()? as usize;
        spend_caret_budget(budget, count + 1)?;
        let header_size = 2 + 2 * count;
        bounds.span(offset, header_size)?;
        let mut offsets = Vec::with_capacity(count);
        for _ in 0..count {
            let relative = reader.read_u16_be()? as u64;
            offsets.push(bounds.relative(offset, relative, header_size, 4)?);
        }
        let mut caret_value = Vec::with_capacity(count);
        for offset in offsets {
            reader.seek(SeekFrom::Start(offset))?;
            let format = reader.read_u16_be()?;
            caret_value.push(match format {
                1 => CaretValue::Format1(CaretValueFormat1 {
                    coordinate: reader.read_i16_be()?,
                }),
                2 => CaretValue::Format2(CaretValueFormat2 {
                    caret_value_point: reader.read_u16_be()?,
                }),
                3 => {
                    bounds.span(offset, 6)?;
                    let coordinate = reader.read_i16_be()?;
                    let relative = reader.read_u16_be()? as u64;
                    let device_table = if relative == 0 {
                        None
                    } else {
                        let device_offset = bounds.relative(offset, relative, 6, 6)?;
                        Some(bounded_caret_device(reader, device_offset, bounds, budget)?)
                    };
                    CaretValue::Format3(CaretValueFormat3 {
                        coordinate,
                        device_table,
                    })
                }
                _ => return Err(caret_error("invalid GDEF caret value format")),
            });
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
    pub(crate) coordinate: i16,
    pub(crate) device_table: Option<DeviceTable>,
}

/// Explicitly unresolved semantics; callers must not substitute zero deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnsupportedCaret {
    ContourPoint,
    DeviceAdjustment,
    VariationAdjustment,
}

impl CaretValue {
    /// Authored scalar before any Device/VariationIndex adjustment. Point
    /// indices are deliberately not interpreted as coordinates.
    pub(crate) fn unadjusted_coordinate(&self) -> Option<i16> {
        match self {
            Self::Format1(value) => Some(value.coordinate),
            Self::Format2(_) => None,
            Self::Format3(value) => Some(value.coordinate),
        }
    }

    /// Device delta in physical pixels at an explicit axis ppem. The caller
    /// converts pixels into its local coordinate units. Not a design-unit delta.
    pub(crate) fn device_pixel_delta(&self, ppem: u16) -> Result<i32, UnsupportedCaret> {
        let Self::Format3(value) = self else {
            return Err(UnsupportedCaret::DeviceAdjustment);
        };
        let device = value
            .device_table
            .as_ref()
            .ok_or(UnsupportedCaret::DeviceAdjustment)?;
        if device.is_variation_index() {
            return Err(UnsupportedCaret::VariationAdjustment);
        }
        if ppem == 0 || device.start_size > device.end_size {
            return Err(UnsupportedCaret::DeviceAdjustment);
        }
        let bits = match device.delta_format {
            1 => 2usize,
            2 => 4,
            3 => 8,
            _ => return Err(UnsupportedCaret::DeviceAdjustment),
        };
        let count = usize::from(device.end_size) - usize::from(device.start_size) + 1;
        if device.delta_value.len() != (count * bits).div_ceil(16) {
            return Err(UnsupportedCaret::DeviceAdjustment);
        }
        if ppem < device.start_size || ppem > device.end_size {
            return Ok(0);
        }
        let index = usize::from(ppem - device.start_size);
        let per_word = 16 / bits;
        let shift = 16 - bits * (index % per_word + 1);
        let mask = (1u16 << bits) - 1;
        let packed = (device.delta_value[index / per_word] >> shift) & mask;
        let value = i32::from(packed);
        Ok(if packed & (1 << (bits - 1)) != 0 {
            value - (1 << bits)
        } else {
            value
        })
    }

    pub(crate) fn design_coordinate(&self) -> Result<i16, UnsupportedCaret> {
        match self {
            Self::Format1(value) => Ok(value.coordinate),
            Self::Format2(_) => Err(UnsupportedCaret::ContourPoint),
            Self::Format3(value) => match &value.device_table {
                None => Ok(value.coordinate),
                Some(device) if device.is_variation_index() => {
                    Err(UnsupportedCaret::VariationAdjustment)
                }
                Some(_) => Err(UnsupportedCaret::DeviceAdjustment),
            },
        }
    }
}

fn caret_error(message: &'static str) -> Error {
    Error::new(ErrorKind::InvalidData, message)
}

#[derive(Clone, Copy)]
struct CaretBounds {
    start: u64,
    end: u64,
}
impl CaretBounds {
    fn new(start: u64, length: usize) -> Result<Self, Error> {
        let end = start
            .checked_add(u64::try_from(length).map_err(|_| caret_error("GDEF length overflow"))?)
            .ok_or_else(|| caret_error("GDEF length overflow"))?;
        Ok(Self { start, end })
    }
    fn span(self, offset: u64, length: usize) -> Result<(), Error> {
        if offset < self.start
            || offset
                .checked_add(length as u64)
                .is_none_or(|end| end > self.end)
        {
            return Err(caret_error("GDEF caret read outside declared table"));
        }
        Ok(())
    }
    fn relative(
        self,
        base: u64,
        relative: u64,
        minimum: usize,
        length: usize,
    ) -> Result<u64, Error> {
        if relative < minimum as u64 {
            return Err(caret_error("invalid GDEF caret relative offset"));
        }
        let offset = base
            .checked_add(relative)
            .ok_or_else(|| caret_error("GDEF caret offset overflow"))?;
        self.span(offset, length)?;
        Ok(offset)
    }
}

fn spend_caret_budget(budget: &mut usize, amount: usize) -> Result<(), Error> {
    *budget = budget
        .checked_sub(amount)
        .ok_or_else(|| caret_error("GDEF caret allocation budget exceeded"))?;
    Ok(())
}

fn bounded_caret_coverage<R: BinaryReader>(
    reader: &mut R,
    offset: u64,
    expected: usize,
    bounds: CaretBounds,
) -> Result<Coverage, Error> {
    bounds.span(offset, 4)?;
    reader.seek(SeekFrom::Start(offset))?;
    let format = reader.read_u16_be()?;
    let count = reader.read_u16_be()? as usize;
    let mut previous = None;
    match format {
        1 => {
            if count != expected {
                return Err(caret_error("GDEF caret Coverage count mismatch"));
            }
            bounds.span(offset, 4 + 2 * count)?;
            for _ in 0..count {
                let glyph = reader.read_u16_be()?;
                if previous.is_some_and(|last| glyph <= last) {
                    return Err(caret_error("unordered GDEF caret Coverage"));
                }
                previous = Some(glyph);
            }
        }
        2 => {
            bounds.span(offset, 4 + 6 * count)?;
            let mut covered = 0usize;
            for _ in 0..count {
                let start = reader.read_u16_be()?;
                let end = reader.read_u16_be()?;
                let index = reader.read_u16_be()? as usize;
                if start > end || previous.is_some_and(|last| start <= last) || index != covered {
                    return Err(caret_error("invalid GDEF caret Coverage range"));
                }
                covered += (end - start) as usize + 1;
                if covered > expected {
                    return Err(caret_error("GDEF caret Coverage count mismatch"));
                }
                previous = Some(end);
            }
            if covered != expected {
                return Err(caret_error("GDEF caret Coverage count mismatch"));
            }
        }
        _ => return Err(caret_error("invalid GDEF caret Coverage format")),
    }
    Coverage::new(reader, offset)
}

fn bounded_caret_device<R: BinaryReader>(
    reader: &mut R,
    offset: u64,
    bounds: CaretBounds,
    budget: &mut usize,
) -> Result<DeviceTable, Error> {
    bounds.span(offset, 6)?;
    reader.seek(SeekFrom::Start(offset))?;
    let start_size = reader.read_u16_be()?;
    let end_size = reader.read_u16_be()?;
    let delta_format = reader.read_u16_be()?;
    let (word_count, variation_index) = match delta_format {
        1..=3 => {
            if start_size > end_size {
                return Err(caret_error("invalid GDEF caret Device size range"));
            }
            let count = (end_size - start_size) as usize + 1;
            ((count * (1usize << delta_format)).div_ceil(16), None)
        }
        0x8000 => (
            0,
            Some(crate::opentype::layouts::device::VariationIndex {
                delta_set_outer_index: start_size,
                delta_set_inner_index: end_size,
            }),
        ),
        _ => return Err(caret_error("invalid GDEF caret Device delta format")),
    };
    bounds.span(offset, 6 + 2 * word_count)?;
    spend_caret_budget(budget, word_count + 1)?;
    let mut delta_value = Vec::with_capacity(word_count);
    for _ in 0..word_count {
        delta_value.push(reader.read_u16_be()?);
    }
    Ok(DeviceTable {
        start_size,
        end_size,
        delta_format,
        delta_value,
        variation_index,
    })
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
        table_offset: u64,
        _length: u32,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(table_offset))?;
        let mark_set_table_format = reader.read_u16_be()?;
        let mark_set_count = reader.read_u16_be()?;
        let mut coverage_offsets = Vec::with_capacity(mark_set_count as usize);
        for _ in 0..mark_set_count {
            coverage_offsets.push(reader.read_u32_be()?);
        }
        let mut coverages = Vec::with_capacity(mark_set_count as usize);
        for coverage_offset in coverage_offsets.iter() {
            let coverage_offset = table_offset
                .checked_add(*coverage_offset as u64)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "GDEF MarkGlyphSets coverage offset overflow",
                    )
                })?;
            coverages.push(Coverage::new(reader, coverage_offset)?);
        }

        Ok(Self {
            mark_set_table_format,
            mark_set_count,
            coverages,
        })
    }

    fn dump_text(&self) -> String {
        let mut string = String::new();
        string += &format!("mark_set_table_format: {}\n", self.mark_set_table_format);
        string += &format!("mark_set_count: {}\n", self.mark_set_count);
        for coverage in self.coverages.iter() {
            string += &format!("coverage: {}\n", coverage);
        }
        string
    }
}

impl std::fmt::Display for Gdef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

impl std::fmt::Display for AttachPointList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

impl std::fmt::Display for AttachPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

impl std::fmt::Display for LigatureCaretList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

impl std::fmt::Display for MarkGlyphSetsDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

#[cfg(test)]
mod caret_tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    fn bytes(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|v| v.to_be_bytes()).collect()
    }
    fn two_ligatures() -> Vec<u8> {
        // Coverage 10,20; physical records 20,10. Never sort the offsets.
        bytes(&[
            1, 0, 0, 0, 12, 0, 8, 2, 24, 16, 1, 2, 10, 20, 1, 4, 1, 730, 1, 4, 1, 220,
        ])
    }
    fn one_caret(record: &[u16]) -> Vec<u8> {
        let mut words = vec![1, 0, 0, 0, 12, 0, 6, 1, 12, 1, 1, 10, 1, 4];
        words.extend(record);
        bytes(&words)
    }
    fn parse(data: &[u8]) -> Result<Gdef, Error> {
        Gdef::new(&mut BytesReader::new(data), 0, data.len())
    }
    fn set(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn caret_coverage_order_is_not_physical_order() {
        let data = two_ligatures();
        let gdef = parse(&data).unwrap();
        assert_eq!(
            gdef.ligature_carets(10).unwrap()[0].design_coordinate(),
            Ok(220)
        );
        assert_eq!(
            gdef.ligature_carets(20).unwrap()[0].design_coordinate(),
            Ok(730)
        );
        assert!(gdef.ligature_carets(11).is_none());
        // The CaretValue array itself also follows logical index order.
        let data = bytes(&[
            1, 0, 0, 0, 12, 0, 6, 1, 12, 1, 1, 10, 2, 10, 6, 1, 700, 1, 200,
        ]);
        let parsed = parse(&data).unwrap();
        assert_eq!(
            parsed
                .ligature_carets(10)
                .unwrap()
                .iter()
                .map(CaretValue::design_coordinate)
                .collect::<Vec<_>>(),
            vec![Ok(200), Ok(700)]
        );
        let data = two_ligatures();
        // A nonzero enclosing table origin preserves relative offsets.
        let mut container = vec![0x55; 17];
        container.extend(&data);
        let parsed = Gdef::new(&mut BytesReader::new(&container), 17, data.len()).unwrap();
        assert_eq!(
            parsed.ligature_carets(10).unwrap()[0].design_coordinate(),
            Ok(220)
        );
    }

    #[test]
    fn caret_signed_format3_point_index_and_adjustments_are_distinct() {
        for coordinate in [i16::MIN, -90, 0, i16::MAX] {
            let simple = parse(&one_caret(&[1, coordinate as u16])).unwrap();
            assert_eq!(
                simple.ligature_carets(10).unwrap()[0].design_coordinate(),
                Ok(coordinate)
            );
            let data = one_caret(&[3, coordinate as u16, 0]);
            let gdef = parse(&data).unwrap();
            assert_eq!(
                gdef.ligature_carets(10).unwrap()[0].design_coordinate(),
                Ok(coordinate)
            );
        }
        let data = one_caret(&[2, 65535]);
        let gdef = parse(&data).unwrap();
        let value = &gdef.ligature_carets(10).unwrap()[0];
        assert_eq!(
            value.design_coordinate(),
            Err(UnsupportedCaret::ContourPoint)
        );
        let CaretValue::Format2(point) = value else {
            panic!()
        };
        assert_eq!(point.caret_value_point, 65535);
        for format in 1..=3 {
            let data = one_caret(&[3, -90i16 as u16, 6, 20, 20, format, 0x4000]);
            let gdef = parse(&data).unwrap();
            let value = &gdef.ligature_carets(10).unwrap()[0];
            assert_eq!(
                value.design_coordinate(),
                Err(UnsupportedCaret::DeviceAdjustment)
            );
            let CaretValue::Format3(caret) = value else {
                panic!()
            };
            assert_eq!(caret.coordinate, -90);
            let device = caret.device_table.as_ref().unwrap();
            assert_eq!(
                (device.start_size, device.end_size, device.delta_format),
                (20, 20, format)
            );
            assert_eq!(device.delta_value, [0x4000]);
        }
        let data = one_caret(&[3, -90i16 as u16, 6, 1, 7, 0x8000]);
        let gdef = parse(&data).unwrap();
        let value = &gdef.ligature_carets(10).unwrap()[0];
        assert_eq!(
            value.design_coordinate(),
            Err(UnsupportedCaret::VariationAdjustment)
        );
        let CaretValue::Format3(caret) = value else {
            panic!()
        };
        let index = caret
            .device_table
            .as_ref()
            .unwrap()
            .variation_index
            .unwrap();
        assert_eq!(
            (index.delta_set_outer_index, index.delta_set_inner_index),
            (1, 7)
        );
    }

    #[test]
    fn caret_bad_offsets_counts_formats_and_declared_bounds_are_rejected() {
        for (offset, value) in [
            (8, 1),
            (8, 65535),
            (12, 0),
            (12, 65535),
            (14, 65535),
            (16, 0),
            (16, 65535),
            (22, 1),
            (24, 20),
            (28, 65535),
            (30, 0),
            (30, 65535),
            (32, 4),
        ] {
            let mut data = two_ligatures();
            set(&mut data, offset, value);
            assert!(parse(&data).is_err(), "offset {offset} value {value}");
        }
        let data = two_ligatures();
        for length in 0..data.len() {
            // Even if the reader has trailing accessible bytes, length is binding.
            assert!(
                Gdef::new(&mut BytesReader::new(&data), 0, length).is_err(),
                "length {length}"
            );
        }
        assert!(Gdef::new(&mut BytesReader::new(&[]), u64::MAX, 1).is_err());
        for record in [
            &[3, 0, 2][..],
            &[3, 0, 6, 21, 20, 1, 0],
            &[3, 0, 6, 20, 20, 4, 0],
            &[3, 0, 6, 20, 100, 3, 0],
            &[3, 0, 65535],
            &[3, 0, 6, 20, 20, 0x8000][..2],
        ] {
            assert!(parse(&one_caret(record)).is_err(), "record {record:?}");
        }
    }

    #[test]
    fn caret_coverage_ranges_require_sorted_contiguous_indices() {
        let data = bytes(&[
            1, 0, 0, 0, 12, 0, 8, 2, 26, 18, 2, 1, 10, 11, 0, 1, 4, 1, 730, 1, 4, 1, 220,
        ]);
        let gdef = parse(&data).unwrap();
        assert_eq!(
            gdef.ligature_carets(10).unwrap()[0].design_coordinate(),
            Ok(220)
        );
        assert_eq!(
            gdef.ligature_carets(11).unwrap()[0].design_coordinate(),
            Ok(730)
        );
        for (offset, value) in [(22, 65535), (24, 12), (26, 9), (26, 12), (28, 1)] {
            let mut malformed = data.clone();
            set(&mut malformed, offset, value);
            assert!(parse(&malformed).is_err(), "offset {offset} value {value}");
        }
    }

    #[test]
    fn caret_fixture_preserves_coordinates_indices_and_unsupported_adjustments() {
        let font = crate::fontreader::Font::get_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/gdef-carets.ttc"
        ))
        .unwrap();
        for face in 0..6 {
            let selected = if face == 0 {
                &font
            } else {
                &font.more_fonts[face - 1]
            };
            let gdef = selected.gdef.as_ref().unwrap();
            for (glyph, which) in [(6, 0), (7, 1)] {
                let carets = gdef.ligature_carets(glyph).unwrap();
                assert_eq!(carets.len(), 1);
                let expected = match face {
                    0 => Ok([220, 730][which]),
                    1 => Err(UnsupportedCaret::ContourPoint),
                    2 => Ok([-90, -160][which]),
                    3 => Err(UnsupportedCaret::DeviceAdjustment),
                    _ => Err(UnsupportedCaret::VariationAdjustment),
                };
                assert_eq!(
                    carets[0].design_coordinate(),
                    expected,
                    "face {face} glyph {glyph}"
                );
            }
        }
    }

    #[test]
    fn caret_alias_amplification_has_a_shared_allocation_budget() {
        let bounds = CaretBounds::new(0, 8).unwrap();
        let data = bytes(&[1, 4, 1, 220]);
        let mut reader = BytesReader::new(&data);
        let mut budget = 2;
        assert!(LigatureGlyph::new(&mut reader, 0, bounds, &mut budget).is_ok());
        assert!(LigatureGlyph::new(&mut reader, 0, bounds, &mut budget).is_err());
    }
    #[test]
    fn fixture_variation_carets_resolve_normalized_and_reject_invalid_indices() {
        let font = crate::fontreader::Font::get_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/gdef-carets.ttc"
        ))
        .unwrap();
        let valid = font.more_fonts[3].gdef.as_ref().unwrap(); // face 4
        for (glyph, base) in [(6, 220.0), (7, 730.0)] {
            let caret = &valid.ligature_carets(glyph).unwrap()[0];
            for (coordinate, delta) in [(0.0, 0.0), (0.25, 25.0), (1.0, 100.0)] {
                assert_eq!(
                    valid.caret_coordinate(caret, &[coordinate]),
                    Ok(base + delta)
                );
            }
            assert_eq!(
                valid.caret_coordinate(caret, &[]),
                Err(UnsupportedCaret::VariationAdjustment)
            );
            let invalid = font.more_fonts[4].gdef.as_ref().unwrap(); // face 5
            assert_eq!(
                invalid.caret_coordinate(&invalid.ligature_carets(glyph).unwrap()[0], &[1.0]),
                Err(UnsupportedCaret::VariationAdjustment)
            );
        }
    }

    #[test]
    fn optional_malformed_variation_store_preserves_static_carets() {
        let mut data = two_ligatures();
        // Upgrade header by moving all body bytes six bytes and fixing its
        // sole header-relative LigCaret offset; internal offsets stay unchanged.
        data[2..4].copy_from_slice(&3u16.to_be_bytes());
        data.splice(12..12, [0, 0, 0xff, 0xff, 0xff, 0xff]);
        data[8..10].copy_from_slice(&18u16.to_be_bytes());
        let parsed = parse(&data).unwrap();
        assert!(parsed.caret_variation_store.as_ref().unwrap().is_err());
        assert_eq!(
            parsed.caret_coordinate(&parsed.ligature_carets(10).unwrap()[0], &[]),
            Ok(220.0)
        );
    }
    #[test]
    fn device_signed_pixels_all_formats_boundaries_and_invalid_context() {
        for (format, bits, values) in [
            (1, 2, vec![-2, -1, 0, 1, -2, -1, 0, 1, -2]),
            (2, 4, vec![-8, -1, 0, 7, -8]),
            (3, 8, vec![-128, 127, -1]),
        ] {
            let mut words = vec![0u16; (values.len() * bits).div_ceil(16)];
            for (i, &value) in values.iter().enumerate() {
                words[i / (16 / bits)] |=
                    ((value as u16) & ((1 << bits) - 1)) << (16 - bits * (i % (16 / bits) + 1));
            }
            let caret = CaretValue::Format3(CaretValueFormat3 {
                coordinate: 100,
                device_table: Some(DeviceTable {
                    start_size: 20,
                    end_size: 20 + values.len() as u16 - 1,
                    delta_format: format,
                    delta_value: words,
                    variation_index: None,
                }),
            });
            assert_eq!(caret.unadjusted_coordinate(), Some(100));
            assert_eq!(
                caret.device_pixel_delta(0),
                Err(UnsupportedCaret::DeviceAdjustment)
            );
            assert_eq!(caret.device_pixel_delta(19), Ok(0));
            for (i, value) in values.iter().enumerate() {
                assert_eq!(caret.device_pixel_delta(20 + i as u16), Ok(*value));
            }
            assert_eq!(caret.device_pixel_delta(20 + values.len() as u16), Ok(0));
        }
    }
}
