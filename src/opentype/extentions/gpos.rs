#![allow(dead_code)]

use std::collections::HashSet;
use std::io::SeekFrom;

use crate::opentype::layouts::{
    classdef::ClassDef, coverage::Coverage, script::ParsedScript, FeatureList,
    FeatureVariationList, ScriptList,
};
use bin_rs::reader::BinaryReader;

use super::gdef::{Gdef, GlyphClass};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ValueRecord {
    pub(crate) x_placement: i16,
    pub(crate) y_placement: i16,
    pub(crate) x_advance: i16,
    pub(crate) y_advance: i16,
    pub(crate) x_pla_var: Option<(u16, u16)>,
    pub(crate) y_pla_var: Option<(u16, u16)>,
    pub(crate) x_adv_var: Option<(u16, u16)>,
    pub(crate) y_adv_var: Option<(u16, u16)>,
}

impl ValueRecord {
    fn is_zero(self) -> bool {
        self == Self::default()
    }

    fn add_assign(&mut self, other: Self) {
        self.x_placement = self.x_placement.saturating_add(other.x_placement);
        self.y_placement = self.y_placement.saturating_add(other.y_placement);
        self.x_advance = self.x_advance.saturating_add(other.x_advance);
        self.y_advance = self.y_advance.saturating_add(other.y_advance);
    }

    fn resolve(self, gdef: Option<&Gdef>, variation_coords: &[f32]) -> Self {
        if variation_coords.is_empty() {
            return self;
        }
        let Some(store) = gdef
            .and_then(|g| g.caret_variation_store.as_ref())
            .and_then(|res| res.as_ref().ok())
        else {
            return self;
        };
        let apply = |base: i16, var: Option<(u16, u16)>| -> i16 {
            let Some((outer, inner)) = var else {
                return base;
            };
            let Ok(delta) = store.delta(outer, inner, variation_coords) else {
                return base;
            };
            if !delta.is_finite() {
                return base;
            }
            let rounded = delta
                .round()
                .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16;
            base.saturating_add(rounded)
        };
        Self {
            x_placement: apply(self.x_placement, self.x_pla_var),
            y_placement: apply(self.y_placement, self.y_pla_var),
            x_advance: apply(self.x_advance, self.x_adv_var),
            y_advance: apply(self.y_advance, self.y_adv_var),
            x_pla_var: None,
            y_pla_var: None,
            x_adv_var: None,
            y_adv_var: None,
        }
    }

    fn parse_variation_index<R: BinaryReader>(
        reader: &mut R,
        subtable_offset: u64,
        device_offset: u16,
    ) -> Option<(u16, u16)> {
        if device_offset == 0 {
            return None;
        }
        let saved = reader.offset().ok()?;
        let target = subtable_offset.checked_add(u64::from(device_offset))?;
        let parsed = (|| -> Option<(u16, u16)> {
            reader.seek(SeekFrom::Start(target)).ok()?;
            let outer = reader.read_u16_be().ok()?;
            let inner = reader.read_u16_be().ok()?;
            let delta_format = reader.read_u16_be().ok()?;
            (delta_format == 0x8000).then_some((outer, inner))
        })();
        let _ = reader.seek(SeekFrom::Start(saved));
        parsed
    }

    fn parse_with_base<R: BinaryReader>(
        reader: &mut R,
        value_format: u16,
        subtable_offset: u64,
    ) -> Result<Self, std::io::Error> {
        let mut value = Self::default();

        if value_format & 0x0001 != 0 {
            value.x_placement = reader.read_i16_be()?;
        }
        if value_format & 0x0002 != 0 {
            value.y_placement = reader.read_i16_be()?;
        }
        if value_format & 0x0004 != 0 {
            value.x_advance = reader.read_i16_be()?;
        }
        if value_format & 0x0008 != 0 {
            value.y_advance = reader.read_i16_be()?;
        }
        if value_format & 0x0010 != 0 {
            let rel = reader.read_u16_be()?;
            value.x_pla_var = Self::parse_variation_index(reader, subtable_offset, rel);
        }
        if value_format & 0x0020 != 0 {
            let rel = reader.read_u16_be()?;
            value.y_pla_var = Self::parse_variation_index(reader, subtable_offset, rel);
        }
        if value_format & 0x0040 != 0 {
            let rel = reader.read_u16_be()?;
            value.x_adv_var = Self::parse_variation_index(reader, subtable_offset, rel);
        }
        if value_format & 0x0080 != 0 {
            let rel = reader.read_u16_be()?;
            value.y_adv_var = Self::parse_variation_index(reader, subtable_offset, rel);
        }
        for flag in [0x0100u16, 0x0200, 0x0400, 0x0800] {
            if value_format & flag != 0 {
                let _ = reader.read_u16_be()?;
            }
        }

        Ok(value)
    }
}

// Table fields are i16, but totals across distinct lookups are not.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SingleAdjustment {
    pub(crate) x_placement: i32,
    pub(crate) y_placement: i32,
    pub(crate) x_advance: i32,
    pub(crate) y_advance: i32,
}

