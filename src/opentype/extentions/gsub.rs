// GSUB -- Glyph Substitution Table

// only check lookup Format1.1 , 4, 6.1, 7

use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits;

#[derive(Debug, Clone)]
pub(crate) struct GSUB {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) scripts: Box<ScriptList>,
    pub(crate) features: Box<FeatureList>,
    pub(crate) lookups: Box<LookupList>,
    // version 1.1
    pub(crate) feature_variations: Option<Box<FeatureVariationList>>,
}

impl GSUB {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R, offset: u32, length: u32) -> Self {
        let offset = offset as u64;

        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let major_version = reader.read_u16_be().unwrap();
        let minor_version = reader.read_u16_be().unwrap();
        let script_list_offset = reader.read_u16_be().unwrap();
        let feature_list_offset = reader.read_u16_be().unwrap();
        let lookup_list_offset = reader.read_u16_be().unwrap();
        let feature_variations_offset = if major_version == 1 && minor_version == 1 {
            reader.read_u16_be().unwrap()
        } else {
            0
        };

        let scripts = Box::new(ScriptList::new(
            reader,
            offset + script_list_offset as u64,
            length,
        ));
        let features = Box::new(FeatureList::new(
            reader,
            offset + feature_list_offset as u64,
            length,
        ));
        let lookups = Box::new(LookupList::new(
            reader,
            offset + lookup_list_offset as u64,
            length,
        ));
        let feature_variations = if feature_variations_offset > 0 {
            Some(Box::new(FeatureVariationList::new(
                reader,
                offset + feature_variations_offset as u64,
                length,
            )))
        } else {
            None
        };
        Self {
            major_version,
            minor_version,
            scripts,
            features,
            lookups,
            feature_variations,
        }
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "GSUB\n".to_string();
        string += &format!("MajorVersion: {}\n", self.major_version);
        string += &format!("MinorVersion: {}\n", self.minor_version);
        string += &format!("Scripts:\n{}\n", self.scripts.to_string());
        string += &format!("Features:\n{}\n", self.features.to_string());
        string += &format!("Lookups:\n{}\n", self.lookups.to_string());
        string += &format!("FeatureVariations:\n{:?}\n", self.feature_variations);
        string
    }

    pub fn lookup_ccmp(&self, griph_ids: Vec<u32>) -> u32{

        0

    }
}

#[derive(Debug, Clone)]

pub(crate) struct LookupList {
    pub(crate) lookups: Box<Vec<Lookup>>,
}

impl LookupList {
    pub(crate) fn to_string(&self) -> String {
        let mut string = format!("LookupCount: {}\n", self.lookups.len());
        for lookup in self.lookups.iter() {
            string += &format!("{}\n", lookup.to_string());
        }
        string
    }
}


#[derive(Debug, Clone)]
pub(crate) struct Lookup {
    pub(crate) lookup_type: u16,
    pub(crate) lookup_flag: u16,
    pub(crate) subtables: Vec<LookupSubstitution>,
}

impl Lookup {
    pub(crate) fn to_string(&self) -> String {
        let mut string = format!("LookupType: {}\n", self.lookup_type);
        string += &format!("LookupFlag: {}\n", self.lookup_flag);
        string += &format!("Subtables:\n");
        for subtable in self.subtables.iter() {
            string += &format!("{}\n", subtable.to_string());
        }
        string
    }
}


#[derive(Debug, Clone)]
pub(crate) struct LookupRaw {
    pub(crate) offset: u64,
    pub(crate) lookup_type: u16,
    pub(crate) lookup_flag: u16,
    pub(crate) subtable_count: u16,
    pub(crate) subtable_offsets: Vec<u16>,
}

#[derive(FromPrimitive, ToPrimitive, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum LookupType {
    SingleSubstitution = 1,
    MultipleSubstitution = 2,
    AlternateSubstitution = 3,
    LigatureSubstitution = 4,
    ContextSubstitution = 5,
    ChainingContextSubstitution = 6,
    ExtensionSubstitution = 7,
    ReverseChainingContextualSingleSubstitution = 8,
}

#[derive(Debug, Clone)]
pub enum LookupFlag {
    RightToLeft = 0x0001,
    IgnoreBaseGlyphs = 0x0002,
    IgnoreLigatures = 0x0004,
    IgnoreMarks = 0x0008,
    UseMarkFilteringSet = 0x0010,
}

impl LookupList {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R, offset: u64, _: u32) -> Self {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let lookup_count = reader.read_u16_be().unwrap();
        let mut lookup_offsets = (0..lookup_count)
            .map(|_| reader.read_u16_be().unwrap())
            .collect::<Vec<u16>>();


