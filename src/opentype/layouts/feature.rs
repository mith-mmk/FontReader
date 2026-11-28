#![allow(dead_code)]

use super::*;
use bin_rs::reader::BinaryReader;
use std::io::SeekFrom;

#[derive(Debug, Clone)]
pub(crate) struct Feature {
    pub(crate) feature_tag: u32,
    feature_offset: u16,
    pub(crate) feature_params: Option<FeatureParams>,
    pub(crate) lookup_list_indices: Vec<u16>,
}

impl Feature {
    fn dump_text(&self) -> String {
        let mut bytes = [0; 4];
        for i in 0..4 {
            bytes[3 - i] = (self.feature_tag >> (i * 8)) as u8;
        }
        let tag = String::from_utf8_lossy(&bytes);
        let mut string = format!("FeatureTag: {}\n", tag);
        string += &format!("FeatureParams: {:?}\n", self.feature_params);
        string += &format!("LookupListIndices: {:?}\n", self.lookup_list_indices);
        if let Some(feature_params) = &self.feature_params {
            string += &format!("FeatureParams: {}\n", feature_params);
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureParams {
    pub(crate) feature_params: Vec<u8>,
}

impl FeatureParams {
    fn dump_text(&self) -> String {
        let mut string = String::new();
        for param in self.feature_params.iter() {
            string += &format!("{:02X} ", param);
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureList {
    pub(crate) feature_count: u16,
    pub(crate) features: Vec<Feature>,
}

impl FeatureList {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        table_end: u64,
    ) -> Result<FeatureList, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let feature_count = reader.read_u16_be()?;
        let mut features = Vec::new();
        for _ in 0..feature_count {
            let feature_tag = reader.read_u32_be()?;
            let feature_offset = reader.read_u16_be()?;
            features.push(Feature {
                feature_tag,
                feature_offset,
                feature_params: None,
                lookup_list_indices: Vec::new(),
            });
        }
        for index in 0..features.len() {
            let next_feature_start = features[index + 1..]
                .iter()
                .map(|next| offset + next.feature_offset as u64)
                .filter(|next_offset| *next_offset > offset + features[index].feature_offset as u64)
                .min()
                .unwrap_or(table_end);
            let feature = &mut features[index];
            let feature_start = offset + feature.feature_offset as u64;
            if feature_start + 4 > table_end {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "feature table offset {} exceeds layout table end {}",
                        feature_start, table_end
                    ),
                ));
            }

            reader.seek(SeekFrom::Start(feature_start))?;
            let feature_params_offset = reader.read_u16_be()?;
            let lookup_count = reader.read_u16_be()?;
            for _ in 0..lookup_count {
                feature.lookup_list_indices.push(reader.read_u16_be()?);
            }
            if feature_params_offset > 0 {
                let params_start = feature_start + feature_params_offset as u64;
                let params_end = next_feature_start.min(table_end);
                if params_start > params_end {
                    continue;
                }
                reader.seek(SeekFrom::Start(params_start))?;
                let feature_params =
                    reader.read_bytes_as_vec((params_end - params_start) as usize)?;
                feature.feature_params = Some(FeatureParams {
                    feature_params: feature_params.to_vec(),
                });
            }
        }

        Ok(Self {
            feature_count,
            features,
        })
    }

    fn dump_text(&self) -> String {
        let mut string = format!("FeatureCount: {}\n", self.feature_count);
        for feature in self.features.iter() {
            string += &format!("{}\n", feature);
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariation {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) feature_variations: Vec<FeatureVariationRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariationRecord {
    pub(crate) condition_set_offset: u32,
    pub(crate) feature_table_substitution_offset: u32,
    pub(crate) conditions: Vec<ConditionTable>,
    pub(crate) substitutions: Vec<(u16, Feature)>,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariations {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) condition_set_count: u16,
    pub(crate) condition_sets: Vec<ConditionSet>,
    pub(crate) feature_table_substitutions: Vec<FeatureTableSubstitution>,
}
impl FeatureVariations {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        _length: u32,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset))?;
        let major_version = reader.read_u16_be()?;
        let minor_version = reader.read_u16_be()?;
        let feature_table_substitution_count = reader.read_u16_be()?;
        let mut feature_table_substitutions = Vec::new();
        for _ in 0..feature_table_substitution_count {
            let feature_table_substitution_offset = reader.read_u16_be()?;
            feature_table_substitutions.push(FeatureTableSubstitution {
                feature_table_substitution: feature_table_substitution_offset,
            });
        }
        let condition_set_count = reader.read_u16_be()?;
        let mut condition_sets = Vec::new();
        for _ in 0..condition_set_count {
            let condition_count = reader.read_u16_be()?;
            let mut conditions = Vec::new();
            for _ in 0..condition_count {
                let format = reader.read_u16_be()?;
                let axis_index = reader.read_u16_be()?;
                let filter_range_min_value = reader.read_f32_be()?;
                let filter_range_max_value = reader.read_f32_be()?;
                conditions.push(ConditionTable {
                    format,
                    axis_index,
                    filter_range_min_value,
                    filter_range_max_value,
                });
            }
            condition_sets.push(ConditionSet {
                condition_count,
                conditions,
            });
        }
        Ok(Self {
            major_version,
            minor_version,
            condition_set_count,
            condition_sets,
            feature_table_substitutions,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureTableSubstitution {
    pub(crate) feature_table_substitution: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariationRecordList {
    pub(crate) feature_variation_record_count: u16,
    pub(crate) feature_variation_records: Vec<FeatureVariationRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariationList {
    pub(crate) feature_variation_count: u32,
    pub(crate) feature_variations: Vec<FeatureVariationRecord>,
}
impl FeatureVariationList {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        table_end: u64,
    ) -> Result<Self, std::io::Error> {
        if offset + 8 > table_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "FeatureVariations header truncated",
            ));
        }
        reader.seek(SeekFrom::Start(offset))?;
        let major_version = reader.read_u16_be()?;
        let minor_version = reader.read_u16_be()?;
        if major_version != 1 || minor_version != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unsupported FeatureVariations version",
            ));
        }
        let feature_variation_count = reader.read_u32_be()?;
        if feature_variation_count > 4096 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "FeatureVariations count exceeds supported limit",
            ));
        }
        let records_end = offset
            .checked_add(8)
            .and_then(|v| v.checked_add(feature_variation_count as u64 * 8))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "FeatureVariations range overflow",
                )
            })?;
        if records_end > table_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "FeatureVariations records exceed table bounds",
            ));
        }
        let mut raw_records = Vec::with_capacity(feature_variation_count as usize);
        for _ in 0..feature_variation_count {
            let condition_set_offset = reader.read_u32_be()?;
            let feature_table_substitution_offset = reader.read_u32_be()?;
            raw_records.push((condition_set_offset, feature_table_substitution_offset));
        }

        let mut feature_variations = Vec::with_capacity(raw_records.len());
        for (condition_set_offset, feature_table_substitution_offset) in raw_records {
            let conditions = if condition_set_offset != 0 {
                let cs_start =
                    offset
                        .checked_add(condition_set_offset as u64)
                        .ok_or_else(|| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "ConditionSet offset overflow",
                            )
                        })?;
                parse_condition_set(reader, cs_start, table_end)?
            } else {
                Vec::new()
            };

            let substitutions = if feature_table_substitution_offset != 0 {
                let fts_start = offset
                    .checked_add(feature_table_substitution_offset as u64)
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "FeatureTableSubstitution offset overflow",
                        )
                    })?;
                parse_feature_table_substitution(reader, fts_start, table_end)?
            } else {
                Vec::new()
            };

            feature_variations.push(FeatureVariationRecord {
                condition_set_offset,
                feature_table_substitution_offset,
                conditions,
                substitutions,
            });
        }
        Ok(Self {
            feature_variation_count,
            feature_variations,
        })
    }

    pub(crate) fn substitute_feature(
        &self,
        normalized_coords: &[f32],
        feature_index: u16,
    ) -> Option<&Feature> {
        for record in &self.feature_variations {
            let matched = record.conditions.iter().all(|cond| {
                if cond.format != 1 {
                    return false;
                }
                let coord = normalized_coords
                    .get(cond.axis_index as usize)
                    .copied()
                    .unwrap_or(0.0);
                coord >= cond.filter_range_min_value && coord <= cond.filter_range_max_value
            });
            if matched {
                return record
                    .substitutions
                    .iter()
                    .find(|(idx, _)| *idx == feature_index)
                    .map(|(_, feature)| feature);
            }
        }
        None
    }
}

