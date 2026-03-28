#![allow(dead_code)]

use std::collections::HashSet;
use std::io::SeekFrom;

use crate::opentype::layouts::{
    classdef::ClassDef,
    coverage::Coverage,
    script::ParsedScript,
    FeatureList, FeatureVariationList, ScriptList,
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

    fn parse<R: BinaryReader>(
        reader: &mut R,
        value_format: u16,
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

        for flag in [0x0010u16, 0x0020, 0x0040, 0x0080, 0x0100, 0x0200, 0x0400, 0x0800] {
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

#[derive(Debug, Clone)]
enum PositioningSubtable {
    PairFormat1(PairPosFormat1),
    PairFormat2(PairPosFormat2),
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
            length,
        ));
        let lookups = Self::parse_lookups(reader, offset + lookup_list_offset as u64)?;
        let feature_variations = if feature_variations_offset > 0 {
            Some(Box::new(FeatureVariationList::new(
                reader,
                offset + feature_variations_offset as u64,
                length,
            )))
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

    fn parse_extension<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<PositioningSubtable, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let _pos_format = reader.read_u16_be()?;
        let extension_lookup_type = reader.read_u16_be()?;
        let extension_offset = reader.read_u32_be()?;
        Ok(PositioningSubtable::Extension(Box::new(Self::parse_subtable(
            reader,
            extension_lookup_type,
            offset + extension_offset as u64,
        )?)))
    }

    fn locale_to_language_system_tag(locale: &str) -> Option<u32> {
        let locale = locale.trim();
        if locale.is_empty() {
            return None;
        }

        let primary = locale
            .split(['-', '_'])
            .next()
            .unwrap_or(locale)
            .trim();
        if primary.is_empty() {
            return None;
        }

        let lower = primary.to_ascii_lowercase();
        let tag = match lower.as_str() {
            "default" => [0, 0, 0, 0],
            "ja" | "jp" | "jpn" => *b"JAN ",
            _ => {
                let mut tag = [b' '; 4];
                for (i, ch) in primary.chars().take(4).enumerate() {
                    tag[i] = ch.to_ascii_uppercase() as u8;
                }
                tag
            }
        };

        Some(u32::from_be_bytes(tag))
    }

    fn get_language_systems<'a>(
        &'a self,
        script: &'a ParsedScript,
        locale: Option<&str>,
    ) -> Vec<&'a crate::opentype::layouts::LanguageSystemRecord> {
        let mut systems = Vec::new();

        if let Some(locale) = locale {
            if let Some(tag) = Self::locale_to_language_system_tag(locale) {
                if let Some(language_system) = script
                    .language_systems
                    .iter()
                    .find(|record| record.language_system_tag == tag)
                {
                    systems.push(language_system);
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
        let mut result = Vec::new();
        let mut seen_lookup_indices = HashSet::new();

        for script in self.scripts.scripts.iter() {
            for language_system in self.get_language_systems(script, locale) {
                for feature_index in language_system.language_system.feature_indexes.iter() {
                    let feature = &self.features.features[*feature_index as usize];
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
}