        let mut lookups = Vec::new();
        for _ in 0..lookup_count {
            let offset = offset + lookup_offsets.pop().unwrap() as u64;
            reader.seek(SeekFrom::Start(offset as u64)).unwrap();
            let lookup_type = reader.read_u16_be().unwrap();
            let lookup_flag = reader.read_u16_be().unwrap();
            let subtable_count = reader.read_u16_be().unwrap();

            let mut subtable_offsets = Vec::new();
            for _ in 0..subtable_count {
                subtable_offsets.push(reader.read_u16_be().unwrap());
            }

            lookups.push(LookupRaw {
                offset,
                lookup_type,
                lookup_flag,
                subtable_count,
                subtable_offsets,
            });
        }
        let mut lookups_parsed = Vec::new();
        for lookup in lookups.iter_mut() {

            let mut subtables = Vec::new();
            let offset = lookup.offset;
            for subtable_offset in lookup.subtable_offsets.iter() {
                let offset = offset + *subtable_offset as u64;

                let lookup_type = num_traits::FromPrimitive::from_u16(lookup.lookup_type).unwrap();
                let subtable = match lookup_type {
                    LookupType::SingleSubstitution => Self::get_single(reader, offset),
                    LookupType::MultipleSubstitution => Self::get_multiple(reader, offset),
                    LookupType::AlternateSubstitution => Self::get_alternate(reader, offset),
                    LookupType::LigatureSubstitution => Self::get_ligature(reader, offset),
                    LookupType::ContextSubstitution => Self::get_context(reader, offset),
                    LookupType::ChainingContextSubstitution => {
                        Self::get_chaining_context(reader, offset)
                    }
                    LookupType::ExtensionSubstitution => Self::get_extension(reader, offset),
                    LookupType::ReverseChainingContextualSingleSubstitution => {
                        Self::get_reverse_chaining_context(reader, offset)
                    }
                    _ => {
                        panic!("Unknown lookup type: {}", lookup.lookup_type);
                    }
                };
                subtables.push(subtable);
            }
            lookups_parsed.push(Lookup {
                lookup_type: lookup.lookup_type,
                lookup_flag: lookup.lookup_flag,
                subtables: subtables,
            });
        }

