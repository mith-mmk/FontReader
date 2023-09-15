use std::io::SeekFrom;
use super::*;
use bin_rs::reader::BinaryReader;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits;


#[derive(Debug, Clone)]

pub(crate) struct LookupList {
    pub(crate) lookups: Box<Vec<Lookup>>,
}

impl LookupList {
    pub(crate) fn to_string(&self) -> String {
        let mut string = format!("LookupCount: {}\n", self.lookups.len());
        for (i,lookup) in self.lookups.iter().enumerate() {
            string += &format!("Lookup[{}]\n{}\n", i, lookup.to_string());
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
        for lookup_offset in lookup_offsets.iter() {
            let offset = offset + *lookup_offset as u64;
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
        let coverage = Self::get_coverage(reader, offset + coverage_offset as u64);
        LookupSubstitution::Multiple(MultipleSubstitutionFormat1 {
            subst_format,
            coverage,
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
        let coverage = Self::get_coverage(reader, offset + coverage_offset as u64);
        LookupSubstitution::Alternate(AlternateSubstitutionFormat1 {
            subst_format,
            coverage,
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
                    class_range_records.push(ClassRangeRecord {
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

pub(crate) enum LookupResult {
    Single(u16),
    Multiple(Vec<u16>),
    Ligature(Vec<LigatureTable>),
    Context(Vec<Rule>),
    Chaining(Vec<ChainSubRule>),
    Chaing63(Vec<ChainSubRule>),
    None,
}

impl LookupSubstitution {
    pub(crate) fn to_string(&self) -> String {
        format!("{:?}", self)
    }

    pub(crate) fn get_coverage(&self) -> (&Coverage, Option<(Vec<Coverage>,Vec<Coverage>, Vec<Coverage>)>) {
        let coverage = match self {
            Self::Single(single) => &single.coverage,
            Self::Single2(single2) => &single2.coverage,
            Self::Multiple(multiple) => &multiple.coverage,
            Self::Alternate(alternate) => &alternate.coverage,
            Self::Ligature(ligature) => &ligature.coverage,
            Self::ContextSubstitution(context) => &context.coverage,
            Self::ChainingContextSubstitution(chaining) => &chaining.coverage,
            Self::ChainingContextSubstitution2(chaining2) => &chaining2.coverage,
            Self::ChainingContextSubstitution3(chaining3) => &chaining3.backtrack_coverages[0],
            _ => {
                panic!("Unknown lookup type: {:?}", self);
            }
        };

        let mut coverages = None;
        if let Self::ChainingContextSubstitution3(chaining3) = self {
            let coverages1 = chaining3.backtrack_coverages.clone();
            let coverages2 = chaining3.input_coverages.clone();
            let coverages3 = chaining3.lookahead_coverages.clone();
            coverages = Some((coverages1, coverages2, coverages3));
        }
        (coverage, coverages)
    }

    pub(crate) fn get_lookup(&self, gliph_id: usize) -> LookupResult {
        match self {
            Self::Single(single) => {
                let coverage = &single.coverage;
                let id = coverage.contains(gliph_id);
                if let Some(id) = id {
                    let return_gliph = (single.delta_glyph_id as i32 + id as i32) & 0xFFFF;
                    LookupResult::Single(return_gliph as u16)
                } else {
                    LookupResult::None
                }
            }
            Self::Single2(single) => {
                let coverage = &single.coverage;
                let id = coverage.contains(gliph_id);
                if let Some(id) = id {
                    let return_gliph = single.substitute_glyph_ids[id];
                    LookupResult::Single(return_gliph)
                } else {
                    LookupResult::None
                }
            }
            Self::Multiple(multiple) => {
                let coverage = &multiple.coverage;
                let id = coverage.contains(gliph_id);
                if let Some(id) = id {
                    let sequence_table = &multiple.sequence_tables[id];
                    let result = sequence_table.substitute_glyph_ids.clone();
                    LookupResult::Multiple(result)
                } else {
                    LookupResult::None
                }
            }
            Self::Alternate(alternate) => {
                let coverage = &alternate.coverage;
                let id = coverage.contains(gliph_id);
                if let Some(id) = id {
                    let alternate_set = &alternate.alternate_set[id];
                    let result = alternate_set.alternate_glyph_ids.clone();
                    LookupResult::Multiple(result)
                } else {
                    LookupResult::None
                }
            }
            Self::Ligature(ligature) => {
                let coverage = &ligature.coverage;
                let id = coverage.contains(gliph_id);
                if let Some(id) = id {
                    let ligature_set = &ligature.ligature_set[id];
                    let result = ligature_set.ligature_table.clone();
                    LookupResult::Ligature(result)
                } else {
                    LookupResult::None
                }
            }
            Self::ContextSubstitution(context) => {
                let coverage = &context.coverage;
                let id = coverage.contains(gliph_id);
                if let Some(id) = id {
                    let rule_set = &context.rule_set[id];
                    let result = rule_set.rule.clone();
                    LookupResult::Context(result)
                } else {
                    LookupResult::None
                }
            }
            Self::ChainingContextSubstitution(chaining) => {
                let coverage = &chaining.coverage;
                let id = coverage.contains(gliph_id);
                if let Some(id) = id {
                    let chain_sub_rule_set = &chaining.chain_sub_rule_set[id];
                    let result = chain_sub_rule_set.chain_sub_rule.clone();
                    LookupResult::Chaining(result)

                } else {
                    LookupResult::None
                }
            }
            Self::ChainingContextSubstitution2(chaining2) => {
                let coverage = &chaining2.coverage;
                let id = coverage.contains(gliph_id);
                if let Some(id) = id {
                    let class_range_record = &chaining2.class_range_records[id];
                    let class = class_range_record.class;
                    let result = vec![class];
                    LookupResult::Multiple(result)
                } else {
                    LookupResult::None
                }
            }
            Self::ChainingContextSubstitution3(chaining3) => {
                todo!() // これ実装したやつ頭おかしいだろ

            }
            Self::ExtensionSubstitution(_) => {
                panic!() // not 7
            }
            Self::ReverseChainSingle(_) => {
                panic!()
            }

            _ => {
                panic!("Unknown lookup type: {:?}", self);
            }
        }
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
    pub(crate) coverage: Coverage,
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
    pub(crate) coverage: Coverage,
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
    pub(crate) class_range_records: Vec<ClassRangeRecord>,
    pub(crate) coverage: Coverage,
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

