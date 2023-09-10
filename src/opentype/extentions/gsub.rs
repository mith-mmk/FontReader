// GSUB -- Glyph Substitution Table

use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits;

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
            Some(Box::new(FeatureVariations::new(
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
}

pub(crate) struct LookupList {
    pub(crate) lookup_count: u16,
    pub(crate) lookups: Box<Vec<Lookup>>,
}

pub(crate) struct Lookup {
    pub(crate) lookup_type: u16,
    pub(crate) lookup_flag: u16,
    pub(crate) subtable_count: u16,
    pub(crate) subtables: Vec<LookupSubstitution>,
}

pub(crate) struct LookupRaw {
    pub(crate) lookup_type: u16,
    pub(crate) lookup_flag: u16,
    pub(crate) subtable_count: u16,
    pub(crate) subtable_offsets: Vec<u32>,
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
        let mut lookups = Vec::new();
        for _ in 0..lookup_count {
            let lookup_type = reader.read_u16_be().unwrap();
            let lookup_flag = reader.read_u16_be().unwrap();
            let subtable_count = reader.read_u16_be().unwrap();
            let mut subtable_offsets = Vec::new();
            for _ in 0..subtable_count {
                subtable_offsets.push(reader.read_u32_be().unwrap());
            }

            lookups.push(LookupRaw {
                lookup_type,
                lookup_flag,
                subtable_count,
                subtable_offsets,
            });
        }
        let mut lookups_parsed = Vec::new();
        for lookup in lookups.iter_mut() {
            let mut subtables = Vec::new();
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
                subtable_count: lookup.subtable_count,
                subtables: subtables,
            });
        }

        Self {
            lookup_count,
            lookups: Box::new(lookups_parsed),
        }
    }
    fn get_single<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let coverage_offset = reader.read_u16_be().unwrap();
        let delta_glyph_id = reader.read_i16_be().unwrap();
        if subst_format != 2 {
            return LookupSubstitution::Single(SingleSubstitutionFormat1 {
                subst_format,
                coverage_offset,
                delta_glyph_id,
            });
        }
        let glyph_count = reader.read_u16_be().unwrap();
        let mut substitute_glyph_ids = Vec::new();
        for _ in 0..glyph_count {
            substitute_glyph_ids.push(reader.read_u16_be().unwrap());
        }
        LookupSubstitution::Single2(SingleSubstitutionFormat2 {
            subst_format,
            coverage_offset,
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
        let mut ligature_set = Vec::new();
        for _ in 0..ligature_set_count {
            let ligature_count = reader.read_u16_be().unwrap();
            let mut ligature_table = Vec::new();
            for _ in 0..ligature_count {
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
        LookupSubstitution::Ligature(LigatureSubstitutionFormat1 {
            subst_format,
            coverage_offset,
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
        LookupSubstitution::ContextSubstitution(ContextSubstitutionFormat1 {
            subst_format,
            coverage_offset,
            rule_set_count,
            rule_set,
        })
    }

    fn get_chaining_context<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let coverage_offset = reader.read_u16_be().unwrap();
        let chain_sub_rule_set_count = reader.read_u16_be().unwrap();
        let mut chain_sub_rule_set = Vec::new();
        for _ in 0..chain_sub_rule_set_count {
            let chain_sub_rule_count = reader.read_u16_be().unwrap();
            let mut chain_sub_rule = Vec::new();
            for _ in 0..chain_sub_rule_count {
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
        LookupSubstitution::ChainingContextSubstitution(ChainingContextSubstitutionFormat1 {
            subst_format,
            coverage_offset,
            chain_sub_rule_set_count,
            chain_sub_rule_set,
        })
    }

    fn get_extension<R: BinaryReader>(reader: &mut R, offset: u64) -> LookupSubstitution {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let subst_format = reader.read_u16_be().unwrap();
        let extension_lookup_type = reader.read_u16_be().unwrap();
        let extension_offset = reader.read_u32_be().unwrap();
        LookupSubstitution::ExtensionSubstitution(ExtensionSubstitutionFormat1 {
            subst_format,
            extension_lookup_type,
            extension_offset,
        })
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

pub(crate) enum LookupSubstitution {
    // Lookup Type 1: Single Substitution Subtable
    // 1.1
    Single(SingleSubstitutionFormat1),
    // 1.2
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
    // Lookup Type 7: Extension Substitution Subtable
    ExtensionSubstitution(ExtensionSubstitutionFormat1),
    // Lookup Type 8: Reverse Chaining Contextual Single Substitution Subtable
    ReverseChainSingle(ReverseChainSingleSubstitutionFormat1),
}

pub(crate) struct SingleSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) delta_glyph_id: i16,
}

pub(crate) struct SingleSubstitutionFormat2 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) glyph_count: u16,
    pub(crate) substitute_glyph_ids: Vec<u16>,
}

pub(crate) struct MultipleSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) sequence_count: u16,
    pub(crate) sequence_tables: Vec<SequenceTable>,
}

pub(crate) struct SequenceTable {
    pub(crate) glyph_count: u16,
    pub(crate) substitute_glyph_ids: Vec<u16>,
}

pub(crate) struct AlternateSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) alternate_set_count: u16,
    pub(crate) alternate_set: Vec<AlternateSet>,
}