        Self {
            lookups: Box::new(lookups_parsed),
        }
    }
    fn get_single<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let coverage_offset = reader.read_u16_be().unwrap();
        if subst_format != 2 {
            let delta_glyph_id = reader.read_i16_be().unwrap();
            let coverage = Self::get_coverage(reader, offset + coverage_offset as u64);
            return LookupSubstitution::Single(SingleSubstitutionFormat1 {
                subst_format,
                coverage,
                delta_glyph_id,
            });
        }
        let glyph_count = reader.read_u16_be().unwrap();
        let mut substitute_glyph_ids = Vec::new();
        for _ in 0..glyph_count {
            substitute_glyph_ids.push(reader.read_u16_be().unwrap());
        }
        let coverage = Self::get_coverage(reader, offset + coverage_offset as u64);
        LookupSubstitution::Single2(SingleSubstitutionFormat2 {
            subst_format,
            coverage,
            glyph_count,
            substitute_glyph_ids: substitute_glyph_ids,
        })
    }

    fn get_multiple<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let coverage_offset = reader.read_u16_be().unwrap();
        let sequence_count = reader.read_u16_be().unwrap();
        let mut sequence_tables = Vec::new();
        for _ in 0..sequence_count {
            let glyph_count = reader.read_u16_be().unwrap();
            let mut substitute_glyph_ids = Vec::new();
            for _ in 0..glyph_count {
                substitute_glyph_ids.push(reader.read_u16_be().unwrap());
            }
            sequence_tables.push(SequenceTable {
                glyph_count,
                substitute_glyph_ids,
            });
        }
        LookupSubstitution::Multiple(MultipleSubstitutionFormat1 {
            subst_format,
            coverage_offset,
            sequence_count,
            sequence_tables,
        })
    }

    fn get_alternate<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let coverage_offset = reader.read_u16_be().unwrap();
        let alternate_set_count = reader.read_u16_be().unwrap();
        let mut alternate_set = Vec::new();
        for _ in 0..alternate_set_count {
            let glyph_count = reader.read_u16_be().unwrap();
            let mut alternate_glyph_ids = Vec::new();
            for _ in 0..glyph_count {
                alternate_glyph_ids.push(reader.read_u16_be().unwrap());
            }
            alternate_set.push(AlternateSet {
                glyph_count,
                alternate_glyph_ids,
            });
        }
        LookupSubstitution::Alternate(AlternateSubstitutionFormat1 {
            subst_format,
            coverage_offset,
            alternate_set_count,
            alternate_set,
        })
    }

    fn get_ligature<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let coverage_offset = reader.read_u16_be().unwrap();
        let ligature_set_count = reader.read_u16_be().unwrap();
        let mut ligature_offset = Vec::new();
        for _ in 0..ligature_set_count {
            ligature_offset.push(reader.read_u16_be().unwrap());
        }

        let mut ligature_set = Vec::new();
        for ligature_offset in ligature_offset.iter() {
            let offset = *ligature_offset as u64 + offset;
            reader.seek(SeekFrom::Start(offset as u64)).unwrap();
            let ligature_count = reader.read_u16_be().unwrap();
            let mut lingature_offset = Vec::new();
            for _ in 0..ligature_count {
                lingature_offset.push(reader.read_u16_be().unwrap());
            }       

            let mut ligature_table = Vec::new();           
            for lingature_offset in lingature_offset.iter() {
                let offset = *lingature_offset as u64 + offset;
                reader.seek(SeekFrom::Start(offset as u64)).unwrap();
                let ligature_glyph = reader.read_u16_be().unwrap();
                let component_count = reader.read_u16_be().unwrap();
                let mut component_glyph_ids = Vec::new();
                for _ in 0..component_count {
                    component_glyph_ids.push(reader.read_u16_be().unwrap());
                }
                ligature_table.push(LigatureTable {
                    ligature_glyph,
                    component_count,
                    component_glyph_ids,
                });
            }
            ligature_set.push(LigatureSet {
                ligature_count,
                ligature_table,
            });
        }
        let offset = offset + coverage_offset as u64;
        let coverage = Self::get_coverage(reader, offset);
        LookupSubstitution::Ligature(LigatureSubstitutionFormat1 {
            subst_format,
            coverage,
            ligature_set_count,
            ligature_set,
        })
    }

    fn get_context<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let coverage_offset = reader.read_u16_be().unwrap();
        let rule_set_count = reader.read_u16_be().unwrap();
        let mut rule_set = Vec::new();
        for _ in 0..rule_set_count {
            let rule_count = reader.read_u16_be().unwrap();
            let mut rule = Vec::new();
            for _ in 0..rule_count {
                let glyph_count = reader.read_u16_be().unwrap();
                let mut input_glyph_ids = Vec::new();
                for _ in 0..glyph_count {
                    input_glyph_ids.push(reader.read_u16_be().unwrap());
                }
                let lookup_count = reader.read_u16_be().unwrap();
                let mut lookup_indexes = Vec::new();
                for _ in 0..lookup_count {
                    lookup_indexes.push(reader.read_u16_be().unwrap());
                }
                rule.push(Rule {
                    glyph_count,
                    input_glyph_ids,
                    lookup_count,
                    lookup_indexes,
                });
            }
            rule_set.push(RuleSet { rule_count, rule });
        }
        let offset = offset + coverage_offset as u64;
        let coverage = Self::get_coverage(reader, offset);
        LookupSubstitution::ContextSubstitution(ContextSubstitutionFormat1 {
            subst_format,
            coverage,
            rule_set_count,
            rule_set,
        })
    }

    fn get_chaining_context<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        match subst_format {
            1 => {
                let coverage_offset = reader.read_u16_be().unwrap();
                let chain_sub_rule_set_count = reader.read_u16_be().unwrap();
                let mut chain_sub_rule_set_offsets = Vec::new();
                for _ in 0..chain_sub_rule_set_count {
                    chain_sub_rule_set_offsets.push(reader.read_u16_be().unwrap());
                }
                let mut chain_sub_rule_set = Vec::new();
                for chain_sub_rule_set_offset in chain_sub_rule_set_offsets.iter() {
                    let offset = *chain_sub_rule_set_offset as u64 + offset;
                    reader.seek(SeekFrom::Start(offset as u64)).unwrap();
                    let chain_sub_rule_count = reader.read_u16_be().unwrap();
                    let mut chain_sub_rule_offsets = Vec::new();
                    for _ in 0..chain_sub_rule_count {
                        chain_sub_rule_offsets.push(reader.read_u16_be().unwrap());
                    }
                    let mut chain_sub_rule = Vec::new();
                    for chain_sub_rule_offset in chain_sub_rule_offsets.iter() {
                        let offset = *chain_sub_rule_offset as u64 + offset;
                        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
                        let backtrack_glyph_count = reader.read_u16_be().unwrap();
                        let mut backtrack_glyph_ids = Vec::new();
                        for _ in 0..backtrack_glyph_count {
                            backtrack_glyph_ids.push(reader.read_u16_be().unwrap());
                        }
                        let input_glyph_count = reader.read_u16_be().unwrap();
                        let mut input_glyph_ids = Vec::new();
                        for _ in 0..input_glyph_count {
                            input_glyph_ids.push(reader.read_u16_be().unwrap());
                        }
                        let lookahead_glyph_count = reader.read_u16_be().unwrap();
                        let mut lookahead_glyph_ids = Vec::new();
                        for _ in 0..lookahead_glyph_count {
                            lookahead_glyph_ids.push(reader.read_u16_be().unwrap());
                        }
                        let lookup_count = reader.read_u16_be().unwrap();
                        let mut lookup_indexes = Vec::new();
                        for _ in 0..lookup_count {
                            lookup_indexes.push(reader.read_u16_be().unwrap());
                        }
                        chain_sub_rule.push(ChainSubRule {
                            backtrack_glyph_count,
                            backtrack_glyph_ids,
                            input_glyph_count,
                            input_glyph_ids,
                            lookahead_glyph_count,
                            lookahead_glyph_ids,
                            lookup_count,
                            lookup_indexes,
                        });
                    }
                    chain_sub_rule_set.push(ChainSubRuleSet {
                        chain_sub_rule_count,
                        chain_sub_rule,
                    });
                }
                let offset = offset + coverage_offset as u64;
                let coverage = Self::get_coverage(reader, offset);                
                LookupSubstitution::ChainingContextSubstitution(ChainingContextSubstitutionFormat1 {
                    subst_format,
                    coverage,
                    chain_sub_rule_set_count,
                    chain_sub_rule_set,
                })
            }
            2 => {
                let coverage_offset = reader.read_u16_be().unwrap();
                let class_range_count = reader.read_u16_be().unwrap();
                let mut class_range_records = Vec::new();
                for _ in 0..class_range_count {
                    let start_glyph_id = reader.read_u16_be().unwrap();
                    let end_glyph_id = reader.read_u16_be().unwrap();
                    let class = reader.read_u16_be().unwrap();
                    class_range_records.push(ClassRangeRecords {
                        start_glyph_id,
                        end_glyph_id,
                        class,
                    });
                }
                let coverage = Self::get_coverage(reader, offset + coverage_offset as u64);

                LookupSubstitution::ChainingContextSubstitution2(ChainingContextSubstitutionFormat2 {
                    subst_format,
                    class_range_count,
                    class_range_records,
                    coverage,
                })
            }
            3 => {
                let backtrack_glyph_count = reader.read_u16_be().unwrap();
                let mut backtrack_coverage_offsets = Vec::new();
                for _ in 0..backtrack_glyph_count {
                    let coverage_offset = reader.read_u16_be().unwrap();
                    backtrack_coverage_offsets.push(coverage_offset);
                }
                let input_glyph_count = reader.read_u16_be().unwrap();
                let mut input_coverage_offsets = Vec::new();
                for _ in 0..input_glyph_count {
                    let coverage_offset = reader.read_u16_be().unwrap();
                    input_coverage_offsets.push(coverage_offset);
                }
                let lookahead_glyph_count = reader.read_u16_be().unwrap();
                let mut lookahead_coverage_offsets = Vec::new();
                for _ in 0..lookahead_glyph_count {
                    let coverage_offset = reader.read_u16_be().unwrap();
                    lookahead_coverage_offsets.push(coverage_offset);
                }
                let seq_lookup_count = reader.read_u16_be().unwrap();
                let mut lookup_records =  Vec::new();
                for _ in 0..seq_lookup_count {
                    let sequence_index = reader.read_u16_be().unwrap();
                    let lookup_list_index = reader.read_u16_be().unwrap();
                    lookup_records.push(LookupRecord {
                        sequence_index,
                        lookup_list_index,
                    });
                }
                let seq_lookup_records = SequenceLookupRecords {
                    lookup_records
                };

                let mut backtrack_coverages = Vec::new();
                for coverage_offset in backtrack_coverage_offsets.iter() {
                    let coverage = Self::get_coverage(reader, offset + *coverage_offset as u64);
                    backtrack_coverages.push(coverage);
                }

                let mut input_coverages = Vec::new();
                for coverage_offset in input_coverage_offsets.iter() {
                    let coverage = Self::get_coverage(reader, offset + *coverage_offset as u64);
                    input_coverages.push(coverage);
                }

                let mut lookahead_coverages = Vec::new();
                for coverage_offset in lookahead_coverage_offsets.iter() {
                    let coverage = Self::get_coverage(reader, offset + *coverage_offset as u64);
                    lookahead_coverages.push(coverage);
                }


                LookupSubstitution::ChainingContextSubstitution3(ChainingContextSubstitutionFormat3{
                    format: subst_format,
                    backtrack_glyph_count,
                    backtrack_coverages,
                    input_glyph_count,
                    input_coverages,
                    lookahead_glyph_count,
                    lookahead_coverages,
                    seq_lookup_count,
                    seq_lookup_records,
                })
            } 
            _ => {
                LookupSubstitution::Unknown
            }
        }
    }

    fn get_coverage<R: BinaryReader>(reader: &mut R, offset: u64) -> Coverage {
            reader.seek(SeekFrom::Start(offset as u64)).unwrap();
            let coverage_format = reader.read_u16_be().unwrap();
            match coverage_format {
                1 => {
                    let glyph_count = reader.read_u16_be().unwrap();
                    let mut glyph_ids = Vec::new();
                    for _ in 0..glyph_count {
                        glyph_ids.push(reader.read_u16_be().unwrap());
                    }
                    Coverage::Format1(CoverageFormat1 {
                        coverage_format,
                        glyph_count,
                        glyph_ids,
                    })
                }
                2 => {
                    let range_count = reader.read_u16_be().unwrap();
                    let mut range_records = Vec::new();
                    for _ in 0..range_count {
                        let start_glyph_id = reader.read_u16_be().unwrap();
                        let end_glyph_id = reader.read_u16_be().unwrap();
                        let start_coverage_index = reader.read_u16_be().unwrap();
                        range_records.push(RangeRecord {
                            start_glyph_id,
                            end_glyph_id,
                            start_coverage_index,
                        });
                    }
                    Coverage::Format2(CoverageFormat2 {
                        coverage_format,
                        range_count,
                        range_records,
                    })
                }
                _ => {
                    panic!("Unknown coverage format: {}", coverage_format);
                }
            }
        }


    fn get_extension<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let _ = reader.read_u16_be(); // 1
        let extension_lookup_type = reader.read_u16_be().unwrap();
        let extension_offset = reader.read_u32_be().unwrap();


        let offset = offset + extension_offset as u64;
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let lookup_type = num_traits::FromPrimitive::from_u16(extension_lookup_type).unwrap();
        let subtable = match lookup_type {
            LookupType::SingleSubstitution => Self::get_single(reader, offset),
            LookupType::MultipleSubstitution => Self::get_multiple(reader, offset),
            LookupType::AlternateSubstitution => Self::get_alternate(reader, offset),
            LookupType::LigatureSubstitution => Self::get_ligature(reader, offset),
            LookupType::ContextSubstitution => Self::get_context(reader, offset),
            LookupType::ChainingContextSubstitution => Self::get_chaining_context(reader, offset),
            // LookupType::ExtensionSubstitution => Self::get_extension(reader, offset), // not 7
            LookupType::ReverseChainingContextualSingleSubstitution => {
                Self::get_reverse_chaining_context(reader, offset)
            }
            _ => {
                panic!("Unknown lookup type: {:?}", lookup_type);
            }
        };
        subtable
    }

    fn get_reverse_chaining_context<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let coverage_offset = reader.read_u16_be().unwrap();
        let backtrack_glyph_count = reader.read_u16_be().unwrap();
        let mut backtrack_glyph_ids = Vec::new();
        for _ in 0..backtrack_glyph_count {
            backtrack_glyph_ids.push(reader.read_u16_be().unwrap());
        }
        let input_glyph_count = reader.read_u16_be().unwrap();
        let mut input_glyph_ids = Vec::new();
        for _ in 0..input_glyph_count {
            input_glyph_ids.push(reader.read_u16_be().unwrap());
        }
        let lookahead_glyph_count = reader.read_u16_be().unwrap();
        let mut lookahead_glyph_ids = Vec::new();
        for _ in 0..lookahead_glyph_count {
            lookahead_glyph_ids.push(reader.read_u16_be().unwrap());
        }
        let substitute_glyph_id = reader.read_u16_be().unwrap();
        LookupSubstitution::ReverseChainSingle(ReverseChainSingleSubstitutionFormat1 {
            subst_format,
            coverage_offset,
            backtrack_glyph_count,
            backtrack_glyph_ids,
            input_glyph_count,
            input_glyph_ids,
            lookahead_glyph_count,
            lookahead_glyph_ids,
            substitute_glyph_id,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum LookupSubstitution {
    // Lookup Type 1: Single Substitution Subtable
    Single(SingleSubstitutionFormat1),
    Single2(SingleSubstitutionFormat2),
    // Lookup Type 2: Multiple Substitution Subtable
    Multiple(MultipleSubstitutionFormat1),
    // Lookup Type 3: Alternate Substitution Subtable
    Alternate(AlternateSubstitutionFormat1),
    // Lookup Type 4: Ligature Substitution Subtable
    Ligature(LigatureSubstitutionFormat1),
    // Lookup Type 5: Contextual Substitution Subtable
    // 5.1
    ContextSubstitution(ContextSubstitutionFormat1),
    // 5.2
    // 5.3
    // Lookup Type 6: Chaining Contextual Substitution Subtable
    ChainingContextSubstitution(ChainingContextSubstitutionFormat1),
    ChainingContextSubstitution2(ChainingContextSubstitutionFormat2),
    ChainingContextSubstitution3(ChainingContextSubstitutionFormat3),
    // Lookup Type 7: Extension Substitution Subtable
    ExtensionSubstitution(ExtensionSubstitutionFormat1),
    // Lookup Type 8: Reverse Chaining Contextual Single Substitution Subtable
    ReverseChainSingle(ReverseChainSingleSubstitutionFormat1),
    Unknown,
}

impl LookupSubstitution {
    pub(crate) fn to_string(&self) -> String {
        format!("{:?}", self)
    }

}


#[derive(Debug, Clone)]
pub(crate) struct SingleSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage: Coverage,
    pub(crate) delta_glyph_id: i16,
}

#[derive(Debug, Clone)]
pub(crate) struct SingleSubstitutionFormat2 {
    pub(crate) subst_format: u16,
    pub(crate) coverage: Coverage,
    pub(crate) glyph_count: u16,
    pub(crate) substitute_glyph_ids: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct MultipleSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) sequence_count: u16,
    pub(crate) sequence_tables: Vec<SequenceTable>,
}

