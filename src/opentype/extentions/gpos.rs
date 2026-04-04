#![allow(dead_code)]

use std::collections::HashSet;
use std::io::SeekFrom;

use crate::opentype::layouts::{
    classdef::ClassDef, coverage::Coverage, script::ParsedScript, FeatureList,
    FeatureVariationList, ScriptList,
};
use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ValueRecord {
    pub(crate) x_placement: i16,
    pub(crate) y_placement: i16,
    pub(crate) x_advance: i16,
    pub(crate) y_advance: i16,
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

    fn parse<R: BinaryReader>(reader: &mut R, value_format: u16) -> Result<Self, std::io::Error> {
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

        for flag in [
            0x0010u16, 0x0020, 0x0040, 0x0080, 0x0100, 0x0200, 0x0400, 0x0800,
        ] {
            if value_format & flag != 0 {
                let _ = reader.read_u16_be()?;
            }
        }

        Ok(value)
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MarkAttachmentAdjustment {
    pub(crate) x_placement: i16,
    pub(crate) y_placement: i16,
}

#[derive(Debug, Clone)]
enum PositioningSubtable {
    PairFormat1(PairPosFormat1),
    PairFormat2(PairPosFormat2),
    MarkToBaseFormat1(MarkToBaseFormat1),
    Extension(Box<PositioningSubtable>),
    Unsupported,
}

impl PositioningSubtable {
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
                if pair.coverage.contains(left as usize).is_none() {
                    return None;
                }
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
            PositioningSubtable::Unsupported => None,
            PositioningSubtable::MarkToBaseFormat1(_) => None,
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
            | PositioningSubtable::Unsupported => None,
        }
    }
}

#[derive(Debug, Clone)]
struct PositioningLookup {
    lookup_type: u16,
    subtables: Vec<PositioningSubtable>,
}

#[derive(Debug, Clone)]
pub(crate) struct GPOS {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) scripts: Box<ScriptList>,
    pub(crate) features: Box<FeatureList>,
    lookups: Vec<PositioningLookup>,
    pub(crate) feature_variations: Option<Box<FeatureVariationList>>,
}

impl GPOS {
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
            reader.read_u16_be()?
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
            FeatureVariationList::new(reader, offset + feature_variations_offset as u64, length)
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
            let _lookup_flag = reader.read_u16_be()?;
            let subtable_count = reader.read_u16_be()?;
            let mut subtable_offsets = Vec::with_capacity(subtable_count as usize);
            for _ in 0..subtable_count {
                subtable_offsets.push(reader.read_u16_be()?);
            }

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
            2 => Self::parse_pair_adjustment(reader, offset),
            4 => Self::parse_mark_to_base(reader, offset),
            9 => Self::parse_extension(reader, offset),
            _ => Ok(PositioningSubtable::Unsupported),
        }
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
                        let value1 = ValueRecord::parse(reader, value_format1)?;
                        let value2 = ValueRecord::parse(reader, value_format2)?;
                        pair_value_records.push(PairValueRecord {
                            second_glyph,
                            value1,
                            value2,
                        });
                    }
                    pair_sets.push(PairSet { pair_value_records });
                }

                Ok(PositioningSubtable::PairFormat1(PairPosFormat1 {
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
                            value1: ValueRecord::parse(reader, value_format1)?,
                            value2: ValueRecord::parse(reader, value_format2)?,
                        });
                    }
                    class1_records.push(class2_records);
                }

                Ok(PositioningSubtable::PairFormat2(PairPosFormat2 {
                    coverage: Coverage::new(reader, offset + coverage_offset as u64)?,
                    class_def1: ClassDef::new(reader, offset + class_def1_offset as u64)?,
                    class_def2: ClassDef::new(reader, offset + class_def2_offset as u64)?,
                    class1_records,
                }))
            }
            _ => Ok(PositioningSubtable::Unsupported),
        }
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
            mark_array: Self::parse_mark_array(reader, offset + mark_array_offset as u64, offset)?,
            base_array: Self::parse_base_array(
                reader,
                offset + base_array_offset as u64,
                offset,
                class_count,
            )?,
        }))
    }

    fn parse_mark_array<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        _subtable_offset: u64,
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
        _subtable_offset: u64,
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
        let _pos_format = reader.read_u16_be()?;
        let extension_lookup_type = reader.read_u16_be()?;
        let extension_offset = reader.read_u32_be()?;
        Ok(PositioningSubtable::Extension(Box::new(
            Self::parse_subtable(
                reader,
                extension_lookup_type,
                offset + extension_offset as u64,
            )?,
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
        if locale.is_none() {
            return self.collect_lookups_from_scripts(
                self.scripts.scripts.iter().collect(),
                locale,
                feature_tags,
            );
        }

        let (preferred_scripts, default_scripts, other_scripts) = self.partition_scripts(locale);
        let mut result = Vec::new();
        let mut seen_lookup_ptrs = HashSet::new();

        for feature_tag in feature_tags {
            let lookups = {
                let preferred = self.collect_lookups_from_scripts(
                    preferred_scripts.clone(),
                    locale,
                    std::slice::from_ref(feature_tag),
                );
                if !preferred.is_empty() {
                    preferred
                } else {
                    let defaults = self.collect_lookups_from_scripts(
                        default_scripts.clone(),
                        locale,
                        std::slice::from_ref(feature_tag),
                    );
                    if !defaults.is_empty() {
                        defaults
                    } else {
                        self.collect_lookups_from_scripts(
                            other_scripts.clone(),
                            locale,
                            std::slice::from_ref(feature_tag),
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
        let mut result = Vec::new();
        let mut seen_lookup_indices = HashSet::new();

        for script in scripts {
            for language_system in self.get_language_systems(script, locale) {
                for feature_index in
                    Self::collect_language_system_feature_indices(&language_system.language_system)
                {
                    let feature = &self.features.features[feature_index as usize];
                    if !feature_tags
                        .iter()
                        .any(|tag| feature.feature_tag == u32::from_be_bytes(*tag))
                    {
                        continue;
                    }

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
                }
            }
        }

        if matched && !adjustment.is_zero() {
            Some(adjustment)
        } else {
            None
        }
    }

    pub(crate) fn lookup_mark_to_base_adjustment(
        &self,
        base: u16,
        mark: u16,
        locale: Option<&str>,
    ) -> Option<MarkAttachmentAdjustment> {
        let feature_tags: &[[u8; 4]] = &[*b"mark"];

        for lookup in self.collect_lookups(locale, feature_tags) {
            if lookup.lookup_type != 4 && lookup.lookup_type != 9 {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

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

        let subtable = GPOS::parse_subtable(&mut reader, 4, 0).expect("parse mark-to-base");
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
}