impl SingleAdjustment {
    fn add_assign(&mut self, value: ValueRecord) {
        self.x_placement += i32::from(value.x_placement);
        self.y_placement += i32::from(value.y_placement);
        self.x_advance += i32::from(value.x_advance);
        self.y_advance += i32::from(value.y_advance);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PairAdjustment {
    pub(crate) first: ValueRecord,
    pub(crate) second: ValueRecord,
}

impl PairAdjustment {
    fn add_assign(&mut self, other: Self) {
        self.first.add_assign(other.first);
        self.second.add_assign(other.second);
    }

    fn is_zero(self) -> bool {
        self.first.is_zero() && self.second.is_zero()
    }
}

#[derive(Debug, Clone)]
struct SinglePos {
    coverage: Coverage,
    // Format 1 shares one value; Format 2 indexes values by coverage index.
    values: Vec<ValueRecord>,
    shared: bool,
}

#[derive(Debug, Clone)]
struct PairValueRecord {
    second_glyph: u16,
    value1: ValueRecord,
    value2: ValueRecord,
}

#[derive(Debug, Clone)]
struct PairSet {
    pair_value_records: Vec<PairValueRecord>,
}

#[derive(Debug, Clone)]
struct PairPosFormat1 {
    second_is_present: bool,
    coverage: Coverage,
    pair_sets: Vec<PairSet>,
}

#[derive(Debug, Clone)]
struct Class2Record {
    value1: ValueRecord,
    value2: ValueRecord,
}

#[derive(Debug, Clone)]
struct PairPosFormat2 {
    second_is_present: bool,
    coverage: Coverage,
    class_def1: ClassDef,
    class_def2: ClassDef,
    class1_records: Vec<Vec<Class2Record>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Anchor {
    x: i16,
    y: i16,
}

#[derive(Debug, Clone)]
struct MarkRecord {
    mark_class: u16,
    mark_anchor: Anchor,
}

#[derive(Debug, Clone)]
struct MarkArray {
    mark_records: Vec<MarkRecord>,
}

#[derive(Debug, Clone)]
struct BaseRecord {
    base_anchors: Vec<Option<Anchor>>,
}

#[derive(Debug, Clone)]
struct BaseArray {
    base_records: Vec<BaseRecord>,
}

#[derive(Debug, Clone)]
struct MarkToBaseFormat1 {
    mark_coverage: Coverage,
    base_coverage: Coverage,
    mark_array: MarkArray,
    base_array: BaseArray,
}

#[derive(Debug, Clone)]
struct ComponentRecord {
    ligature_anchors: Vec<Option<Anchor>>,
}

#[derive(Debug, Clone)]
struct LigatureAttach {
    component_records: Vec<ComponentRecord>,
}

#[derive(Debug, Clone)]
struct LigatureArray {
    ligature_attaches: Vec<LigatureAttach>,
}

#[derive(Debug, Clone)]
struct MarkToLigatureFormat1 {
    mark_coverage: Coverage,
    ligature_coverage: Coverage,
    mark_array: MarkArray,
    ligature_array: LigatureArray,
}

#[derive(Debug, Clone)]
struct Mark2Record {
    mark2_anchors: Vec<Option<Anchor>>,
}

#[derive(Debug, Clone)]
struct Mark2Array {
    mark2_records: Vec<Mark2Record>,
}

#[derive(Debug, Clone)]
struct MarkToMarkFormat1 {
    mark1_coverage: Coverage,
    mark2_coverage: Coverage,
    mark1_array: MarkArray,
    mark2_array: Mark2Array,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MarkAttachmentAdjustment {
    pub(crate) x_placement: i16,
    pub(crate) y_placement: i16,
}

#[derive(Debug, Clone)]
struct CursivePosFormat1 {
    coverage: Coverage,
    records: Vec<(Option<Anchor>, Option<Anchor>)>,
}

#[derive(Debug, Clone)]
enum PositioningSubtable {
    Single(SinglePos),
    PairFormat1(PairPosFormat1),
    PairFormat2(PairPosFormat2),
    CursiveFormat1(CursivePosFormat1),
    MarkToBaseFormat1(MarkToBaseFormat1),
    MarkToLigatureFormat1(MarkToLigatureFormat1),
    MarkToMarkFormat1(MarkToMarkFormat1),
    Extension(Box<PositioningSubtable>),
    Unsupported,
}

impl PositioningSubtable {
    fn lookup_single_adjustment(&self, glyph: u16) -> Option<ValueRecord> {
        match self {
            Self::Single(single) => {
                let index = single.coverage.contains(glyph as usize)?;
                single
                    .values
                    .get(if single.shared { 0 } else { index })
                    .copied()
            }
            Self::Extension(extension) => extension.lookup_single_adjustment(glyph),
            _ => None,
        }
    }

    // The presence of ValueRecord2, not its values, controls cursor movement.
    fn pair_consumes_second(&self) -> bool {
        match self {
            Self::PairFormat1(pair) => pair.second_is_present,
            Self::PairFormat2(pair) => pair.second_is_present,
            Self::Extension(inner) => inner.pair_consumes_second(),
            _ => false,
        }
    }

    fn lookup_pair_adjustment(&self, left: u16, right: u16) -> Option<PairAdjustment> {
        match self {
            PositioningSubtable::PairFormat1(pair) => {
                let coverage_index = pair.coverage.contains(left as usize)?;
                let pair_set = pair.pair_sets.get(coverage_index)?;
                let record = pair_set
                    .pair_value_records
                    .iter()
                    .find(|record| record.second_glyph == right)?;
                Some(PairAdjustment {
                    first: record.value1,
                    second: record.value2,
                })
            }
            PositioningSubtable::PairFormat2(pair) => {
                pair.coverage.contains(left as usize)?;
                let class1 = pair.class_def1.get_class(left) as usize;
                let class2 = pair.class_def2.get_class(right) as usize;
                let class1_record = pair.class1_records.get(class1)?;
                let class2_record = class1_record.get(class2)?;
                Some(PairAdjustment {
                    first: class2_record.value1,
                    second: class2_record.value2,
                })
            }
            PositioningSubtable::Extension(extension) => {
                extension.lookup_pair_adjustment(left, right)
            }
            PositioningSubtable::Single(_)
            | PositioningSubtable::Unsupported
            | PositioningSubtable::CursiveFormat1(_)
            | PositioningSubtable::MarkToBaseFormat1(_)
            | PositioningSubtable::MarkToLigatureFormat1(_)
            | PositioningSubtable::MarkToMarkFormat1(_) => None,
        }
    }

    fn lookup_cursive_anchors(&self, glyph: u16) -> Option<(Option<Anchor>, Option<Anchor>)> {
        match self {
            PositioningSubtable::CursiveFormat1(cursive) => {
                let index = cursive.coverage.contains(glyph as usize)?;
                cursive.records.get(index).copied()
            }
            PositioningSubtable::Extension(extension) => extension.lookup_cursive_anchors(glyph),
            _ => None,
        }
    }

    fn lookup_mark_to_base_adjustment(
        &self,
        base: u16,
        mark: u16,
    ) -> Option<MarkAttachmentAdjustment> {
        match self {
            PositioningSubtable::MarkToBaseFormat1(mark_to_base) => {
                let base_index = mark_to_base.base_coverage.contains(base as usize)?;
                let mark_index = mark_to_base.mark_coverage.contains(mark as usize)?;
                let mark_record = mark_to_base.mark_array.mark_records.get(mark_index)?;
                let base_record = mark_to_base.base_array.base_records.get(base_index)?;
                let base_anchor = base_record
                    .base_anchors
                    .get(mark_record.mark_class as usize)?
                    .as_ref()?;
                Some(MarkAttachmentAdjustment {
                    x_placement: base_anchor.x.saturating_sub(mark_record.mark_anchor.x),
                    y_placement: base_anchor.y.saturating_sub(mark_record.mark_anchor.y),
                })
            }
            PositioningSubtable::Extension(extension) => {
                extension.lookup_mark_to_base_adjustment(base, mark)
            }
            PositioningSubtable::PairFormat1(_)
            | PositioningSubtable::PairFormat2(_)
            | PositioningSubtable::CursiveFormat1(_)
            | PositioningSubtable::MarkToLigatureFormat1(_)
            | PositioningSubtable::MarkToMarkFormat1(_)
            | PositioningSubtable::Single(_)
            | PositioningSubtable::Unsupported => None,
        }
    }

    fn lookup_mark_to_ligature_adjustment(
        &self,
        ligature: u16,
        mark: u16,
        component_index: usize,
    ) -> Option<MarkAttachmentAdjustment> {
        match self {
            PositioningSubtable::MarkToLigatureFormat1(mark_to_ligature) => {
                let ligature_index = mark_to_ligature
                    .ligature_coverage
                    .contains(ligature as usize)?;
                let mark_index = mark_to_ligature.mark_coverage.contains(mark as usize)?;
                let mark_record = mark_to_ligature.mark_array.mark_records.get(mark_index)?;
                let ligature_attach = mark_to_ligature
                    .ligature_array
                    .ligature_attaches
                    .get(ligature_index)?;
                let component_record = ligature_attach.component_records.get(component_index)?;
                let ligature_anchor = component_record
                    .ligature_anchors
                    .get(mark_record.mark_class as usize)?
                    .as_ref()?;
                Some(MarkAttachmentAdjustment {
                    x_placement: ligature_anchor.x.saturating_sub(mark_record.mark_anchor.x),
                    y_placement: ligature_anchor.y.saturating_sub(mark_record.mark_anchor.y),
                })
            }
            PositioningSubtable::Extension(extension) => {
                extension.lookup_mark_to_ligature_adjustment(ligature, mark, component_index)
            }
            PositioningSubtable::PairFormat1(_)
            | PositioningSubtable::PairFormat2(_)
            | PositioningSubtable::CursiveFormat1(_)
            | PositioningSubtable::MarkToBaseFormat1(_)
            | PositioningSubtable::MarkToMarkFormat1(_)
            | PositioningSubtable::Single(_)
            | PositioningSubtable::Unsupported => None,
        }
    }

    fn lookup_mark_to_mark_adjustment(
        &self,
        base_mark: u16,
        combining_mark: u16,
    ) -> Option<MarkAttachmentAdjustment> {
        match self {
            PositioningSubtable::MarkToMarkFormat1(mark_to_mark) => {
                let base_mark_index = mark_to_mark.mark2_coverage.contains(base_mark as usize)?;
                let combining_mark_index = mark_to_mark
                    .mark1_coverage
                    .contains(combining_mark as usize)?;
                let mark1_record = mark_to_mark
                    .mark1_array
                    .mark_records
                    .get(combining_mark_index)?;
                let mark2_record = mark_to_mark
                    .mark2_array
                    .mark2_records
                    .get(base_mark_index)?;
                let mark2_anchor = mark2_record
                    .mark2_anchors
                    .get(mark1_record.mark_class as usize)?
                    .as_ref()?;
                Some(MarkAttachmentAdjustment {
                    x_placement: mark2_anchor.x.saturating_sub(mark1_record.mark_anchor.x),
                    y_placement: mark2_anchor.y.saturating_sub(mark1_record.mark_anchor.y),
                })
            }
            PositioningSubtable::Extension(extension) => {
                extension.lookup_mark_to_mark_adjustment(base_mark, combining_mark)
            }
            PositioningSubtable::PairFormat1(_)
            | PositioningSubtable::PairFormat2(_)
            | PositioningSubtable::CursiveFormat1(_)
            | PositioningSubtable::MarkToBaseFormat1(_)
            | PositioningSubtable::MarkToLigatureFormat1(_)
            | PositioningSubtable::Single(_)
            | PositioningSubtable::Unsupported => None,
        }
    }
}

#[derive(Debug, Clone)]
struct PositioningLookup {
    lookup_type: u16,
    lookup_flag: u16,
    mark_filtering_set: Option<u16>,
    subtables: Vec<PositioningSubtable>,
}

impl PositioningLookup {
    // Filter lookup inputs, not attachment targets (whose search has separate rules).
    fn includes_glyph(&self, glyph: u16, gdef: Option<&Gdef>) -> bool {
        let Some(gdef) = gdef else { return true };
        match gdef.glyph_class(glyph) {
            Some(GlyphClass::Base) => self.lookup_flag & 0x0002 == 0,
            Some(GlyphClass::Ligature) => self.lookup_flag & 0x0004 == 0,
            Some(GlyphClass::Mark) => {
                if self.lookup_flag & 0x0008 != 0 {
                    return false;
                }
                self.includes_mark_attachment_class(glyph, gdef)
            }
            _ => true,
        }
    }

    // Mkmk targets ignore the three IgnoreFlags, but retain set/class filtering.
    fn includes_mark_attachment_class(&self, glyph: u16, gdef: &Gdef) -> bool {
        if self.lookup_flag & 0x0010 != 0 {
            return self
                .mark_filtering_set
                .and_then(|index| {
                    gdef.mark_glyph_sets_def
                        .as_ref()?
                        .coverages
                        .get(index as usize)
                })
                .is_some_and(|coverage| coverage.contains(glyph as usize).is_some());
        }
        let attachment_type = self.lookup_flag >> 8;
        attachment_type == 0 || gdef.mark_attachment_class(glyph) == Some(attachment_type)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Gpos {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) scripts: Box<ScriptList>,
    pub(crate) features: Box<FeatureList>,
    lookups: Vec<PositioningLookup>,
    pub(crate) feature_variations: Option<Box<FeatureVariationList>>,
}

impl Gpos {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, std::io::Error> {
        let offset = offset as u64;
        reader.seek(SeekFrom::Start(offset))?;
        let major_version = reader.read_u16_be()?;
        let minor_version = reader.read_u16_be()?;
        let script_list_offset = reader.read_u16_be()?;
        let feature_list_offset = reader.read_u16_be()?;
        let lookup_list_offset = reader.read_u16_be()?;
        let feature_variations_offset = if major_version == 1 && minor_version == 1 {
            reader.read_u32_be()?
        } else {
            0
        };

        let scripts = Box::new(ScriptList::new(
            reader,
            offset + script_list_offset as u64,
            length,
        )?);
        let features = Box::new(FeatureList::new(
            reader,
            offset + feature_list_offset as u64,
            offset + length as u64,
        )?);
        let lookups = Self::parse_lookups(reader, offset + lookup_list_offset as u64)?;
        let feature_variations = if feature_variations_offset > 0 {
            FeatureVariationList::new(
                reader,
                offset + feature_variations_offset as u64,
                offset + length as u64,
            )
            .ok()
            .map(Box::new)
        } else {
            None
        };

        Ok(Self {
            major_version,
            minor_version,
            scripts,
            features,
            lookups,
            feature_variations,
        })
    }

    fn parse_lookups<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<Vec<PositioningLookup>, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let lookup_count = reader.read_u16_be()?;
        let mut lookup_offsets = Vec::with_capacity(lookup_count as usize);
        for _ in 0..lookup_count {
            lookup_offsets.push(reader.read_u16_be()?);
        }

        let mut lookups = Vec::with_capacity(lookup_offsets.len());
        for lookup_offset in lookup_offsets {
            let lookup_offset = offset + lookup_offset as u64;
            reader.seek(SeekFrom::Start(lookup_offset))?;
            let lookup_type = reader.read_u16_be()?;
            let lookup_flag = reader.read_u16_be()?;
            let subtable_count = reader.read_u16_be()?;
            let mut subtable_offsets = Vec::with_capacity(subtable_count as usize);
            for _ in 0..subtable_count {
                subtable_offsets.push(reader.read_u16_be()?);
            }

            let mark_filtering_set = if lookup_flag & 0x0010 != 0 {
                Some(reader.read_u16_be()?)
            } else {
                None
            };
            let mut subtables = Vec::with_capacity(subtable_offsets.len());
            for subtable_offset in subtable_offsets {
                subtables.push(Self::parse_subtable(
                    reader,
                    lookup_type,
                    lookup_offset + subtable_offset as u64,
                )?);
            }

            lookups.push(PositioningLookup {
                lookup_type,
                lookup_flag,
                mark_filtering_set,
                subtables,
            });
        }

        Ok(lookups)
    }

    fn parse_subtable<R: BinaryReader>(
        reader: &mut R,
        lookup_type: u16,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        match lookup_type {
            1 => Self::parse_single_adjustment(reader, offset),
            2 => Self::parse_pair_adjustment(reader, offset),
            3 => Self::parse_cursive_attachment(reader, offset),
            4 => Self::parse_mark_to_base(reader, offset),
            5 => Self::parse_mark_to_ligature(reader, offset),
            6 => Self::parse_mark_to_mark(reader, offset),
            9 => Self::parse_extension(reader, offset),
            _ => Ok(PositioningSubtable::Unsupported),
        }
    }

    fn parse_single_adjustment<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let format = reader.read_u16_be()?;
        if format != 1 && format != 2 {
            return Ok(PositioningSubtable::Unsupported);
        }
        let coverage_offset = reader.read_u16_be()?;
        let value_format = reader.read_u16_be()?;
        if coverage_offset == 0 || value_format & 0xff00 != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid SinglePos coverage offset or reserved ValueFormat bits",
            ));
        }
        let count = if format == 1 {
            1
        } else {
            reader.read_u16_be()?
        };
        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            values.push(ValueRecord::parse_with_base(reader, value_format, offset)?);
        }
        let coverage = Coverage::new(reader, offset + coverage_offset as u64)?;
        // Validate index arithmetic here before Coverage::contains performs it.
        let glyph_count = match &coverage {
            Coverage::Format1(table) => {
                if table.glyph_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "SinglePos coverage glyphs must be sorted and unique",
                    ));
                }
                table.glyph_ids.len()
            }
            Coverage::Format2(table) => {
                let mut total = 0usize;
                let mut previous_end = None;
                for range in &table.range_records {
                    if range.start_glyph_id > range.end_glyph_id
                        || previous_end.is_some_and(|end| end >= range.start_glyph_id)
                        || range.start_coverage_index as usize != total
                    {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "invalid SinglePos coverage range or index",
                        ));
                    }
                    total += range.end_glyph_id as usize - range.start_glyph_id as usize + 1;
                    previous_end = Some(range.end_glyph_id);
                }
                total
            }
        };
        if format == 2 && glyph_count != values.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "SinglePos value count does not match coverage",
            ));
        }
        Ok(PositioningSubtable::Single(SinglePos {
            coverage,
            values,
            shared: format == 1,
        }))
    }

    fn parse_pair_adjustment<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let pos_format = reader.read_u16_be()?;
        let coverage_offset = reader.read_u16_be()?;
        let value_format1 = reader.read_u16_be()?;
        let value_format2 = reader.read_u16_be()?;

        match pos_format {
            1 => {
                let pair_set_count = reader.read_u16_be()?;
                let mut pair_set_offsets = Vec::with_capacity(pair_set_count as usize);
                for _ in 0..pair_set_count {
                    pair_set_offsets.push(reader.read_u16_be()?);
                }

                let mut pair_sets = Vec::with_capacity(pair_set_offsets.len());
                for pair_set_offset in pair_set_offsets {
                    reader.seek(SeekFrom::Start(offset + pair_set_offset as u64))?;
                    let pair_value_count = reader.read_u16_be()?;
                    let mut pair_value_records = Vec::with_capacity(pair_value_count as usize);
                    for _ in 0..pair_value_count {
                        let second_glyph = reader.read_u16_be()?;
                        let value1 = ValueRecord::parse_with_base(reader, value_format1, offset)?;
                        let value2 = ValueRecord::parse_with_base(reader, value_format2, offset)?;
                        pair_value_records.push(PairValueRecord {
                            second_glyph,
                            value1,
                            value2,
                        });
                    }
                    pair_sets.push(PairSet { pair_value_records });
                }

                Ok(PositioningSubtable::PairFormat1(PairPosFormat1 {
                    second_is_present: value_format2 != 0,
                    coverage: Coverage::new(reader, offset + coverage_offset as u64)?,
                    pair_sets,
                }))
            }
            2 => {
                let class_def1_offset = reader.read_u16_be()?;
                let class_def2_offset = reader.read_u16_be()?;
                let class1_count = reader.read_u16_be()?;
                let class2_count = reader.read_u16_be()?;

                let mut class1_records = Vec::with_capacity(class1_count as usize);
                for _ in 0..class1_count {
                    let mut class2_records = Vec::with_capacity(class2_count as usize);
                    for _ in 0..class2_count {
                        class2_records.push(Class2Record {
                            value1: ValueRecord::parse_with_base(reader, value_format1, offset)?,
                            value2: ValueRecord::parse_with_base(reader, value_format2, offset)?,
                        });
                    }
                    class1_records.push(class2_records);
                }

                Ok(PositioningSubtable::PairFormat2(PairPosFormat2 {
                    second_is_present: value_format2 != 0,
                    coverage: Coverage::new(reader, offset + coverage_offset as u64)?,
                    class_def1: ClassDef::new(reader, offset + class_def1_offset as u64)?,
                    class_def2: ClassDef::new(reader, offset + class_def2_offset as u64)?,
                    class1_records,
                }))
            }
            _ => Ok(PositioningSubtable::Unsupported),
        }
    }

    fn parse_cursive_attachment<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let pos_format = reader.read_u16_be()?;
        if pos_format != 1 {
            return Ok(PositioningSubtable::Unsupported);
        }
        let coverage_offset = reader.read_u16_be()?;
        let entry_exit_count = reader.read_u16_be()?;
        let mut raw_records = Vec::with_capacity(entry_exit_count as usize);
        for _ in 0..entry_exit_count {
            let entry_offset = reader.read_u16_be()?;
            let exit_offset = reader.read_u16_be()?;
            raw_records.push((entry_offset, exit_offset));
        }
        let coverage = Coverage::new(reader, offset + coverage_offset as u64)?;
        let mut records = Vec::with_capacity(raw_records.len());
        for (entry_offset, exit_offset) in raw_records {
            let entry = if entry_offset != 0 {
                Self::parse_anchor(reader, offset + entry_offset as u64)?
            } else {
                None
            };
            let exit = if exit_offset != 0 {
                Self::parse_anchor(reader, offset + exit_offset as u64)?
            } else {
                None
            };
            records.push((entry, exit));
        }
        Ok(PositioningSubtable::CursiveFormat1(CursivePosFormat1 {
            coverage,
            records,
        }))
    }

    fn parse_mark_to_base<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let pos_format = reader.read_u16_be()?;
        if pos_format != 1 {
            return Ok(PositioningSubtable::Unsupported);
        }

        let mark_coverage_offset = reader.read_u16_be()?;
        let base_coverage_offset = reader.read_u16_be()?;
        let class_count = reader.read_u16_be()?;
        let mark_array_offset = reader.read_u16_be()?;
        let base_array_offset = reader.read_u16_be()?;

        Ok(PositioningSubtable::MarkToBaseFormat1(MarkToBaseFormat1 {
            mark_coverage: Coverage::new(reader, offset + mark_coverage_offset as u64)?,
            base_coverage: Coverage::new(reader, offset + base_coverage_offset as u64)?,
            mark_array: Self::parse_mark_array(reader, offset + mark_array_offset as u64)?,
            base_array: Self::parse_base_array(
                reader,
                offset + base_array_offset as u64,
                class_count,
            )?,
        }))
    }

    fn parse_mark_to_ligature<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let pos_format = reader.read_u16_be()?;
        if pos_format != 1 {
            return Ok(PositioningSubtable::Unsupported);
        }

        let mark_coverage_offset = reader.read_u16_be()?;
        let ligature_coverage_offset = reader.read_u16_be()?;
        let class_count = reader.read_u16_be()?;
        let mark_array_offset = reader.read_u16_be()?;
        let ligature_array_offset = reader.read_u16_be()?;

        Ok(PositioningSubtable::MarkToLigatureFormat1(
            MarkToLigatureFormat1 {
                mark_coverage: Coverage::new(reader, offset + mark_coverage_offset as u64)?,
                ligature_coverage: Coverage::new(reader, offset + ligature_coverage_offset as u64)?,
                mark_array: Self::parse_mark_array(reader, offset + mark_array_offset as u64)?,
                ligature_array: Self::parse_ligature_array(
                    reader,
                    offset + ligature_array_offset as u64,
                    class_count,
                )?,
            },
        ))
    }

    fn parse_mark_to_mark<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let pos_format = reader.read_u16_be()?;
        if pos_format != 1 {
            return Ok(PositioningSubtable::Unsupported);
        }

        let mark1_coverage_offset = reader.read_u16_be()?;
        let mark2_coverage_offset = reader.read_u16_be()?;
        let class_count = reader.read_u16_be()?;
        let mark1_array_offset = reader.read_u16_be()?;
        let mark2_array_offset = reader.read_u16_be()?;

        Ok(PositioningSubtable::MarkToMarkFormat1(MarkToMarkFormat1 {
            mark1_coverage: Coverage::new(reader, offset + mark1_coverage_offset as u64)?,
            mark2_coverage: Coverage::new(reader, offset + mark2_coverage_offset as u64)?,
            mark1_array: Self::parse_mark_array(reader, offset + mark1_array_offset as u64)?,
            mark2_array: Self::parse_mark2_array(
                reader,
                offset + mark2_array_offset as u64,
                class_count,
            )?,
        }))
    }

    fn parse_mark_array<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<MarkArray, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let mark_count = reader.read_u16_be()?;
        let mut records = Vec::with_capacity(mark_count as usize);

        for _ in 0..mark_count {
            let mark_class = reader.read_u16_be()?;
            let mark_anchor_offset = reader.read_u16_be()?;
            records.push((mark_class, mark_anchor_offset));
        }

        let mut mark_records = Vec::with_capacity(records.len());
        for (mark_class, mark_anchor_offset) in records {
            let mark_anchor = if mark_anchor_offset == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "mark anchor offset must not be zero",
                ));
            } else {
                Self::parse_anchor(reader, offset + mark_anchor_offset as u64)?
            }
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "mark anchor offset must not be zero",
                )
            })?;
            mark_records.push(MarkRecord {
                mark_class,
                mark_anchor,
            });
        }

        Ok(MarkArray { mark_records })
    }

    fn parse_base_array<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        class_count: u16,
    ) -> Result<BaseArray, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let base_count = reader.read_u16_be()?;
        let mut anchor_offsets = Vec::with_capacity(base_count as usize);

        for _ in 0..base_count {
            let mut offsets = Vec::with_capacity(class_count as usize);
            for _ in 0..class_count {
                offsets.push(reader.read_u16_be()?);
            }
            anchor_offsets.push(offsets);
        }

        let mut base_records = Vec::with_capacity(anchor_offsets.len());
        for offsets in anchor_offsets {
            let mut base_anchors = Vec::with_capacity(offsets.len());
            for anchor_offset in offsets {
                let anchor = if anchor_offset == 0 {
                    None
                } else {
                    Self::parse_anchor(reader, offset + anchor_offset as u64)?
                };
                base_anchors.push(anchor);
            }
            base_records.push(BaseRecord { base_anchors });
        }

        Ok(BaseArray { base_records })
    }

    fn parse_ligature_array<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        class_count: u16,
    ) -> Result<LigatureArray, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let ligature_count = reader.read_u16_be()?;
        let mut ligature_offsets = Vec::with_capacity(ligature_count as usize);
        for _ in 0..ligature_count {
            ligature_offsets.push(reader.read_u16_be()?);
        }

        let mut ligature_attaches = Vec::with_capacity(ligature_offsets.len());
        for ligature_offset in ligature_offsets {
            reader.seek(SeekFrom::Start(offset + ligature_offset as u64))?;
            let component_count = reader.read_u16_be()?;
            let mut component_offsets = Vec::with_capacity(component_count as usize);
            for _ in 0..component_count {
                let mut anchor_offsets = Vec::with_capacity(class_count as usize);
                for _ in 0..class_count {
                    anchor_offsets.push(reader.read_u16_be()?);
                }
                component_offsets.push(anchor_offsets);
            }

            let mut component_records = Vec::with_capacity(component_offsets.len());
            for anchor_offsets in component_offsets {
                let mut ligature_anchors = Vec::with_capacity(anchor_offsets.len());
                for anchor_offset in anchor_offsets {
                    let anchor = if anchor_offset == 0 {
                        None
                    } else {
                        Self::parse_anchor(
                            reader,
                            offset + ligature_offset as u64 + anchor_offset as u64,
                        )?
                    };
                    ligature_anchors.push(anchor);
                }
                component_records.push(ComponentRecord { ligature_anchors });
            }

            ligature_attaches.push(LigatureAttach { component_records });
        }

        Ok(LigatureArray { ligature_attaches })
    }

    fn parse_mark2_array<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        class_count: u16,
    ) -> Result<Mark2Array, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let mark2_count = reader.read_u16_be()?;
        let mut anchor_offsets = Vec::with_capacity(mark2_count as usize);

        for _ in 0..mark2_count {
            let mut offsets = Vec::with_capacity(class_count as usize);
            for _ in 0..class_count {
                offsets.push(reader.read_u16_be()?);
            }
            anchor_offsets.push(offsets);
        }

        let mut mark2_records = Vec::with_capacity(anchor_offsets.len());
        for offsets in anchor_offsets {
            let mut mark2_anchors = Vec::with_capacity(offsets.len());
            for anchor_offset in offsets {
                let anchor = if anchor_offset == 0 {
                    None
                } else {
                    Self::parse_anchor(reader, offset + anchor_offset as u64)?
                };
                mark2_anchors.push(anchor);
            }
            mark2_records.push(Mark2Record { mark2_anchors });
        }

        Ok(Mark2Array { mark2_records })
    }

    fn parse_anchor<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<Option<Anchor>, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let anchor_format = reader.read_u16_be()?;
        let x = reader.read_i16_be()?;
        let y = reader.read_i16_be()?;

        match anchor_format {
            1 => Ok(Some(Anchor { x, y })),
            2 => {
                let _anchor_point = reader.read_u16_be()?;
                Ok(Some(Anchor { x, y }))
            }
            3 => {
                let _x_device_offset = reader.read_u16_be()?;
                let _y_device_offset = reader.read_u16_be()?;
                Ok(Some(Anchor { x, y }))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unknown anchor format",
            )),
        }
    }

    fn parse_extension<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let pos_format = reader.read_u16_be()?;
        let extension_lookup_type = reader.read_u16_be()?;
        let extension_offset = reader.read_u32_be()?;
        // OpenType GPOS extension format 1 cannot point to another extension.
        // Valid but unsupported types (3, 7, 8) retain their existing behavior.
        if pos_format != 1 || !(1..=8).contains(&extension_lookup_type) || extension_offset == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid GPOS extension header",
            ));
        }
        let offset = offset
            .checked_add(u64::from(extension_offset))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "GPOS extension offset overflow",
                )
            })?;
        Ok(PositioningSubtable::Extension(Box::new(
            Self::parse_subtable(reader, extension_lookup_type, offset)?,
        )))
    }

    fn locale_subtags(locale: &str) -> Vec<String> {
        let locale = locale.trim();
        if locale.is_empty() {
            return Vec::new();
        }

        locale
            .split(['-', '_'])
            .map(str::trim)
            .filter(|subtag| !subtag.is_empty())
            .map(|subtag| subtag.to_ascii_lowercase())
            .collect()
    }

    fn push_language_system_tag(tags: &mut Vec<u32>, tag: [u8; 4]) {
        let tag = u32::from_be_bytes(tag);
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }

    fn locale_to_language_system_tags(locale: &str) -> Vec<u32> {
        let subtags = Self::locale_subtags(locale);
        if subtags.is_empty() {
            return Vec::new();
        }

        let mut tags = Vec::new();
        match subtags[0].as_str() {
            "default" => Self::push_language_system_tag(&mut tags, [0, 0, 0, 0]),
            "ja" | "jp" | "jpn" => Self::push_language_system_tag(&mut tags, *b"JAN "),
            "ar" | "ara" => Self::push_language_system_tag(&mut tags, *b"ARA "),
            "fa" | "fas" | "per" => Self::push_language_system_tag(&mut tags, *b"FAR "),
            "ur" | "urd" => Self::push_language_system_tag(&mut tags, *b"URD "),
            "sd" | "snd" => Self::push_language_system_tag(&mut tags, *b"SND "),
            "he" | "heb" => {
                Self::push_language_system_tag(&mut tags, *b"IWR ");
                Self::push_language_system_tag(&mut tags, *b"HEB ");
            }
            "syr" => Self::push_language_system_tag(&mut tags, *b"SYR "),
            "syrj" => Self::push_language_system_tag(&mut tags, *b"SYRJ"),
            "syrn" => Self::push_language_system_tag(&mut tags, *b"SYRN"),
            _ => {}
        }

        for subtag in &subtags {
            match subtag.as_str() {
                "jp" => Self::push_language_system_tag(&mut tags, *b"JAN "),
                "arab" | "ar" | "ara" => Self::push_language_system_tag(&mut tags, *b"ARA "),
                "urd" | "ur" => Self::push_language_system_tag(&mut tags, *b"URD "),
                "far" | "fa" | "fas" | "per" => Self::push_language_system_tag(&mut tags, *b"FAR "),
                "snd" | "sd" => Self::push_language_system_tag(&mut tags, *b"SND "),
                "heb" | "he" => {
                    Self::push_language_system_tag(&mut tags, *b"IWR ");
                    Self::push_language_system_tag(&mut tags, *b"HEB ");
                }
                "syrc" | "syr" => Self::push_language_system_tag(&mut tags, *b"SYR "),
                "syrj" => Self::push_language_system_tag(&mut tags, *b"SYRJ"),
                "syrn" => Self::push_language_system_tag(&mut tags, *b"SYRN"),
                _ if (subtag.len() == 3 || subtag.len() == 4)
                    && subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) =>
                {
                    let mut tag = [b' '; 4];
                    for (i, ch) in subtag.chars().take(4).enumerate() {
                        tag[i] = ch.to_ascii_uppercase() as u8;
                    }
                    Self::push_language_system_tag(&mut tags, tag);
                }
                _ => {}
            }
        }

        tags
    }

    fn get_language_systems<'a>(
        &'a self,
        script: &'a ParsedScript,
        locale: Option<&str>,
    ) -> Vec<&'a crate::opentype::layouts::LanguageSystemRecord> {
        let mut systems = Vec::new();

        if let Some(locale) = locale {
            for tag in Self::locale_to_language_system_tags(locale) {
                if let Some(language_system) = script
                    .language_systems
                    .iter()
                    .find(|record| record.language_system_tag == tag)
                {
                    if !systems.iter().any(
                        |existing: &&crate::opentype::layouts::LanguageSystemRecord| {
                            existing.language_system_tag == language_system.language_system_tag
                        },
                    ) {
                        systems.push(language_system);
                    }
                }
            }

            if systems.is_empty() {
                if let Some(default_system) = script
                    .language_systems
                    .iter()
                    .find(|record| record.language_system_tag == 0)
                {
                    systems.push(default_system);
                }
            }

            if systems.is_empty() {
                if let Some(first) = script.language_systems.first() {
                    systems.push(first);
                }
            }
        } else {
            if let Some(default_system) = script
                .language_systems
                .iter()
                .find(|record| record.language_system_tag == 0)
            {
                systems.push(default_system);
            }

            for system in script.language_systems.iter() {
                if system.language_system_tag != 0 {
                    systems.push(system);
                }
            }
        }

        systems
    }

    fn collect_lookups<'a>(
        &'a self,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) -> Vec<&'a PositioningLookup> {
        self.collect_lookups_with_variations(locale, feature_tags, &[])
    }

    fn collect_lookups_with_variations<'a>(
        &'a self,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
        variation_coords: &[f32],
    ) -> Vec<&'a PositioningLookup> {
        if locale.is_none() {
            return self.collect_lookups_from_scripts_with_variations(
                self.scripts.scripts.iter().collect(),
                locale,
                feature_tags,
                variation_coords,
            );
        }

        let (preferred_scripts, default_scripts, other_scripts) = self.partition_scripts(locale);
        let mut result = Vec::new();
        let mut seen_lookup_ptrs = HashSet::new();

        for feature_tag in feature_tags {
            let lookups = {
                let preferred = self.collect_lookups_from_scripts_with_variations(
                    preferred_scripts.clone(),
                    locale,
                    std::slice::from_ref(feature_tag),
                    variation_coords,
                );
                if !preferred.is_empty() {
                    preferred
                } else {
                    let defaults = self.collect_lookups_from_scripts_with_variations(
                        default_scripts.clone(),
                        locale,
                        std::slice::from_ref(feature_tag),
                        variation_coords,
                    );
                    if !defaults.is_empty() {
                        defaults
                    } else {
                        self.collect_lookups_from_scripts_with_variations(
                            other_scripts.clone(),
                            locale,
                            std::slice::from_ref(feature_tag),
                            variation_coords,
                        )
                    }
                }
            };

            for lookup in lookups {
                let ptr = lookup as *const PositioningLookup as usize;
                if seen_lookup_ptrs.insert(ptr) {
                    result.push(lookup);
                }
            }
        }

        result
    }

    fn collect_lookups_from_scripts<'a>(
        &'a self,
        scripts: Vec<&'a ParsedScript>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) -> Vec<&'a PositioningLookup> {
        self.collect_lookups_from_scripts_with_variations(scripts, locale, feature_tags, &[])
    }

    fn collect_lookups_from_scripts_with_variations<'a>(
        &'a self,
        scripts: Vec<&'a ParsedScript>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
        variation_coords: &[f32],
    ) -> Vec<&'a PositioningLookup> {
        let mut result = Vec::new();
        let mut seen_lookup_indices = HashSet::new();

        for script in scripts {
            for language_system in self.get_language_systems(script, locale) {
                for feature_index in
                    Self::collect_language_system_feature_indices(&language_system.language_system)
                {
                    let Some(base_feature) = self.features.features.get(feature_index as usize)
                    else {
                        continue;
                    };
                    if !feature_tags
                        .iter()
                        .any(|tag| base_feature.feature_tag == u32::from_be_bytes(*tag))
                    {
                        continue;
                    }
                    let feature = self
                        .feature_variations
                        .as_deref()
                        .and_then(|fv| fv.substitute_feature(variation_coords, feature_index))
                        .unwrap_or(base_feature);

                    for lookup_index in feature.lookup_list_indices.iter() {
                        if !seen_lookup_indices.insert(*lookup_index) {
                            continue;
                        }
                        if let Some(lookup) = self.lookups.get(*lookup_index as usize) {
                            result.push(lookup);
                        }
                    }
                }
            }
        }

        result
    }

    fn partition_scripts<'a>(
        &'a self,
        locale: Option<&str>,
    ) -> (
        Vec<&'a ParsedScript>,
        Vec<&'a ParsedScript>,
        Vec<&'a ParsedScript>,
    ) {
        if locale.is_none() {
            return (
                self.scripts.scripts.iter().collect(),
                Vec::new(),
                Vec::new(),
            );
        }

        let mut preferred = Vec::new();
        let mut defaults = Vec::new();
        let mut others = Vec::new();
        let preferred_tags = locale.map(Self::locale_to_script_tags).unwrap_or_default();

        if let Some(locale) = locale {
            for script_tag in Self::locale_to_script_tags(locale) {
                if let Some(script) = self
                    .scripts
                    .scripts
                    .iter()
                    .find(|script| script.script_tag == script_tag)
                {
                    preferred.push(script);
                }
            }
        }

        for script in self.scripts.scripts.iter() {
            if script.script_tag == u32::from_be_bytes(*b"DFLT") {
                defaults.push(script);
            } else if preferred_tags.contains(&script.script_tag) {
                if !preferred
                    .iter()
                    .any(|existing| existing.script_tag == script.script_tag)
                {
                    preferred.push(script);
                }
            } else {
                others.push(script);
            }
        }

        (preferred, defaults, others)
    }

    fn locale_to_script_tags(locale: &str) -> Vec<u32> {
        let subtags = Self::locale_subtags(locale);
        if subtags.is_empty() {
            return Vec::new();
        }

        let mut tags = Vec::new();
        let mut push_tag = |tag: [u8; 4]| {
            let tag = u32::from_be_bytes(tag);
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        };

        for subtag in &subtags {
            match subtag.as_str() {
                "arab" | "ar" | "ara" | "urd" | "fas" | "per" | "snd" => push_tag(*b"arab"),
                "hebr" | "he" | "heb" => push_tag(*b"hebr"),
                "syrc" | "syr" | "syc" | "syrj" | "syrn" => push_tag(*b"syrc"),
                "kana" | "ja" | "jp" | "jpn" => push_tag(*b"kana"),
                "hani" | "zh" | "zho" | "chi" => push_tag(*b"hani"),
                "hang" | "ko" | "kor" => push_tag(*b"hang"),
                _ if subtag.len() == 4 && subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) => {
                    let mut tag = [b' '; 4];
                    for (index, byte) in subtag.bytes().take(4).enumerate() {
                        tag[index] = byte;
                    }
                    push_tag(tag);
                }
                _ => {}
            }
        }

        tags
    }

    fn collect_language_system_feature_indices(
        language_system: &crate::opentype::layouts::LanguageSystem,
    ) -> Vec<u16> {
        let mut feature_indices = Vec::new();

        if language_system.required_feature_index != 0xFFFF {
            feature_indices.push(language_system.required_feature_index);
        }

        for feature_index in &language_system.feature_indexes {
            if !feature_indices.contains(feature_index) {
                feature_indices.push(*feature_index);
            }
        }

        feature_indices
    }

    /// SinglePos on default spacing and vertical positioning features, with
    /// optional GDEF ItemVariationStore delta evaluation.
    pub(crate) fn lookup_single_adjustment(
        &self,
        glyph: u16,
        is_vertical: bool,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Option<SingleAdjustment> {
        self.lookup_single_adjustment_with_coords(glyph, is_vertical, locale, gdef, &[])
    }

    pub(crate) fn lookup_single_adjustment_with_coords(
        &self,
        glyph: u16,
        is_vertical: bool,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
        variation_coords: &[f32],
    ) -> Option<SingleAdjustment> {
        self.lookup_single_adjustment_with_options(
            glyph,
            is_vertical,
            locale,
            gdef,
            variation_coords,
            &[],
        )
    }

    pub(crate) fn lookup_single_adjustment_with_options(
        &self,
        glyph: u16,
        is_vertical: bool,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
        variation_coords: &[f32],
        custom_features: &[crate::commands::FontFeatureSetting],
    ) -> Option<SingleAdjustment> {
        let base_tags: &[[u8; 4]] = if is_vertical {
            &[*b"vkrn", *b"dist", *b"vert"]
        } else {
            &[*b"kern", *b"dist"]
        };
        let feature_tags = crate::commands::resolve_feature_tags(base_tags, custom_features, true);
        let mut result = SingleAdjustment::default();
        let mut matched = false;
        for lookup in self.collect_lookups_with_variations(locale, &feature_tags, variation_coords)
        {
            if (lookup.lookup_type != 1 && lookup.lookup_type != 9)
                || !lookup.includes_glyph(glyph, gdef)
            {
                continue;
            }
            for subtable in &lookup.subtables {
                if let Some(value) = subtable.lookup_single_adjustment(glyph) {
                    result.add_assign(value.resolve(gdef, variation_coords));
                    matched = true;
                    // Even a zero value is a match, not a fallback opportunity.
                    break;
                }
            }
        }
        matched.then_some(result)
    }

    /// Apply each selected PairPos lookup to the logical glyph sequence.
    /// None entries are boundaries (tabs/newlines), never skippable glyphs.
    /// Accumulate in i32 just like SinglePos, without intermediate i16 clipping.
    pub(crate) fn pair_adjustments_with_coverage(
        &self,
        glyphs: &[Option<u16>],
        is_vertical: bool,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> (Vec<SingleAdjustment>, Vec<bool>) {
        self.pair_adjustments_with_coverage_and_coords(glyphs, is_vertical, locale, gdef, &[])
    }

    pub(crate) fn pair_adjustments_with_coverage_and_coords(
        &self,
        glyphs: &[Option<u16>],
        is_vertical: bool,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
        variation_coords: &[f32],
    ) -> (Vec<SingleAdjustment>, Vec<bool>) {
        self.pair_adjustments_with_options(glyphs, is_vertical, locale, gdef, variation_coords, &[])
    }

    pub(crate) fn pair_adjustments_with_options(
        &self,
        glyphs: &[Option<u16>],
        is_vertical: bool,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
        variation_coords: &[f32],
        custom_features: &[crate::commands::FontFeatureSetting],
    ) -> (Vec<SingleAdjustment>, Vec<bool>) {
        let base_tags = if is_vertical { [*b"vkrn"] } else { [*b"kern"] };
        let tags = crate::commands::resolve_feature_tags(&base_tags, custom_features, true);
        let mut result = vec![SingleAdjustment::default(); glyphs.len()];
        let mut covered = vec![false; glyphs.len()];
        for lookup in self.collect_lookups_with_variations(locale, &tags, variation_coords) {
            if lookup.lookup_type != 2 && lookup.lookup_type != 9 {
                continue;
            }
            let mut first = 0;
            while first < glyphs.len() {
                let Some(left) = glyphs[first].filter(|id| lookup.includes_glyph(*id, gdef)) else {
                    first += 1;
                    continue;
                };
                let mut second = first + 1;
                while second < glyphs.len()
                    && glyphs[second].is_some_and(|id| !lookup.includes_glyph(id, gdef))
                {
                    second += 1;
                }
                let Some(right) = glyphs.get(second).copied().flatten() else {
                    first = second + 1;
                    continue;
                };
                let matched = lookup.subtables.iter().find_map(|subtable| {
                    subtable
                        .lookup_pair_adjustment(left, right)
                        .map(|pair| (pair, subtable.pair_consumes_second()))
                });
                if let Some((pair, consumes_second)) = matched {
                    covered[first] = true;
                    covered[second] = true;
                    result[first].add_assign(pair.first.resolve(gdef, variation_coords));
                    result[second].add_assign(pair.second.resolve(gdef, variation_coords));
                    // A zero record is still a match. ValueFormat2 == 0 lets
                    // the second glyph start the next pair in this lookup.
                    first = second + usize::from(consumes_second);
                } else {
                    first = second;
                }
            }
        }
        (result, covered)
    }

    #[cfg(test)]
    pub(crate) fn pair_adjustments(
        &self,
        glyphs: &[Option<u16>],
        is_vertical: bool,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Vec<SingleAdjustment> {
        self.pair_adjustments_with_coverage(glyphs, is_vertical, locale, gdef)
            .0
    }

    // Legacy two-glyph parser assertions; production uses the sequence executor.
    #[cfg(test)]
    pub(crate) fn lookup_pair_adjustment(
        &self,
        left: u16,
        right: u16,
        is_vertical: bool,
        locale: Option<&str>,
    ) -> Option<PairAdjustment> {
        let feature_tags: &[[u8; 4]] = if is_vertical {
            &[*b"vkrn"]
        } else {
            &[*b"kern"]
        };

        let mut adjustment = PairAdjustment::default();
        let mut matched = false;

        for lookup in self.collect_lookups(locale, feature_tags) {
            if lookup.lookup_type != 2 && lookup.lookup_type != 9 {
                continue;
            }
            for subtable in &lookup.subtables {
                if let Some(found) = subtable.lookup_pair_adjustment(left, right) {
                    adjustment.add_assign(found);
                    matched = true;
                    // A lookup is an ordered list of alternative subtables.
                    // Once one subtable matches, the remaining subtables in
                    // that lookup must not apply a second adjustment.
                    break;
                }
            }
        }

        if matched && !adjustment.is_zero() {
            Some(adjustment)
        } else {
            None
        }
    }

    /// Apply GPOS Type 3 Cursive Attachment (`curs`) lookups across consecutive
    /// GDEF-included glyphs, aligning exit/entry vertical anchors along connected
    /// cursive chains (honoring LookupFlag `RightToLeft` bit 0x0001).
    pub(crate) fn cursive_adjustments(
        &self,
        glyphs: &[Option<u16>],
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Vec<SingleAdjustment> {
        self.cursive_adjustments_with_options(glyphs, locale, gdef, &[], &[])
    }

    pub(crate) fn cursive_adjustments_with_options(
        &self,
        glyphs: &[Option<u16>],
        locale: Option<&str>,
        gdef: Option<&Gdef>,
        variation_coords: &[f32],
        custom_features: &[crate::commands::FontFeatureSetting],
    ) -> Vec<SingleAdjustment> {
        let tags = crate::commands::resolve_feature_tags(&[*b"curs"], custom_features, false);
        let mut result = vec![SingleAdjustment::default(); glyphs.len()];
        for lookup in self.collect_lookups_with_variations(locale, &tags, variation_coords) {
            if lookup.lookup_type != 3 && lookup.lookup_type != 9 {
                continue;
            }
            let rtl_flag = lookup.lookup_flag & 0x0001 != 0;
            // Collect pairs of consecutive included glyphs within contiguous runs.
            let mut links: Vec<(usize, usize, i32)> = Vec::new();
            let mut first = 0;
            while first < glyphs.len() {
                let Some(left) = glyphs[first].filter(|id| lookup.includes_glyph(*id, gdef)) else {
                    first += 1;
                    continue;
                };
                let mut second = first + 1;
                while second < glyphs.len()
                    && glyphs[second].is_some_and(|id| !lookup.includes_glyph(id, gdef))
                {
                    second += 1;
                }
                let Some(right) = glyphs.get(second).copied().flatten() else {
                    first = second + 1;
                    continue;
                };
                let anchors = lookup.subtables.iter().find_map(|subtable| {
                    let (_, exit) = subtable.lookup_cursive_anchors(left)?;
                    let (entry, _) = subtable.lookup_cursive_anchors(right)?;
                    Some((exit?, entry?))
                });
                if let Some((exit, entry)) = anchors {
                    links.push((first, second, i32::from(exit.y) - i32::from(entry.y)));
                }
                first = second;
            }
            if links.is_empty() {
                continue;
            }
            let mut y_offsets = vec![0i32; glyphs.len()];
            if rtl_flag {
                // RTL cursive flag: last glyph anchors to baseline, chain backwards.
                for &(prev, next, dy) in links.iter().rev() {
                    y_offsets[prev] = y_offsets[next].saturating_sub(dy);
                }
            } else {
                // LTR cursive flag: first glyph anchors to baseline, chain forwards.
                for &(prev, next, dy) in &links {
                    y_offsets[next] = y_offsets[prev].saturating_add(dy);
                }
            }
            for (idx, dy) in y_offsets.into_iter().enumerate() {
                if dy != 0 {
                    result[idx].y_placement = result[idx].y_placement.saturating_add(dy);
                }
            }
        }
        result
    }

    pub(crate) fn lookup_mark_to_base_adjustment(
        &self,
        base: u16,
        mark: u16,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Option<MarkAttachmentAdjustment> {
        let feature_tags: &[[u8; 4]] = &[*b"mark"];

        for lookup in self.collect_lookups(locale, feature_tags) {
            // Only the current input is filtered here. In particular, IgnoreBase/
            // IgnoreLigature must not reject the already selected attachment target.
            if (lookup.lookup_type != 4 && lookup.lookup_type != 9)
                || !lookup.includes_glyph(mark, gdef)
            {
                continue;
            }
            for subtable in &lookup.subtables {
                if let Some(found) = subtable.lookup_mark_to_base_adjustment(base, mark) {
                    return Some(found);
                }
            }
        }

        None
    }

    // This is a bounded fallback, not a global complex-script feature plan.
    pub(crate) fn lookup_tibetan_vowel_adjustment(
        &self,
        base: u16,
        mark: u16,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Option<MarkAttachmentAdjustment> {
        self.lookup_tibetan_base_adjustment(base, mark, locale, gdef, &[*b"abvm", *b"blwm"])
    }

    pub(crate) fn lookup_tibetan_upper_adjustment(
        &self,
        base: u16,
        mark: u16,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Option<MarkAttachmentAdjustment> {
        self.lookup_tibetan_base_adjustment(base, mark, locale, gdef, &[*b"abvm"])
    }

    fn lookup_tibetan_base_adjustment(
        &self,
        base: u16,
        mark: u16,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
        tags: &[[u8; 4]],
    ) -> Option<MarkAttachmentAdjustment> {
        let scripts = self
            .scripts
            .scripts
            .iter()
            .filter(|script| script.script_tag == u32::from_be_bytes(*b"tibt"))
            .collect();
        for lookup in self.collect_lookups_from_scripts(scripts, locale, tags) {
            if lookup.lookup_type != 4 && lookup.lookup_type != 9 {
                continue;
            }
            // Do not guess participant classes when filtering is requested.
            let has_filter_classes = gdef.is_some_and(|gdef| {
                gdef.glyph_class(base).is_some() && gdef.glyph_class(mark).is_some()
            });
            if (!has_filter_classes && lookup.lookup_flag & 0xff1e != 0)
                || !lookup.includes_glyph(base, gdef)
                || !lookup.includes_glyph(mark, gdef)
            {
                continue;
            }
            for subtable in &lookup.subtables {
                if let Some(found) = subtable.lookup_mark_to_base_adjustment(base, mark) {
                    return Some(found);
                }
            }
        }
        None
    }

    pub(crate) fn lookup_mark_to_ligature_adjustment(
        &self,
        ligature: u16,
        mark: u16,
        component_index: usize,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Option<MarkAttachmentAdjustment> {
        let feature_tags: &[[u8; 4]] = &[*b"mark"];

        for lookup in self.collect_lookups(locale, feature_tags) {
            // Only the current input is filtered here. In particular, IgnoreBase/
            // IgnoreLigature must not reject the already selected attachment target.
            if (lookup.lookup_type != 5 && lookup.lookup_type != 9)
                || !lookup.includes_glyph(mark, gdef)
            {
                continue;
            }
            for subtable in &lookup.subtables {
                if let Some(found) =
                    subtable.lookup_mark_to_ligature_adjustment(ligature, mark, component_index)
                {
                    return Some(found);
                }
            }
        }

        None
    }

    #[cfg(test)]
    pub(crate) fn lookup_mark_to_mark_adjustment(
        &self,
        base_mark: u16,
        combining_mark: u16,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Option<MarkAttachmentAdjustment> {
        self.lookup_mark_to_mark_attachment(
            1,
            |index| [base_mark, combining_mark].get(index).copied(),
            || false,
            locale,
            gdef,
        )
        .map(|(_, adjustment)| adjustment)
    }

    pub(crate) fn lookup_mark_to_mark_attachment(
        &self,
        index: usize,
        glyph_at: impl Fn(usize) -> Option<u16>,
        allow_skipping: impl Fn() -> bool,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> Option<(usize, MarkAttachmentAdjustment)> {
        let combining_mark = glyph_at(index)?;
        // Most adjacent attachments need no ownership walk. Cache the full
        // guard only when a lookup actually needs to skip an excluded mark.
        let mut skip_allowed = None;
        for lookup in self.collect_lookups(locale, &[*b"mkmk"]) {
            if (lookup.lookup_type != 6 && lookup.lookup_type != 9)
                || !lookup.includes_glyph(combining_mark, gdef)
            {
                continue;
            }
            let mut cursor = index.checked_sub(1)?;
            let target = loop {
                let Some(glyph) = glyph_at(cursor) else {
                    break None;
                };
                match gdef.and_then(|table| table.glyph_class(glyph)) {
                    Some(GlyphClass::Base | GlyphClass::Ligature | GlyphClass::Component) => {
                        break None
                    }
                    Some(GlyphClass::Mark)
                        if !lookup.includes_mark_attachment_class(glyph, gdef.unwrap())
                            && *skip_allowed.get_or_insert_with(&allow_skipping) =>
                    {
                        let Some(previous) = cursor.checked_sub(1) else {
                            break None;
                        };
                        cursor = previous;
                    }
                    // Unknown classes and guarded-out runs retain adjacent coverage
                    // behavior; this is not a claim of complete HB target filtering.
                    _ => break Some(glyph),
                }
            };
            let Some(target) = target else { continue };
            for subtable in &lookup.subtables {
                if let Some(found) = subtable.lookup_mark_to_mark_adjustment(target, combining_mark)
                {
                    return Some((cursor, found));
                }
            }
            // Coverage/anchor failure cannot search past an admitted target.
            // A later lookup starts a fresh search with its own properties.
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    #[test]
    fn adjacent_mkmk_never_walks_ownership_and_skipping_checks_once() {
        let file = crate::FontFile::from_buffer(include_bytes!(
            "../../../tests/fixtures/mkmk-predecessor.ttc"
        ))
        .unwrap();
        let font = file.face(3).unwrap(); // first uncovered target, then later lookup
        let mut gpos = font.font().more_fonts[2].gpos.clone().unwrap();
        let gdef = font.font().more_fonts[2].gdef.as_ref().unwrap();
        // Both lookups now exclude comma; first lacks acute Mark2 coverage.
        let properties = gpos
            .lookups
            .iter()
            .rev()
            .find(|l| l.lookup_type == 6)
            .map(|l| (l.lookup_flag, l.mark_filtering_set))
            .unwrap();
        for lookup in gpos.lookups.iter_mut().filter(|l| l.lookup_type == 6) {
            lookup.lookup_flag = properties.0;
            lookup.mark_filtering_set = properties.1;
        }
        for index in 1..10_001 {
            let result = gpos.lookup_mark_to_mark_attachment(
                index,
                |_| Some(6),
                || panic!("admitted adjacent mark must not scan its full prefix"),
                None,
                Some(gdef),
            );
            assert_eq!(result.unwrap().0, index - 1);
        }
        let calls = std::cell::Cell::new(0);
        let glyphs = [1, 6, 3, 6];
        let result = gpos.lookup_mark_to_mark_attachment(
            3,
            |at| glyphs.get(at).copied(),
            || {
                calls.set(calls.get() + 1);
                true
            },
            None,
            Some(gdef),
        );
        assert_eq!(result.unwrap().0, 1);
        assert_eq!(calls.get(), 1, "one cached ownership proof across lookups");
        // A denied ownership proof preserves a covered immediate target even
        // when that target is excluded by the lookup's set. Unknown ownership
        // is compatibility behavior, not partial target filtering.
        let mut covered = font.font().more_fonts[9].gpos.clone().unwrap();
        for lookup in covered.lookups.iter_mut().filter(|l| l.lookup_type == 6) {
            lookup.lookup_flag = properties.0;
            lookup.mark_filtering_set = properties.1;
        }
        let result = covered.lookup_mark_to_mark_attachment(
            3,
            |at| glyphs.get(at).copied(),
            || false,
            None,
            Some(gdef),
        );
        assert_eq!(result.unwrap().0, 2);
    }

    #[test]
    fn mark_input_filter_reports_no_anchor_despite_gdef_metadata() {
        let font = crate::load_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/mark-coordinates.ttf"
        ))
        .unwrap();
        let mut gpos = font.font().gpos.clone().unwrap();
        let mut gdef = font.font().gdef.clone().unwrap();
        // Acute (gid6) has attachment class1. GDEF metadata alone does not
        // establish an anchor in these actual GPOS results.
        let bytes = [0u8, 1, 0, 6, 0, 1, 0, 1];
        gdef.mark_attach_class_def = Some(
            crate::opentype::layouts::ClassDef::new(&mut BytesReader::new(&bytes), 0).unwrap(),
        );
        for flag in [0, 2, 4, 8, 0x100, 0x200, 0x10] {
            for lookup in &mut gpos.lookups {
                lookup.lookup_flag = flag;
            }
            let expected = matches!(flag, 0 | 2 | 4 | 0x100);
            assert_eq!(
                gpos.lookup_mark_to_base_adjustment(1, 6, None, Some(&gdef))
                    .is_some(),
                expected
            );
            assert_eq!(
                gpos.lookup_mark_to_ligature_adjustment(2, 6, 1, None, Some(&gdef))
                    .is_some(),
                expected
            );
            assert_eq!(
                gpos.lookup_mark_to_mark_adjustment(6, 6, None, Some(&gdef))
                    .is_some(),
                expected
            );
        }
    }

    fn single_bytes(format: u16, glyphs: &[u16], values: &[[i16; 4]]) -> Vec<u8> {
        let mut words = vec![format, 0, 0x000f];
        if format == 2 {
            words.push(values.len() as u16);
        }
        for value in values {
            words.extend(value.iter().map(|field| *field as u16));
        }
        words[1] = (words.len() * 2) as u16;
        words.extend([1, glyphs.len() as u16]);
        words.extend(glyphs);
        words.iter().flat_map(|word| word.to_be_bytes()).collect()
    }

    fn parse_single(bytes: &[u8]) -> Result<PositioningSubtable, std::io::Error> {
        Gpos::parse_subtable(&mut BytesReader::new(bytes), 1, 0)
    }

    fn single_gpos(subtables: Vec<PositioningSubtable>, flag: u16) -> Gpos {
        let bytes = include_bytes!("../../../tests/fixtures/gpos-single.bin");
        let mut gpos = Gpos::new(&mut BytesReader::new(bytes), 0, bytes.len() as u32).unwrap();
        let kern_index = gpos
            .features
            .features
            .iter()
            .find(|feature| feature.feature_tag == u32::from_be_bytes(*b"kern"))
            .unwrap()
            .lookup_list_indices[0] as usize;
        for lookup in &mut gpos.lookups {
            lookup.subtables.clear();
        }
        gpos.lookups[kern_index] = PositioningLookup {
            lookup_type: 1,
            lookup_flag: flag,
            mark_filtering_set: None,
            subtables,
        };
        gpos
    }

    fn pair_bytes(format: u16, first: u16, second: u16, value: i16) -> Vec<u8> {
        let mut words = if format == 1 {
            // One pair set with an empty second ValueRecord.
            vec![1, 18, 4, 0, 1, 12, 1, second, value as u16]
        } else {
            // Class 0 is a valid pair match, including a zero matrix entry.
            vec![2, 18, 4, 0, 24, 30, 1, 1, value as u16]
        };
        words.extend([1, 1, first]);
        if format == 2 {
            words.extend([1, 0, 0, 1, 0, 0]);
        }
        words.iter().flat_map(|word| word.to_be_bytes()).collect()
    }

    fn pair_gpos(subtables: Vec<PositioningSubtable>, flag: u16) -> Gpos {
        let mut gpos = single_gpos(subtables, flag);
        for lookup in &mut gpos.lookups {
            lookup.lookup_type = 2;
        }
        gpos
    }

    #[test]
    fn pair_zero_first_match_blocks_fallback_in_both_formats_and_extensions() {
        for format in [1, 2] {
            for extension in [false, true] {
                let parse = |value| {
                    let bytes = pair_bytes(format, 10, 20, value);
                    let sub = Gpos::parse_subtable(&mut BytesReader::new(&bytes), 2, 0).unwrap();
                    if extension {
                        PositioningSubtable::Extension(Box::new(sub))
                    } else {
                        sub
                    }
                };
                let gpos = pair_gpos(vec![parse(0), parse(-50)], 0);
                assert_eq!(
                    gpos.pair_adjustments(&[Some(10), Some(20)], false, None, None),
                    vec![SingleAdjustment::default(); 2]
                );
                // A missed first coverage falls through to the next subtable.
                let missed = Gpos::parse_subtable(
                    &mut BytesReader::new(&pair_bytes(format, 11, 20, 0)),
                    2,
                    0,
                )
                .unwrap();
                let gpos = pair_gpos(vec![missed, parse(-50)], 0);
                assert_eq!(
                    gpos.pair_adjustments(&[Some(10), Some(20)], false, None, None)[0].x_advance,
                    -50
                );
            }
        }
    }

    #[test]
    fn pair_distinct_lookups_accumulate_without_i16_clipping_or_duplicate_references() {
        let mut gpos = pair_gpos(vec![], 0);
        let mut indices = Vec::new();
        for value in [20000, 20000, -20000] {
            let bytes = pair_bytes(1, 10, 20, value);
            indices.push(gpos.lookups.len() as u16);
            gpos.lookups.push(PositioningLookup {
                lookup_type: 2,
                lookup_flag: 0,
                mark_filtering_set: None,
                subtables: vec![Gpos::parse_subtable(&mut BytesReader::new(&bytes), 2, 0).unwrap()],
            });
        }
        let kern = gpos
            .features
            .features
            .iter_mut()
            .find(|feature| feature.feature_tag == u32::from_be_bytes(*b"kern"))
            .unwrap();
        kern.lookup_list_indices = indices.clone();
        kern.lookup_list_indices.extend(indices);
        assert_eq!(
            gpos.pair_adjustments(&[Some(10), Some(20)], false, None, None)[0].x_advance,
            20000
        );
        gpos.lookups.last_mut().unwrap().subtables.clear();
        assert_eq!(
            gpos.pair_adjustments(&[Some(10), Some(20)], false, None, None)[0].x_advance,
            40000
        );
    }

    #[test]
    fn pair_filters_first_glyph_and_missing_gdef_does_not_invent_classes() {
        let font = crate::load_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/pair-filtering.ttc"
        ))
        .unwrap();
        let gdef = font.font().gdef.as_ref().unwrap();
        for (flag, first) in [(2, 1), (4, 4), (8, 5)] {
            let bytes = pair_bytes(1, first, 2, -50);
            let gpos = pair_gpos(
                vec![Gpos::parse_subtable(&mut BytesReader::new(&bytes), 2, 0).unwrap()],
                flag,
            );
            let glyphs = [Some(first), Some(2)];
            assert_eq!(
                gpos.pair_adjustments(&glyphs, false, None, Some(gdef))[0].x_advance,
                0
            );
            assert_eq!(
                gpos.pair_adjustments(&glyphs, false, None, None)[0].x_advance,
                -50
            );
        }
    }

    #[test]
    fn pair_cursor_skips_consumed_second_and_restarts_after_boundaries() {
        let font = crate::load_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/pair-filtering.ttc"
        ))
        .unwrap();
        let gdef = font.font().gdef.as_ref().unwrap();
        for format in [1, 2] {
            for present in [false, true] {
                for extension in [false, true] {
                    let mut bytes = if present {
                        let mut words: Vec<u16> = if format == 1 {
                            vec![1, 20, 4, 4, 1, 12, 1, 10, 50, 0]
                        } else {
                            vec![2, 20, 4, 4, 26, 32, 1, 1, 50, 0]
                        };
                        words.extend([1, 1, 10]);
                        if format == 2 {
                            words.extend([1, 0, 0, 1, 0, 0]);
                        }
                        words.iter().flat_map(|word| word.to_be_bytes()).collect()
                    } else {
                        pair_bytes(format, 10, 10, 50)
                    };
                    if extension {
                        let mut header = vec![0, 1, 0, 2, 0, 0, 0, 8];
                        header.append(&mut bytes);
                        bytes = header;
                    }
                    let sub = Gpos::parse_subtable(
                        &mut BytesReader::new(&bytes),
                        if extension { 9 } else { 2 },
                        0,
                    )
                    .unwrap();
                    let gpos = pair_gpos(vec![sub], 8);
                    let glyphs = [
                        Some(10),
                        Some(5),
                        Some(10),
                        Some(5),
                        Some(10),
                        Some(5),
                        Some(10),
                        None,
                        Some(10),
                        Some(10),
                    ];
                    let actual: Vec<_> = gpos
                        .pair_adjustments(&glyphs, false, None, Some(gdef))
                        .iter()
                        .map(|value| value.x_advance)
                        .collect();
                    let expected = if present {
                        vec![50, 0, 0, 0, 50, 0, 0, 0, 50, 0]
                    } else {
                        vec![50, 0, 50, 0, 50, 0, 0, 0, 50, 0]
                    };
                    assert_eq!(
                        actual, expected,
                        "format {format}, present {present}, extension {extension}"
                    );
                }
            }
        }
    }

    #[test]
    fn pair_rejects_truncated_records_in_both_formats() {
        for format in [1, 2] {
            let bytes = pair_bytes(format, 10, 20, -50);
            for length in 0..bytes.len() {
                assert!(
                    Gpos::parse_subtable(&mut BytesReader::new(&bytes[..length]), 2, 0).is_err(),
                    "format {format}, length {length}"
                );
            }
        }
    }

    #[test]
    fn vert_single_selects_extensions_deduplicates_and_preserves_zero_precedence() {
        for extension in [false, true] {
            for zero_first in [false, true] {
                let parse = |values: [i16; 4]| {
                    let mut bytes = single_bytes(1, &[10], &[values]);
                    if extension {
                        let mut header = vec![0, 1, 0, 1, 0, 0, 0, 8];
                        header.append(&mut bytes);
                        bytes = header;
                    }
                    Gpos::parse_subtable(
                        &mut BytesReader::new(&bytes),
                        if extension { 9 } else { 1 },
                        0,
                    )
                    .unwrap()
                };
                let mut subtables = vec![parse([500, 1000, 0, -1000])];
                if zero_first {
                    subtables.insert(0, parse([0; 4]));
                }
                let mut gpos = single_gpos(subtables, 0);
                let feature = gpos
                    .features
                    .features
                    .iter_mut()
                    .find(|f| f.feature_tag == u32::from_be_bytes(*b"kern"))
                    .unwrap();
                feature.feature_tag = u32::from_be_bytes(*b"vert");
                let indices = feature.lookup_list_indices.clone();
                for lookup in &mut gpos.lookups {
                    lookup.lookup_type = if extension { 9 } else { 1 };
                }
                for locale in [None, Some("ja")] {
                    assert!(gpos
                        .lookup_single_adjustment(10, false, locale, None)
                        .is_none());
                    assert_eq!(
                        gpos.lookup_single_adjustment(10, true, locale, None)
                            .unwrap(),
                        if zero_first {
                            SingleAdjustment::default()
                        } else {
                            SingleAdjustment {
                                x_placement: 500,
                                y_placement: 1000,
                                x_advance: 0,
                                y_advance: -1000,
                            }
                        }
                    );
                }
                for feature in &mut gpos.features.features {
                    if [*b"vkrn", *b"dist"]
                        .iter()
                        .any(|tag| feature.feature_tag == u32::from_be_bytes(*tag))
                    {
                        feature.lookup_list_indices = indices.clone();
                    }
                }
                for locale in [None, Some("ja")] {
                    assert_eq!(
                        gpos.lookup_single_adjustment(10, true, locale, None)
                            .unwrap()
                            .y_advance,
                        if zero_first { 0 } else { -1000 }
                    );
                }
            }
        }
    }

    #[test]
    fn vert_single_retains_gdef_inclusion_and_exclusion() {
        let font =
            crate::load_font_from_buffer(include_bytes!("../../../tests/fixtures/gpos-single.ttf"))
                .unwrap();
        let gdef = font.font().gdef.as_ref().unwrap();
        for flag in [0, 8] {
            let single = parse_single(&single_bytes(1, &[4], &[[500, 1000, 0, -1000]])).unwrap();
            let mut gpos = single_gpos(vec![single], flag);
            for feature in &mut gpos.features.features {
                if feature.feature_tag == u32::from_be_bytes(*b"kern") {
                    feature.feature_tag = u32::from_be_bytes(*b"vert");
                }
            }
            assert_eq!(
                gpos.lookup_single_adjustment(4, true, Some("ja"), Some(gdef))
                    .is_some(),
                flag == 0
            );
            assert!(gpos
                .lookup_single_adjustment(4, true, Some("ja"), None)
                .is_some());
        }
    }

    #[test]
    fn single_format1_shares_all_four_values() {
        let single = parse_single(&single_bytes(1, &[10, 20], &[[1, -2, 3, -4]])).unwrap();
        let expected = ValueRecord {
            x_placement: 1,
            y_placement: -2,
            x_advance: 3,
            y_advance: -4,
            ..Default::default()
        };
        assert_eq!(single.lookup_single_adjustment(10), Some(expected));
        assert_eq!(single.lookup_single_adjustment(20), Some(expected));
        assert_eq!(single.lookup_single_adjustment(11), None);
    }

    #[test]
    fn single_format2_indexes_each_covered_glyph() {
        let single = parse_single(&single_bytes(
            2,
            &[10, 20],
            &[[1, 2, 3, 4], [-1, -2, -3, -4]],
        ))
        .unwrap();
        assert_eq!(single.lookup_single_adjustment(10).unwrap().x_advance, 3);
        assert_eq!(single.lookup_single_adjustment(20).unwrap().y_advance, -4);
        assert!(single.lookup_single_adjustment(21).is_none());
    }

    #[test]
    fn single_rejects_inconsistent_coverage_and_reserved_format() {
        assert!(parse_single(&single_bytes(2, &[10, 20], &[[1, 2, 3, 4]])).is_err());
        assert!(parse_single(&single_bytes(1, &[20, 10], &[[1, 2, 3, 4]])).is_err());
        let mut bytes = single_bytes(1, &[10], &[[1, 2, 3, 4]]);
        bytes[4..6].copy_from_slice(&0x0100u16.to_be_bytes());
        assert!(parse_single(&bytes).is_err());
        bytes[4..6].copy_from_slice(&0x000fu16.to_be_bytes());
        bytes[2..4].copy_from_slice(&0u16.to_be_bytes());
        assert!(parse_single(&bytes).is_err());
    }

    #[test]
    fn single_rejects_every_truncated_prefix() {
        let bytes = single_bytes(2, &[10, 20], &[[1, 2, 3, 4], [-1, -2, -3, -4]]);
        for length in 0..bytes.len() {
            assert!(parse_single(&bytes[..length]).is_err(), "length {length}");
        }
        assert!(parse_single(&bytes).is_ok());
    }

    #[test]
    fn single_coverage_ranges_cannot_overflow_the_index() {
        let mut bytes = single_bytes(1, &[10], &[[1, 2, 3, 4]]);
        bytes.truncate(14);
        for word in [2u16, 1, 10, 11, 65535] {
            bytes.extend(word.to_be_bytes());
        }
        assert!(parse_single(&bytes).is_err());
        bytes[22..24].copy_from_slice(&0u16.to_be_bytes());
        let single = parse_single(&bytes).unwrap();
        assert_eq!(single.lookup_single_adjustment(11).unwrap().x_advance, 3);
    }

    #[test]
    fn single_consumes_device_offsets_without_shifting_the_next_record() {
        let words = [
            2u16, 28, 0x001f, 2, 1, 2, 3, 4, 0, 11, 12, 13, 14, 0, 1, 2, 10, 20,
        ];
        let bytes: Vec<_> = words.iter().flat_map(|word| word.to_be_bytes()).collect();
        let single = parse_single(&bytes).unwrap();
        assert_eq!(single.lookup_single_adjustment(20).unwrap().x_advance, 13);
    }

    #[test]
    fn single_extension_uses_the_relative_offset() {
        let mut bytes = vec![0, 1, 0, 1, 0, 0, 0, 8];
        bytes.extend(single_bytes(1, &[10], &[[1, 2, 3, 4]]));
        let extension = Gpos::parse_subtable(&mut BytesReader::new(&bytes), 9, 0).unwrap();
        let mut gpos = single_gpos(vec![extension], 0);
        for lookup in &mut gpos.lookups {
            lookup.lookup_type = 9;
        }
        assert_eq!(
            gpos.lookup_single_adjustment(10, false, None, None)
                .unwrap()
                .x_advance,
            3
        );
    }

    #[test]
    fn single_zero_match_stops_subtable_fallback() {
        let zero = parse_single(&single_bytes(1, &[10], &[[0; 4]])).unwrap();
        let nonzero = parse_single(&single_bytes(1, &[10], &[[1; 4]])).unwrap();
        let gpos = single_gpos(vec![zero, nonzero], 0);
        assert_eq!(
            gpos.lookup_single_adjustment(10, false, None, None),
            Some(SingleAdjustment::default())
        );
    }

    #[test]
    fn single_adds_distinct_lookups_and_deduplicates_shared_lookup() {
        let single = parse_single(&single_bytes(1, &[10], &[[1, 2, 3, 4]])).unwrap();
        let mut gpos = single_gpos(vec![single.clone()], 0);
        let index = gpos.lookups.len() as u16;
        gpos.lookups.push(PositioningLookup {
            lookup_type: 1,
            lookup_flag: 0,
            mark_filtering_set: None,
            subtables: vec![single],
        });
        for feature in gpos.features.features.iter_mut() {
            if feature.feature_tag == u32::from_be_bytes(*b"kern") {
                feature.lookup_list_indices.extend([index, index]);
            }
        }
        assert_eq!(
            gpos.lookup_single_adjustment(10, false, None, None)
                .unwrap()
                .x_advance,
            6
        );
    }

    #[test]
    fn single_accumulates_beyond_i16_without_saturation() {
        let mut gpos = single_gpos(vec![], 0);
        let mut indices = Vec::new();
        for value in [20000, 20000, -20000] {
            indices.push(gpos.lookups.len() as u16);
            gpos.lookups.push(PositioningLookup {
                lookup_type: 1,
                lookup_flag: 0,
                mark_filtering_set: None,
                subtables: vec![parse_single(&single_bytes(1, &[10], &[[value; 4]])).unwrap()],
            });
        }
        let kern = gpos
            .features
            .features
            .iter_mut()
            .find(|feature| feature.feature_tag == u32::from_be_bytes(*b"kern"))
            .unwrap();
        kern.lookup_list_indices = indices;
        let adjustment = gpos
            .lookup_single_adjustment(10, false, None, None)
            .unwrap();
        assert_eq!(
            adjustment,
            SingleAdjustment {
                x_placement: 20000,
                y_placement: 20000,
                x_advance: 20000,
                y_advance: 20000
            }
        );
        gpos.lookups.last_mut().unwrap().subtables.clear();
        assert_eq!(
            gpos.lookup_single_adjustment(10, false, None, None)
                .unwrap()
                .x_advance,
            40000
        );
    }

    #[test]
    fn single_filters_gdef_classes_and_respects_mark_precedence() {
        use super::super::gdef::MarkGlyphSetsDef;
        use crate::opentype::layouts::coverage::{Coverage, CoverageFormat1};
        let font =
            crate::load_font_from_buffer(include_bytes!("../../../tests/fixtures/gpos-single.ttf"))
                .unwrap();
        let mut gdef = font.font().gdef.clone().unwrap();
        let single = parse_single(&single_bytes(1, &[1, 4], &[[1; 4]])).unwrap();
        let mut gpos = single_gpos(vec![single], 0x000a);
        assert!(gpos
            .lookup_single_adjustment(1, false, None, Some(&gdef))
            .is_none());
        assert!(gpos
            .lookup_single_adjustment(4, false, None, Some(&gdef))
            .is_none());
        assert!(gpos
            .lookup_single_adjustment(4, false, None, None)
            .is_some());
        gdef.mark_glyph_sets_def = Some(MarkGlyphSetsDef {
            mark_set_table_format: 1,
            mark_set_count: 1,
            coverages: vec![Coverage::Format1(CoverageFormat1 {
                coverage_format: 1,
                glyph_count: 1,
                glyph_ids: vec![4],
            })],
        });
        for lookup in &mut gpos.lookups {
            lookup.lookup_flag = 0x0110;
            lookup.mark_filtering_set = Some(0);
        }
        // The mark set takes precedence over the absent attachment class.
        assert!(gpos
            .lookup_single_adjustment(4, false, None, Some(&gdef))
            .is_some());
        for lookup in &mut gpos.lookups {
            lookup.lookup_flag |= 0x0008;
        }
        assert!(gpos
            .lookup_single_adjustment(4, false, None, Some(&gdef))
            .is_none());
        for lookup in &mut gpos.lookups {
            lookup.lookup_flag = 0x0010;
            lookup.mark_filtering_set = Some(9);
        }
        assert!(gpos
            .lookup_single_adjustment(4, false, None, Some(&gdef))
            .is_none());
        use crate::opentype::layouts::coverage::{CoverageFormat2, RangeRecord};
        gdef.mark_glyph_sets_def.as_mut().unwrap().coverages[0] =
            Coverage::Format2(CoverageFormat2 {
                coverage_format: 2,
                range_count: 1,
                range_records: vec![RangeRecord {
                    start_glyph_id: 3,
                    end_glyph_id: 4,
                    start_coverage_index: 65535,
                }],
            });
        for lookup in &mut gpos.lookups {
            lookup.mark_filtering_set = Some(0);
        }
        assert!(gpos
            .lookup_single_adjustment(4, false, None, Some(&gdef))
            .is_none());
    }

    #[test]
    fn mark_to_base_format1_lookup_returns_anchor_delta() {
        let bytes = [
            0x00, 0x01, // posFormat
            0x00, 0x0C, // markCoverageOffset
            0x00, 0x12, // baseCoverageOffset
            0x00, 0x01, // classCount
            0x00, 0x18, // markArrayOffset
            0x00, 0x24, // baseArrayOffset
            0x00, 0x01, // mark coverage format
            0x00, 0x01, // glyph count
            0x00, 0x14, // glyph 20
            0x00, 0x01, // base coverage format
            0x00, 0x01, // glyph count
            0x00, 0x0A, // glyph 10
            0x00, 0x01, // mark count
            0x00, 0x00, // mark class
            0x00, 0x06, // mark anchor offset
            0x00, 0x01, // anchor format 1
            0x00, 0x03, // mark x
            0x00, 0x07, // mark y
            0x00, 0x01, // base count
            0x00, 0x04, // base anchor offset
            0x00, 0x01, // anchor format 1
            0x00, 0x0A, // base x
            0x00, 0x14, // base y
        ];
        let mut reader = BytesReader::new(&bytes);

        let subtable = Gpos::parse_subtable(&mut reader, 4, 0).expect("parse mark-to-base");
        let adjustment = subtable
            .lookup_mark_to_base_adjustment(10, 20)
            .expect("mark attachment adjustment");

        assert_eq!(
            adjustment,
            MarkAttachmentAdjustment {
                x_placement: 7,
                y_placement: 13,
            }
        );
    }

    #[test]
    fn mark_to_mark_format1_lookup_returns_anchor_delta() {
        let bytes = [
            0x00, 0x01, // posFormat
            0x00, 0x0C, // mark1CoverageOffset
            0x00, 0x12, // mark2CoverageOffset
            0x00, 0x01, // classCount
            0x00, 0x18, // mark1ArrayOffset
            0x00, 0x24, // mark2ArrayOffset
            0x00, 0x01, // mark1 coverage format
            0x00, 0x01, // glyph count
            0x00, 0x1E, // glyph 30
            0x00, 0x01, // mark2 coverage format
            0x00, 0x01, // glyph count
            0x00, 0x14, // glyph 20
            0x00, 0x01, // mark count
            0x00, 0x00, // mark class
            0x00, 0x06, // mark anchor offset
            0x00, 0x01, // anchor format 1
            0x00, 0x04, // mark1 x
            0x00, 0x05, // mark1 y
            0x00, 0x01, // mark2 count
            0x00, 0x04, // mark2 anchor offset
            0x00, 0x01, // anchor format 1
            0x00, 0x0C, // mark2 x
            0x00, 0x14, // mark2 y
        ];
        let mut reader = BytesReader::new(&bytes);

        let subtable = Gpos::parse_subtable(&mut reader, 6, 0).expect("parse mark-to-mark");
        let adjustment = subtable
            .lookup_mark_to_mark_adjustment(20, 30)
            .expect("mark-to-mark adjustment");

        assert_eq!(
            adjustment,
            MarkAttachmentAdjustment {
                x_placement: 8,
                y_placement: 15,
            }
        );
    }

    #[test]
    fn mark_to_ligature_format1_lookup_returns_anchor_delta_for_component() {
        let bytes = [
            0x00, 0x01, // posFormat
            0x00, 0x0C, // markCoverageOffset
            0x00, 0x12, // ligatureCoverageOffset
            0x00, 0x01, // classCount
            0x00, 0x18, // markArrayOffset
            0x00, 0x24, // ligatureArrayOffset
            0x00, 0x01, // mark coverage format
            0x00, 0x01, // glyph count
            0x00, 0x14, // glyph 20
            0x00, 0x01, // ligature coverage format
            0x00, 0x01, // glyph count
            0x00, 0x0A, // glyph 10
            0x00, 0x01, // mark count
            0x00, 0x00, // mark class
            0x00, 0x06, // mark anchor offset
            0x00, 0x01, // anchor format 1
            0x00, 0x03, // mark x
            0x00, 0x07, // mark y
            0x00, 0x01, // ligature count
            0x00, 0x04, // ligature attach offset
            0x00, 0x02, // component count
            0x00, 0x06, // component 0 anchor offset
            0x00, 0x0C, // component 1 anchor offset
            0x00, 0x01, // anchor format 1
            0x00, 0x08, // component 0 x
            0x00, 0x10, // component 0 y
            0x00, 0x01, // anchor format 1
            0x00, 0x12, // component 1 x
            0x00, 0x1A, // component 1 y
        ];
        let mut reader = BytesReader::new(&bytes);

        let subtable = Gpos::parse_subtable(&mut reader, 5, 0).expect("parse mark-to-ligature");
        let adjustment = subtable
            .lookup_mark_to_ligature_adjustment(10, 20, 1)
            .expect("mark-to-ligature adjustment");

        assert_eq!(
            adjustment,
            MarkAttachmentAdjustment {
                x_placement: 15,
                y_placement: 19,
            }
        );
    }
}

#[cfg(test)]
mod extension_header_tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    fn header(format: u16, kind: u16, offset: u32) -> Vec<u8> {
        [
            format.to_be_bytes().as_slice(),
            kind.to_be_bytes().as_slice(),
            offset.to_be_bytes().as_slice(),
        ]
        .concat()
    }

    #[test]
    fn extension_rejects_invalid_format_before_reading_target() {
        for format in [0, 2, u16::MAX] {
            // InvalidData, not a reader error: validate before following the pointer.
            let absent_target = header(format, 1, u32::MAX);
            assert_eq!(
                Gpos::parse_extension(&mut BytesReader::new(&absent_target), 0)
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::InvalidData
            );
            let mut bytes = header(format, 1, 8);
            bytes.extend([1u16, 6, 0, 1, 1, 10].iter().flat_map(|v| v.to_be_bytes()));
            assert_eq!(
                Gpos::parse_extension(&mut BytesReader::new(&bytes), 0)
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn extension_rejects_self_nested_and_unknown_types_before_dispatch() {
        for kind in [0, 9, 10, 65535] {
            for offset in [0, 8, u32::MAX] {
                let mut bytes = header(1, kind, offset);
                // Even a finite nested extension is structurally invalid.
                bytes.extend(header(1, 1, 8));
                bytes.extend([1u16, 6, 0, 1, 1, 10].iter().flat_map(|v| v.to_be_bytes()));
                assert_eq!(
                    Gpos::parse_extension(&mut BytesReader::new(&bytes), 0)
                        .unwrap_err()
                        .kind(),
                    std::io::ErrorKind::InvalidData
                );
            }
        }
    }

    #[test]
    fn extension_rejects_null_truncated_and_out_of_range_targets() {
        let bytes = header(1, 1, 8);
        for length in 0..=bytes.len() {
            assert!(Gpos::parse_extension(&mut BytesReader::new(&bytes[..length]), 0).is_err());
        }
        for offset in [0, 9, u32::MAX] {
            assert!(
                Gpos::parse_extension(&mut BytesReader::new(&header(1, 1, offset)), 0).is_err()
            );
        }
    }

    #[test]
    fn extension_preserves_relative_single_and_valid_unsupported_types() {
        for (base, relative) in [(0usize, 8usize), (13, 65544)] {
            let mut bytes = vec![0; base];
            bytes.extend(header(1, 1, relative as u32));
            bytes.resize(base + relative, 0);
            // SinglePos with xAdvance=3, coverage glyph 10.
            let target: Vec<_> = [1u16, 8, 4, 3, 1, 1, 10]
                .iter()
                .flat_map(|v| v.to_be_bytes())
                .collect();
            bytes.extend(&target);
            let extension =
                Gpos::parse_extension(&mut BytesReader::new(&bytes), base as u64).unwrap();
            assert_eq!(extension.lookup_single_adjustment(10).unwrap().x_advance, 3);
            for length in 0..target.len() {
                assert!(Gpos::parse_extension(
                    &mut BytesReader::new(&bytes[..base + relative + length]),
                    base as u64
                )
                .is_err());
            }
        }
        for kind in [7, 8] {
            let bytes = header(1, kind, 8);
            let PositioningSubtable::Extension(inner) =
                Gpos::parse_extension(&mut BytesReader::new(&bytes), 0).unwrap()
            else {
                panic!("extension")
            };
            assert!(matches!(*inner, PositioningSubtable::Unsupported));
        }
    }

    #[test]
    fn value_record_variation_index_applies_gdef_store_delta() {
        let font = crate::fontreader::Font::get_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/gdef-carets.ttc"
        ))
        .unwrap();
        // Face 4 has a valid CaretVariationStore (outer=0, inner=0 -> +25 at 0.25, +100 at 1.0).
        let gdef = font.more_fonts[3].gdef.as_ref().unwrap();
        // SinglePos format 1: coverage at 10, valueFormat = 0x0044 (xAdvance + xAdvDevice),
        // xAdvance = -10, xAdvDevice offset = 16 -> VariationIndex(outer=0, inner=0, format=0x8000).
        let words: [u16; 11] = [
            1,      // format 1
            10,     // coverage offset
            0x0044, // xAdvance | xAdvDevice
            (-10i16) as u16,
            16, // xAdvDevice offset
            1,
            1,
            10, // Coverage Format 1: glyph 10
            0,
            0,
            0x8000, // VariationIndex (outer=0, inner=0, deltaFormat=0x8000)
        ];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_be_bytes()).collect();
        let subtable = Gpos::parse_single_adjustment(&mut BytesReader::new(&bytes), 0).unwrap();
        let raw = subtable.lookup_single_adjustment(10).unwrap();
        assert_eq!(raw.x_advance, -10);
        assert_eq!(raw.x_adv_var, Some((0, 0)));
        assert_eq!(raw.resolve(Some(gdef), &[]).x_advance, -10);
        assert_eq!(raw.resolve(Some(gdef), &[0.25]).x_advance, 15);
        assert_eq!(raw.resolve(Some(gdef), &[1.0]).x_advance, 90);
        // Malformed store or out-of-bounds VariationIndex fails closed to base value.
        let mut bad_gdef = gdef.clone();
        bad_gdef.caret_variation_store = Some(Err(
            crate::opentype::extentions::gdef_variation::VariationError::Malformed,
        ));
        assert_eq!(raw.resolve(Some(&bad_gdef), &[1.0]).x_advance, -10);
        let mut oob = raw;
        oob.x_adv_var = Some((99, 99));
        assert_eq!(oob.resolve(Some(gdef), &[1.0]).x_advance, -10);
    }

    #[test]
    fn cursive_attachment_format1_chains_vertical_anchors_ltr_and_rtl() {
        // CursivePosFormat1: format=1, coverageOffset=14, count=2,
        // rec0(glyph 10): entry=0, exit=22 -> Anchor(x=100, y=50)
        // rec1(glyph 11): entry=28 -> Anchor(x=0, y=20), exit=0
        let words: [u16; 17] = [
            1,  // format 1
            14, // coverageOffset (byte 14)
            2,  // entryExitCount
            0, 22, // rec 0: entry=0, exit=22
            28, 0, // rec 1: entry=28, exit=0
            1, 2, 10, 11, // Coverage Format 1: glyphs 10, 11 (8 bytes: 14..22)
            1, 100, 50, // exit anchor at 22: format 1, x=100, y=50
            1, 0, 20, // entry anchor at 28: format 1, x=0, y=20
        ];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_be_bytes()).collect();
        let subtable = Gpos::parse_cursive_attachment(&mut BytesReader::new(&bytes), 0).unwrap();
        let make_gpos = |lookup_flag: u16| {
            let base = include_bytes!("../../../tests/fixtures/gpos-single.bin");
            let mut gpos = Gpos::new(&mut BytesReader::new(base), 0, base.len() as u32).unwrap();
            let feature = gpos
                .features
                .features
                .iter_mut()
                .find(|f| f.feature_tag == u32::from_be_bytes(*b"kern"))
                .unwrap();
            feature.feature_tag = u32::from_be_bytes(*b"curs");
            let lookup_index = feature.lookup_list_indices[0] as usize;
            for lookup in &mut gpos.lookups {
                lookup.subtables.clear();
            }
            gpos.lookups[lookup_index] = PositioningLookup {
                lookup_type: 3,
                lookup_flag,
                mark_filtering_set: None,
                subtables: vec![subtable.clone()],
            };
            gpos
        };
        // LTR chaining (flag 0): glyph 10 stays at y=0, glyph 11 shifts by exit.y - entry.y = +30.
        let ltr = make_gpos(0).cursive_adjustments(&[Some(10), Some(11)], None, None);
        assert_eq!(ltr[0].y_placement, 0);
        assert_eq!(ltr[1].y_placement, 30);

        // RTL chaining (flag 0x0001): glyph 11 anchors at y=0, glyph 10 shifts by -30.
        let rtl = make_gpos(0x0001).cursive_adjustments(&[Some(10), Some(11)], None, None);
        assert_eq!(rtl[0].y_placement, -30);
        assert_eq!(rtl[1].y_placement, 0);
    }
}