fn parse_condition_set<R: BinaryReader>(
    reader: &mut R,
    cs_start: u64,
    table_end: u64,
) -> Result<Vec<ConditionTable>, std::io::Error> {
    if cs_start + 2 > table_end {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "ConditionSet header exceeds table bounds",
        ));
    }
    reader.seek(SeekFrom::Start(cs_start))?;
    let condition_count = reader.read_u16_be()?;
    let list_end = cs_start
        .checked_add(2)
        .and_then(|v| v.checked_add(condition_count as u64 * 4))
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "ConditionSet overflow")
        })?;
    if list_end > table_end {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "ConditionSet offsets exceed table bounds",
        ));
    }
    let mut offsets = Vec::with_capacity(condition_count as usize);
    for _ in 0..condition_count {
        offsets.push(reader.read_u32_be()?);
    }
    let mut conditions = Vec::with_capacity(offsets.len());
    for rel in offsets {
        let cond_start = cs_start.checked_add(rel as u64).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Condition offset overflow")
        })?;
        if cond_start + 8 > table_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Condition table exceeds table bounds",
            ));
        }
        reader.seek(SeekFrom::Start(cond_start))?;
        let format = reader.read_u16_be()?;
        let axis_index = reader.read_u16_be()?;
        let min_raw = reader.read_i16_be()?;
        let max_raw = reader.read_i16_be()?;
        conditions.push(ConditionTable {
            format,
            axis_index,
            filter_range_min_value: min_raw as f32 / 16384.0,
            filter_range_max_value: max_raw as f32 / 16384.0,
        });
    }
    Ok(conditions)
}