#[derive(Debug, Clone)]
pub(crate) struct SequenceTable {
    pub(crate) glyph_count: u16,
    pub(crate) substitute_glyph_ids: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct AlternateSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) alternate_set_count: u16,
    pub(crate) alternate_set: Vec<AlternateSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct AlternateSet {
    pub(crate) glyph_count: u16,
    pub(crate) alternate_glyph_ids: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct LigatureSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage: Coverage,
    pub(crate) ligature_set_count: u16,
    pub(crate) ligature_set: Vec<LigatureSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct LigatureSet {
    pub(crate) ligature_count: u16,
    pub(crate) ligature_table: Vec<LigatureTable>,
}

#[derive(Debug, Clone)]
pub(crate) struct LigatureTable {
    pub(crate) ligature_glyph: u16,
    pub(crate) component_count: u16,
    pub(crate) component_glyph_ids: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContextSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage: Coverage,
    pub(crate) rule_set_count: u16,
    pub(crate) rule_set: Vec<RuleSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuleSet {
    pub(crate) rule_count: u16,
    pub(crate) rule: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub(crate) struct Rule {
    pub(crate) glyph_count: u16,
    pub(crate) input_glyph_ids: Vec<u16>,
    pub(crate) lookup_count: u16,
    pub(crate) lookup_indexes: Vec<u16>,
}

// Lookup Type 6: Chaining Contextual Substitution Subtable Format 1
#[derive(Debug, Clone)]
pub(crate) struct ChainingContextSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage: Coverage,
    pub(crate) chain_sub_rule_set_count: u16,
    pub(crate) chain_sub_rule_set: Vec<ChainSubRuleSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct ChainSubRuleSet {
    pub(crate) chain_sub_rule_count: u16,
    pub(crate) chain_sub_rule: Vec<ChainSubRule>,
}

#[derive(Debug, Clone)]
pub(crate) struct ChainSubRule {
    pub(crate) backtrack_glyph_count: u16,
    pub(crate) backtrack_glyph_ids: Vec<u16>,
    pub(crate) input_glyph_count: u16,
    pub(crate) input_glyph_ids: Vec<u16>,
    pub(crate) lookahead_glyph_count: u16,
    pub(crate) lookahead_glyph_ids: Vec<u16>,
    pub(crate) lookup_count: u16,
    pub(crate) lookup_indexes: Vec<u16>,
}

// Lookup Type 6: Chaining Contextual Substitution Subtable Format 2
#[derive(Debug, Clone)]
pub(crate) struct ChainingContextSubstitutionFormat2 {
    pub(crate) subst_format: u16, // 2
    pub(crate) class_range_count: u16,
    pub(crate) class_range_records: Vec<ClassRangeRecords>,
    pub(crate) coverage: Coverage,
}

#[derive(Debug, Clone)]
/* */
pub(crate) struct ClassRangeRecords {
    pub(crate) start_glyph_id: u16,
    pub(crate) end_glyph_id: u16,
    pub(crate) class: u16,
}

// Lookup Type 6: Chaining Contextual Substitution Subtable Format 3
#[derive(Debug, Clone)]
pub(crate) struct ChainingContextSubstitutionFormat3 {
    pub(crate) format: u16, // 3
    pub(crate) backtrack_glyph_count: u16,
    pub(crate) backtrack_coverages: Vec<Coverage>,
    pub(crate) input_glyph_count: u16,
    pub(crate) input_coverages: Vec<Coverage>,
    pub(crate) lookahead_glyph_count: u16,
    pub(crate) lookahead_coverages: Vec<Coverage>,
    pub(crate) seq_lookup_count: u16,
    pub(crate) seq_lookup_records: SequenceLookupRecords,
}

#[derive(Debug, Clone)]

pub(crate) enum Coverage {
    Format1(CoverageFormat1),
    Format2(CoverageFormat2),
}

#[derive(Debug, Clone)]
pub(crate) struct CoverageFormat1 {
    pub(crate) coverage_format: u16,
    pub(crate) glyph_count: u16,
    pub(crate) glyph_ids: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct CoverageFormat2 {
    pub(crate) coverage_format: u16,
    pub(crate) range_count: u16,
    pub(crate) range_records: Vec<RangeRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct RangeRecord {
    pub(crate) start_glyph_id: u16,
    pub(crate) end_glyph_id: u16,
    pub(crate) start_coverage_index: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct SequenceLookupRecords {
    pub(crate) lookup_records: Vec<LookupRecord>,
}

#[derive(Debug, Clone)]
pub struct LookupRecord {
    pub(crate) sequence_index: u16,
    pub(crate) lookup_list_index: u16,
}



// Lookup Type 7: Extension Substitution Subtable
#[derive(Debug, Clone)]
pub(crate) struct ExtensionSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) extension_lookup_type: u16,
    pub(crate) extension_offset: u32,
}

// Lookup Type 8: Reverse Chaining Contextual Single Substitution Subtable
#[derive(Debug, Clone)]
pub(crate) struct ReverseChainSingleSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) backtrack_glyph_count: u16,
    pub(crate) backtrack_glyph_ids: Vec<u16>,
    pub(crate) input_glyph_count: u16,
    pub(crate) input_glyph_ids: Vec<u16>,
    pub(crate) lookahead_glyph_count: u16,
    pub(crate) lookahead_glyph_ids: Vec<u16>,
    pub(crate) substitute_glyph_id: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct Feature {
    pub(crate) feature_tag: u32,
    feature_offset: u16,
    pub(crate) feature_params: Option<FeatureParams>,
    pub(crate) lookup_list_indices: Vec<u16>,
}

impl Feature {
    pub(crate) fn to_string(&self) -> String {
        let mut bytes = [0;4];
        for i in 0..4 {
            bytes[3 - i] = (self.feature_tag >> (i * 8)) as u8;
        }
        let tag = std::str::from_utf8(&bytes).unwrap();
        let mut string = format!("FeatureTag: {}\n", tag);
        string += &format!("FeatureParams: {:?}\n", self.feature_params);
        string += &format!("LookupListIndices: {:?}\n", self.lookup_list_indices);
        string
    }


}

#[derive(Debug, Clone)]
pub(crate) struct FeatureParams {
    pub(crate) feature_params: u16,
}



#[derive(Debug, Clone)]
pub(crate) struct FeatureList {
    pub(crate) feature_count: u16,
    pub(crate) features: Box<Vec<Feature>>,
}

impl FeatureList {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64, length: u32) -> FeatureList {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let feature_count = reader.read_u16_be().unwrap();
        let mut features = Vec::new();
        for _ in 0..feature_count {
            let feature_tag = reader.read_u32_be().unwrap();
            let feature_offset = reader.read_u16_be().unwrap();
            features.push(Feature {
                feature_tag,
                feature_offset,
                feature_params: None,
                lookup_list_indices: Vec::new(),
            });
        }
        for feature in features.iter_mut() {
            let offset = offset + feature.feature_offset as u64;
            reader.seek(SeekFrom::Start(offset as u64)).unwrap();
            let lookup_count = reader.read_u16_be().unwrap();
            for _ in 0..lookup_count {
                feature.lookup_list_indices.push(reader.read_u16_be().unwrap());
            }
        }

        Self {
            feature_count,
            features: Box::new(features),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = format!("FeatureCount: {}\n", self.feature_count);
        for feature in self.features.iter() {
            string += &format!("{}\n", feature.to_string());
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariation {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) feature_variations: Box<Vec<FeatureVariationRecord>>,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariationRecord {
    pub(crate) condition_set_offset: u16,
    pub(crate) feature_table_substitution_offset: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariations {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) condition_set_count: u16,
    pub(crate) condition_sets: Box<Vec<ConditionSet>>,
    pub(crate) feature_table_substitutions: Box<Vec<FeatureTableSubstitution>>,
}
impl FeatureVariations {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64, length: u32) -> Self {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let major_version = reader.read_u16_be().unwrap();
        let minor_version = reader.read_u16_be().unwrap();
        let feature_table_substitution_count = reader.read_u16_be().unwrap();
        let mut feature_table_substitutions = Vec::new();
        for _ in 0..feature_table_substitution_count {
            let feature_table_substitution_offset = reader.read_u16_be().unwrap();
            feature_table_substitutions.push(FeatureTableSubstitution {
                feature_table_substitution: feature_table_substitution_offset,
            });
        }
        let condition_set_count = reader.read_u16_be().unwrap();
        let mut condition_sets = Vec::new();
        for _ in 0..condition_set_count {
            let condition_count = reader.read_u16_be().unwrap();
            let mut conditions = Vec::new();
            for _ in 0..condition_count {
                let format = reader.read_u16_be().unwrap();
                let axis_index = reader.read_u16_be().unwrap();
                let filter_range_min_value = reader.read_f32_be().unwrap();
                let filter_range_max_value = reader.read_f32_be().unwrap();
                conditions.push(ConditionTable {
                    format,
                    axis_index,
                    filter_range_min_value,
                    filter_range_max_value,
                });
            }
            condition_sets.push(ConditionSet {
                condition_count,
                conditions: Box::new(conditions),
            });
        }
        Self {
            major_version,
            minor_version,
            condition_set_count,
            condition_sets: Box::new(condition_sets),
            feature_table_substitutions: Box::new(feature_table_substitutions),
        }

    }
}

#[derive(Debug, Clone)]
pub(crate) struct ConditionSet {
    pub(crate) condition_count: u16,
    pub(crate) conditions: Box<Vec<ConditionTable>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConditionTable {
    pub(crate) format: u16,
    pub(crate) axis_index: u16,
    pub(crate) filter_range_min_value: f32,
    pub(crate) filter_range_max_value: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureTableSubstitution {
    pub(crate) feature_table_substitution: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct Script {
    pub(crate) script_tag: u32, // https://learn.microsoft.com/ja-jp/typography/opentype/spec/scripttags
    pub(crate) script_offset: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedScript {
    pub(crate) script_tag: u32,
    pub(crate) default_language_system_offset: u16,
    pub(crate) language_systems: Box<Vec<LanguageSystemRecord>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LanguageSystemRecord {
    pub(crate) language_system_tag: u32,
    pub(crate) language_system: LanguageSystem,
}

impl LanguageSystemRecord {
    pub(crate) fn to_string(&self) -> String {
        let mut string = format!("LanguageSystem: {}\n", self.language_system_tag);
        string += &format!("{}\n", self.language_system.to_string());
        string
        
    }


}

impl ParsedScript {
    pub(crate) fn parse<R: BinaryReader>(reader: &mut R, script: &Script) -> Self {
        let offset = script.script_offset;
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let default_language_system_offset = reader.read_u16_be().unwrap();
        let language_system_count = reader.read_u16_be().unwrap();
        let mut language_systems = Vec::new();
        for _ in 0..language_system_count {
            let language_system_tag = reader.read_u32_be().unwrap();
            let language_system_offset = reader.read_u16_be().unwrap(); // todo!
            let lookup_order_offset = reader.read_u16_be().unwrap();
            let required_feature_index = reader.read_u16_be().unwrap();
            let feature_index_count = reader.read_u16_be().unwrap();
            let mut feature_indexes = Vec::new();
            for _ in 0..feature_index_count {
                feature_indexes.push(reader.read_u16_be().unwrap());
            }
            language_systems.push(LanguageSystemRecord {
                language_system_tag,
                language_system: LanguageSystem {
                    lookup_order_offset,
                    required_feature_index,
                    feature_index_count,
                    feature_indexes,
                },
            });
        }


        Self {
            script_tag: script.script_tag,
            default_language_system_offset,
            language_systems: Box::new(language_systems),
        }

    }

    pub(crate) fn to_string(&self) -> String {
        let mut u8s = [0;4];
        for i in 0..4 {
            u8s[3 - i] = (self.script_tag >> (i * 8)) as u8;
        }
        let tag = unsafe { std::str::from_utf8_unchecked(&u8s) };
        let mut string = format!("Script: {} {}\n", tag, self.language_systems.len());
        for language_system in self.language_systems.iter() {
            string += &format!("{}", language_system.to_string());
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptList {
    pub(crate) script_count: u16,
    pub(crate) scripts: Box<Vec<ParsedScript>>,
}

impl ScriptList {
    fn new<R: BinaryReader>(reader: &mut R, script_list_offset: u64, length: u32) -> ScriptList {
        reader.seek(SeekFrom::Start(script_list_offset as u64)).unwrap();
        let script_count = reader.read_u16_be().unwrap();
        let mut scripts = Vec::new();
        for _ in 0..script_count {
            let script_tag = reader.read_u32_be().unwrap();
            let script_offset = reader.read_u16_be().unwrap();
            scripts.push(Script {
                script_tag,
                script_offset: script_offset as u64 + script_list_offset,
            });
        }
        let mut parced_scripts = Vec::new();

        for script in scripts.iter_mut() {
            let parsed_script = ParsedScript::parse(reader, script);
            parced_scripts.push(parsed_script)
        }

        Self {
            script_count,
            scripts: Box::new(parced_scripts),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = String::new();
        string += &format!("script count {} :{}\n", self.script_count, self.scripts.len());
        for script in self.scripts.iter() {
            string += &format!("{}",script.to_string());
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptRecord {
    pub(crate) script_tag: u32,
    pub(crate) script: ParsedScript,
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptTable {
    pub(crate) default_language_system_offset: u16,
    pub(crate) language_system_count: u16,
    pub(crate) language_systems: Box<Vec<LanguageSystem>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LanguageSystem {
    pub(crate) lookup_order_offset: u16,
    pub(crate) required_feature_index: u16,
    pub(crate) feature_index_count: u16,
    pub(crate) feature_indexes: Vec<u16>,
}
impl LanguageSystem {
    pub(crate) fn to_string(&self) -> String {
        let mut string = format!("LanguageSystem: {}\n", self.lookup_order_offset);
        string += &format!("{}\n", self.required_feature_index);
        string += &format!("{}\n", self.feature_index_count);
        for feature_index in self.feature_indexes.iter() {
            string += &format!("{}\n", feature_index);
        }
        string
    }


}


#[derive(Debug, Clone)]
pub(crate) struct LanguageSystemTable {
    pub(crate) lookup_order: u16,
    pub(crate) required_feature_index: u16,
    pub(crate) feature_index_count: u16,
    pub(crate) feature_indexes: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariationRecordList {
    pub(crate) feature_variation_record_count: u16,
    pub(crate) feature_variation_records: Box<Vec<FeatureVariationRecord>>,
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureVariationList {
    pub(crate) feature_variation_count: u16,
    pub(crate) feature_variations: Vec<FeatureVariation>,
}
impl FeatureVariationList {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64, length: u32) -> Self {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let feature_variation_count = reader.read_u16_be().unwrap();
        let mut feature_variations = Vec::new();
        for _ in 0..feature_variation_count {
            let major_version = reader.read_u16_be().unwrap();
            let minor_version = reader.read_u16_be().unwrap();
            let feature_variation_record_count = reader.read_u16_be().unwrap();
            let mut feature_variation_records = Vec::new();
            for _ in 0..feature_variation_record_count {
                let condition_set_offset = reader.read_u16_be().unwrap();
                let feature_table_substitution_offset = reader.read_u16_be().unwrap();
                feature_variation_records.push(FeatureVariationRecord {
                    condition_set_offset,
                    feature_table_substitution_offset,
                });
            }
            feature_variations.push(FeatureVariation {
                major_version,
                minor_version,
                feature_variations: Box::new(feature_variation_records),
            });
        }
        Self {
            feature_variation_count,
            feature_variations,
        }
    }
}