pub(crate) struct AlternateSet {
    pub(crate) glyph_count: u16,
    pub(crate) alternate_glyph_ids: Vec<u16>,
}

pub(crate) struct LigatureSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) ligature_set_count: u16,
    pub(crate) ligature_set: Vec<LigatureSet>,
}

pub(crate) struct LigatureSet {
    pub(crate) ligature_count: u16,
    pub(crate) ligature_table: Vec<LigatureTable>,
}

pub(crate) struct LigatureTable {
    pub(crate) ligature_glyph: u16,
    pub(crate) component_count: u16,
    pub(crate) component_glyph_ids: Vec<u16>,
}

pub(crate) struct ContextSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) rule_set_count: u16,
    pub(crate) rule_set: Vec<RuleSet>,
}

pub(crate) struct RuleSet {
    pub(crate) rule_count: u16,
    pub(crate) rule: Vec<Rule>,
}

pub(crate) struct Rule {
    pub(crate) glyph_count: u16,
    pub(crate) input_glyph_ids: Vec<u16>,
    pub(crate) lookup_count: u16,
    pub(crate) lookup_indexes: Vec<u16>,
}

pub(crate) struct ChainingContextSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) coverage_offset: u16,
    pub(crate) chain_sub_rule_set_count: u16,
    pub(crate) chain_sub_rule_set: Vec<ChainSubRuleSet>,
}

pub(crate) struct ChainSubRuleSet {
    pub(crate) chain_sub_rule_count: u16,
    pub(crate) chain_sub_rule: Vec<ChainSubRule>,
}

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

// Lookup Type 7: Extension Substitution Subtable
pub(crate) struct ExtensionSubstitutionFormat1 {
    pub(crate) subst_format: u16,
    pub(crate) extension_lookup_type: u16,
    pub(crate) extension_offset: u32,
}

// Lookup Type 8: Reverse Chaining Contextual Single Substitution Subtable
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

pub(crate) struct Feature {
    pub(crate) feature_tag: u32,
    pub(crate) feature_offset: u16,
}

pub(crate) struct FeatureList {
    pub(crate) feature_count: u16,
    pub(crate) features: Box<Vec<Feature>>,
}
impl FeatureList {
    fn new<R: BinaryReader>(reader: &mut R, u64: u64, length: u32) -> FeatureList {
        todo!()
    }
}

pub(crate) struct FeatureVariation {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) feature_variations: Box<Vec<FeatureVariationRecord>>,
}

pub(crate) struct FeatureVariationRecord {
    pub(crate) condition_set_offset: u16,
    pub(crate) feature_table_substitution_offset: u16,
}

pub(crate) struct FeatureVariations {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) condition_set_count: u16,
    pub(crate) condition_sets: Box<Vec<ConditionSet>>,
    pub(crate) feature_table_substitution_count: u16,
    pub(crate) feature_table_substitutions: Box<Vec<FeatureTableSubstitution>>,
}
impl FeatureVariations {
    fn new<R: BinaryReader>(reader: &mut R, u64: u64, length: u32) -> FeatureVariationList {
        todo!()
    }
}

pub(crate) struct ConditionSet {
    pub(crate) condition_count: u16,
    pub(crate) conditions: Box<Vec<ConditionTable>>,
}

pub(crate) struct ConditionTable {
    pub(crate) format: u16,
    pub(crate) axis_index: u16,
    pub(crate) filter_range_min_value: f32,
    pub(crate) filter_range_max_value: f32,
}

pub(crate) struct FeatureTableSubstitution {
    pub(crate) feature_table_substitution: u16,
}

pub(crate) struct Script {
    pub(crate) script_tag: u32,
    pub(crate) script_offset: u16,
}

pub(crate) struct ScriptList {
    pub(crate) script_count: u16,
    pub(crate) scripts: Box<Vec<Script>>,
}
impl ScriptList {
    fn new<R: BinaryReader>(reader: &mut R, script_list_offset: u64, length: u32) -> ScriptList {
        todo!()
    }
}

pub(crate) struct ScriptRecord {
    pub(crate) script_tag: u32,
    pub(crate) script_offset: u16,
}

pub(crate) struct ScriptTable {
    pub(crate) default_language_system_offset: u16,
    pub(crate) language_system_count: u16,
    pub(crate) language_systems: Box<Vec<LanguageSystem>>,
}

pub(crate) struct LanguageSystem {
    pub(crate) lookup_order_offset: u16,
    pub(crate) required_feature_index: u16,
    pub(crate) feature_index_count: u16,
    pub(crate) feature_indexes: Vec<u16>,
}

pub(crate) struct LanguageSystemTable {
    pub(crate) lookup_order: u16,
    pub(crate) required_feature_index: u16,
    pub(crate) feature_index_count: u16,
    pub(crate) feature_indexes: Vec<u16>,
}

pub(crate) struct FeatureVariationRecordList {
    pub(crate) feature_variation_record_count: u16,
    pub(crate) feature_variation_records: Box<Vec<FeatureVariationRecord>>,
}

pub(crate) struct FeatureVariationList {
    pub(crate) feature_variation_count: u16,
    pub(crate) feature_variations: Vec<FeatureVariation>,
}