fn parse_feature_table_substitution<R: BinaryReader>(
    reader: &mut R,
    fts_start: u64,
    table_end: u64,
) -> Result<Vec<(u16, Feature)>, std::io::Error> {
    if fts_start + 6 > table_end {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "FeatureTableSubstitution header exceeds table bounds",
        ));
    }
    reader.seek(SeekFrom::Start(fts_start))?;
    let major = reader.read_u16_be()?;
    let minor = reader.read_u16_be()?;
    if major != 1 || minor != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unsupported FeatureTableSubstitution version",
        ));
    }
    let sub_count = reader.read_u16_be()?;
    let recs_end = fts_start
        .checked_add(6)
        .and_then(|v| v.checked_add(sub_count as u64 * 6))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "FeatureTableSubstitution overflow",
            )
        })?;
    if recs_end > table_end {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "FeatureTableSubstitution records exceed table bounds",
        ));
    }
    let mut records = Vec::with_capacity(sub_count as usize);
    for _ in 0..sub_count {
        let feature_index = reader.read_u16_be()?;
        let alt_offset = reader.read_u32_be()?;
        records.push((feature_index, alt_offset));
    }
    let mut substitutions = Vec::with_capacity(records.len());
    for (feature_index, alt_offset) in records {
        let alt_start = fts_start.checked_add(alt_offset as u64).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "alternate feature offset overflow",
            )
        })?;
        if alt_start + 4 > table_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "alternate feature exceeds table bounds",
            ));
        }
        reader.seek(SeekFrom::Start(alt_start))?;
        let _params_offset = reader.read_u16_be()?;
        let lookup_count = reader.read_u16_be()?;
        let lookups_end = alt_start
            .checked_add(4)
            .and_then(|v| v.checked_add(lookup_count as u64 * 2))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "alternate feature lookup overflow",
                )
            })?;
        if lookups_end > table_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "alternate feature lookups exceed table bounds",
            ));
        }
        let mut lookup_list_indices = Vec::with_capacity(lookup_count as usize);
        for _ in 0..lookup_count {
            lookup_list_indices.push(reader.read_u16_be()?);
        }
        substitutions.push((
            feature_index,
            Feature {
                feature_tag: 0,
                feature_offset: 0,
                feature_params: None,
                lookup_list_indices,
            },
        ));
    }
    Ok(substitutions)
}

impl std::fmt::Display for Feature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

impl std::fmt::Display for FeatureParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

impl std::fmt::Display for FeatureList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn display_preserves_feature_list_dump() {
        let list = FeatureList {
            feature_count: 1,
            features: vec![Feature {
                feature_tag: u32::from_be_bytes(*b"liga"),
                feature_offset: 0,
                feature_params: Some(FeatureParams {
                    feature_params: vec![0, 255],
                }),
                lookup_list_indices: vec![2],
            }],
        };
        let expected = "FeatureCount: 1\nFeatureTag: liga\nFeatureParams: Some(FeatureParams { feature_params: [0, 255] })\nLookupListIndices: [2]\nFeatureParams: 00 FF \n\n";
        assert_eq!(list.to_string(), expected);
        assert_eq!(format!("{list}"), expected);
    }
}
