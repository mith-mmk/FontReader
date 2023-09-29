use super::{classdef::ClassDef, *};
use bin_rs::reader::BinaryReader;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits;
use std::io::SeekFrom;

#[derive(Debug, Clone)]

pub(crate) struct LookupList {
    pub(crate) lookups: Box<Vec<Lookup>>,
}

impl LookupList {
    pub(crate) fn to_string(&self) -> String {
        let mut string = format!("LookupCount: {}\n", self.lookups.len());
        for (i, lookup) in self.lookups.iter().enumerate() {
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

    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        lookup: &LookupRaw,
    ) -> Result<Self, std::io::Error> {
        let mut subtables = Vec::new();
        let offset = lookup.offset;
        for subtable_offset in lookup.subtable_offsets.iter() {
            let offset = offset + *subtable_offset as u64;

            let lookup_type = num_traits::FromPrimitive::from_u16(lookup.lookup_type)
                .unwrap_or(LookupType::Unknown);
            let subtable = match lookup_type {
                LookupType::SingleSubstitution => LookupList::get_single(reader, offset),
                LookupType::MultipleSubstitution => LookupList::get_multiple(reader, offset),
                LookupType::AlternateSubstitution => LookupList::get_alternate(reader, offset),
                LookupType::LigatureSubstitution => LookupList::get_ligature(reader, offset),
                LookupType::ContextSubstitution => LookupList::get_context(reader, offset),
                LookupType::ChainingContextSubstitution => {
                    LookupList::get_chaining_context(reader, offset)
                }
                LookupType::ExtensionSubstitution => LookupList::get_extension(reader, offset),
                LookupType::ReverseChainingContextualSingleSubstitution => {
                    LookupList::get_reverse_chaining_context(reader, offset)
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Unknown lookup type"),
                    ))
                }
            };
            subtables.push(subtable?);
        }
        Ok(Self {
            lookup_type: lookup.lookup_type,
            lookup_flag: lookup.lookup_flag,
            subtables: subtables,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LookupRaw {
    pub(crate) offset: u64,
    pub(crate) lookup_type: u16,
    pub(crate) lookup_flag: u16,
    pub(crate) subtable_offsets: Vec<u16>,
}

impl LookupRaw {
    fn new<R: BinaryReader>(reader: &mut R, offset: u64) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let lookup_type = reader.read_u16_be()?;
        let lookup_flag = reader.read_u16_be()?;
        let subtable_count = reader.read_u16_be()?;

        let mut subtable_offsets = Vec::new();
        for _ in 0..subtable_count {
            subtable_offsets.push(reader.read_u16_be()?);
        }
        Ok(Self {
            offset,
            lookup_type,
            lookup_flag,
            subtable_offsets,
        })
    }
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
    Unknown = 0xFFFF,
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
    pub(crate) fn get<R: BinaryReader>(
        number: usize,
        reader: &mut R,
        offset: u64,
        _: u32,
    ) -> Result<Lookup, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let lookup_count = reader.read_u16_be()?;
        let mut lookup_offsets = Vec::with_capacity(lookup_count as usize);
        for i in 0..lookup_count {
            lookup_offsets.push(reader.read_u16_be()?);
        }

        let mut lookups = Vec::new();
        for lookup_offset in lookup_offsets.iter() {
            let offset = offset + *lookup_offset as u64;
            let lookup_raw = LookupRaw::new(reader, offset)?;
            lookups.push(lookup_raw);
        }
        let lookup_raw = lookups.get(number).unwrap();
        Lookup::new(reader, lookup_raw)
    }

    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        _: u32,
    ) -> Result<Self, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let lookup_count = reader.read_u16_be()?;
        let mut lookup_offsets = Vec::with_capacity(lookup_count as usize);
        for i in 0..lookup_count {
            lookup_offsets.push(reader.read_u16_be()?);
        }

        let mut lookups = Vec::new();
        for lookup_offset in lookup_offsets.iter() {
            let offset = offset + *lookup_offset as u64;
            let lookup_raw = LookupRaw::new(reader, offset)?;

            lookups.push(lookup_raw);
        }
        let mut lookups_parsed = Vec::new();
        for lookup_raw in lookups.iter_mut() {
            let lookup = Lookup::new(reader, lookup_raw)?;
            lookups_parsed.push(lookup);
        }

        Ok(Self {
            lookups: Box::new(lookups_parsed),
        })
    }

    fn get_single<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let subst_format = reader.read_u16_be()?;
        let coverage_offset = reader.read_u16_be()?;
        if subst_format != 2 {
            let delta_glyph_id = reader.read_i16_be()?;
            let coverage = Self::get_coverage(reader, offset + coverage_offset as u64)?;
            return Ok(LookupSubstitution::Single(SingleSubstitutionFormat1 {
                subst_format,
                coverage,
                delta_glyph_id,
            }));
        }
        let glyph_count = reader.read_u16_be()?;
        let mut substitute_glyph_ids = Vec::new();
        for _ in 0..glyph_count {
            substitute_glyph_ids.push(reader.read_u16_be()?);
        }
        let coverage = Self::get_coverage(reader, offset + coverage_offset as u64)?;
        Ok(LookupSubstitution::Single2(SingleSubstitutionFormat2 {
            subst_format,
            coverage,
            glyph_count,
            substitute_glyph_ids: substitute_glyph_ids,
        }))
    }

    fn get_multiple<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let subst_format = reader.read_u16_be()?;
        let coverage_offset = reader.read_u16_be()?;
        let sequence_count = reader.read_u16_be()?;
        let mut sequence_offsets = Vec::new();
        for _ in 0..sequence_count {
            sequence_offsets.push(reader.read_u16_be()?);
        }
        let mut sequence_tables = Vec::new();
        for sequence_offset in sequence_offsets.iter() {
            let sequence_offset = *sequence_offset as u64 + offset;
            reader.seek(SeekFrom::Start(sequence_offset as u64))?;
            let glyph_count = reader.read_u16_be()?;
            let mut substitute_glyph_ids = Vec::new();
            for _ in 0..glyph_count {
                substitute_glyph_ids.push(reader.read_u16_be()?);
            }
            sequence_tables.push(SequenceTable {
                glyph_count,
                substitute_glyph_ids,
            });
        }
        let coverage = Self::get_coverage(reader, offset + coverage_offset as u64)?;
        Ok(LookupSubstitution::Multiple(MultipleSubstitutionFormat1 {
            subst_format,
            coverage,
            sequence_count,
            sequence_tables,
        }))
    }

    fn get_alternate<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let subst_format = reader.read_u16_be()?;
        let coverage_offset = reader.read_u16_be()?;
        let alternate_set_count = reader.read_u16_be()?;
        let mut alternet_set_offset = Vec::new();
        for _ in 0..alternate_set_count {
            alternet_set_offset.push(reader.read_u16_be()?);
        }

        let mut alternate_set = Vec::new();
        for alternet_set_offset in alternet_set_offset.iter() {
            let offset = *alternet_set_offset as u64 + offset;
            reader.seek(SeekFrom::Start(offset as u64))?;
            let glyph_count = reader.read_u16_be()?;
            let mut alternate_glyph_ids = Vec::new();
            for _ in 0..glyph_count {
                alternate_glyph_ids.push(reader.read_u16_be()?);
            }
            alternate_set.push(AlternateSet {
                glyph_count,
                alternate_glyph_ids,
            });
        }
        let coverage = Self::get_coverage(reader, offset + coverage_offset as u64)?;
        Ok(LookupSubstitution::Alternate(
            AlternateSubstitutionFormat1 {
                subst_format,
                coverage,
                alternate_set_count,
                alternate_set,
            },
        ))
    }

    fn get_ligature<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let subst_format = reader.read_u16_be()?;
        let coverage_offset = reader.read_u16_be()?;
        let ligature_set_count = reader.read_u16_be()?;
        let mut ligature_offset = Vec::new();
        for _ in 0..ligature_set_count {
            ligature_offset.push(reader.read_u16_be()?);
        }

        let mut ligature_set = Vec::new();
        for ligature_offset in ligature_offset.iter() {
            let offset = *ligature_offset as u64 + offset;
            reader.seek(SeekFrom::Start(offset as u64))?;
            let ligature_count = reader.read_u16_be()?;
            let mut lingature_offset = Vec::new();
            for _ in 0..ligature_count {
                lingature_offset.push(reader.read_u16_be()?);
            }

            let mut ligature_table = Vec::new();
            for lingature_offset in lingature_offset.iter() {
                let offset = *lingature_offset as u64 + offset;
                reader.seek(SeekFrom::Start(offset as u64))?;
                let ligature_glyph = reader.read_u16_be()?;
                let component_count = reader.read_u16_be()?;
                let mut component_glyph_ids = Vec::new();
                for _ in 0..component_count {
                    component_glyph_ids.push(reader.read_u16_be()?);
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
        let coverage = Self::get_coverage(reader, offset)?;
        Ok(LookupSubstitution::Ligature(LigatureSubstitutionFormat1 {
            subst_format,
            coverage,
            ligature_set_count,
            ligature_set,
        }))
    }

    fn get_context<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let subst_format = reader.read_u16_be()?;
        if subst_format == 1 {
            return Self::get_context_format1(reader, offset);
        } else if subst_format == 2 {
            return Self::get_context_format2(reader, offset);
        } else if subst_format == 3 {
            return Self::get_context_format3(reader, offset);
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unknown context format"),
            ))
        }
    }

    fn get_context_format1<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        let coverage_offset = reader.read_u16_be()?;
        let rule_set_count = reader.read_u16_be()?;
        let mut rule_set_offsets = Vec::new();
        for _ in 0..rule_set_count {
            rule_set_offsets.push(reader.read_u16_be()?);
        }
        let mut rule_sets = Vec::new();
        for rule_set_offset in rule_set_offsets.iter() {
            let rule_set = Self::get_seq_rule_set(reader, offset + *rule_set_offset as u64)?;
            rule_sets.push(rule_set);
        }
        let offset = offset + coverage_offset as u64;
        let coverage = Self::get_coverage(reader, offset)?;
        Ok(LookupSubstitution::ContextSubstitution(
            ContextSubstitutionFormat1 {
                subst_format: 1 as u16,
                coverage,
                rule_set_count,
                rule_sets,
            },
        ))
    }

    fn get_class_def<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<ClassDef, std::io::Error> {
        ClassDef::new(reader, offset)
    }

    fn get_context_format2<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        let coverage_offset = reader.read_u16_be()?;
        let class_def_offset = reader.read_u16_be()?;
        let class_seq_rule_set_count = reader.read_u16_be()?;
        let mut class_seq_rule_set_offsets = Vec::new();
        for _ in 0..class_seq_rule_set_count {
            class_seq_rule_set_offsets.push(reader.read_u16_be()?);
        }
        let mut class_seq_rule_sets = Vec::new();
        for class_seq_rule_set_offset in class_seq_rule_set_offsets.iter() {
            let offset = *class_seq_rule_set_offset as u64 + offset;
            let class_seq_rule_set = Self::get_class_seq_rule_set(reader, offset)?;
            class_seq_rule_sets.push(class_seq_rule_set);
        }
        let coverage = Self::get_coverage(reader, offset + coverage_offset as u64)?;
        let class_def = Self::get_class_def(reader, offset + class_def_offset as u64)?;

        Ok(LookupSubstitution::ContextSubstitution2(
            ContextSubstitutionFormat2 {
                subst_format: 2 as u16,
                coverage,
                class_def,
                class_seq_rule_set_count,
                class_seq_rule_sets,
            },
        ))
    }

    fn get_class_seq_rule_set<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<ClassSequenceRuleSet, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let class_seq_rule_count = reader.read_u16_be()?;
        let mut class_seq_rule_offsets = Vec::new();
        for _ in 0..class_seq_rule_count {
            class_seq_rule_offsets.push(reader.read_u16_be()?);
        }
        let mut class_seq_rules = Vec::new();
        for class_seq_rule_offset in class_seq_rule_offsets.iter() {
            if *class_seq_rule_offset == 0 {
                class_seq_rules.push(ClassSequenceRule {
                    glyph_count: 0,
                    input_sequences: Vec::new(),
                    seq_lookup_count: 0,
                    seq_lookup_records: SequenceLookupRecords {
                        lookup_records: Vec::new(),
                    },
                });
                continue;
            }
            let offset = *class_seq_rule_offset as u64 + offset;
            let class_seq_rule = Self::get_class_seq_rule(reader, offset)?;
            class_seq_rules.push(class_seq_rule);
        }
        Ok(ClassSequenceRuleSet {
            class_seq_rule_count,
            class_seq_rules,
        })
    }

    fn get_class_seq_rule<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<ClassSequenceRule, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let glyph_count = reader.read_u16_be()?;
        let seq_lookup_count = reader.read_u16_be()?;
        let mut input_sequences = Vec::new();
        for _ in 0..glyph_count as i32 - 1 {
            let input_sequence = reader.read_u16_be()?;
            input_sequences.push(input_sequence);
        }
        let mut lookup_records = Vec::new();
        for _ in 0..seq_lookup_count {
            let sequence_index = reader.read_u16_be()?;
            let lookup_list_index = reader.read_u16_be()?;
            lookup_records.push(LookupRecord {
                sequence_index,
                lookup_list_index,
            });
        }
        let seq_lookup_records = SequenceLookupRecords { lookup_records };

        Ok(ClassSequenceRule {
            glyph_count,
            input_sequences,
            seq_lookup_count,
            seq_lookup_records,
        })
    }

    fn get_seq_rule_set<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<SequenceRuleSet, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let rule_count = reader.read_u16_be()?;
        let mut rule_offsets = Vec::new();
        for _ in 0..rule_count {
            rule_offsets.push(reader.read_u16_be()?);
        }
        let mut rules = Vec::new();
        for rule_offset in rule_offsets.iter() {
            let offset = *rule_offset as u64 + offset;
            let rule = Self::get_seq_rule(reader, offset)?;
            rules.push(rule);
        }
        Ok(SequenceRuleSet { rule_count, rules })
    }

    fn get_seq_rule<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<SequenceRule, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let glyph_count = reader.read_u16_be()?;
        let lookup_count = reader.read_u16_be()?;
        let mut input_sequence = Vec::new();
        for _ in 0..glyph_count - 1 {
            input_sequence.push(reader.read_u16_be()?);
        }
        let mut lookup_indexes = Vec::new();
        for _ in 0..lookup_count {
            lookup_indexes.push(reader.read_u16_be()?);
        }
        Ok(SequenceRule {
            glyph_count,
            input_sequence,
            lookup_count,
            lookup_indexes,
        })
    }

    fn get_context_format3<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        // ContextSubstitutionFormat3
        let glyph_count = reader.read_u16_be()?;
        let seq_lookup_count = reader.read_u16_be()?;
        let mut coverage_offsets = Vec::new();
        for _ in 0..glyph_count {
            coverage_offsets.push(reader.read_u16_be()?);
        }
        let mut lookup_records = Vec::new();
        for _ in 0..seq_lookup_count {
            let sequence_index = reader.read_u16_be()?;
            let lookup_list_index = reader.read_u16_be()?;
            lookup_records.push(LookupRecord {
                sequence_index,
                lookup_list_index,
            });
        }
        let seq_lookup_records = SequenceLookupRecords { lookup_records };
        let mut coverages = Vec::new();
        for coverage_offset in coverage_offsets.iter() {
            let coverage = Self::get_coverage(reader, offset + *coverage_offset as u64)?;
            coverages.push(coverage);
        }
        Ok(LookupSubstitution::ContextSubstitution3(
            ContextSubstitutionFormat3 {
                subst_format: 3 as u16,
                glyph_count,
                coverages,
                seq_lookup_count,
                seq_lookup_records,
            },
        ))
    }

    fn get_chaining_context<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let subst_format = reader.read_u16_be()?;
        match subst_format {
            1 => {
                let coverage_offset = reader.read_u16_be()?;
                let chain_sub_rule_set_count = reader.read_u16_be()?;
                let mut chain_sub_rule_set_offsets = Vec::new();
                for _ in 0..chain_sub_rule_set_count {
                    chain_sub_rule_set_offsets.push(reader.read_u16_be()?);
                }
                let mut chain_sub_rule_set = Vec::new();
                for chain_sub_rule_set_offset in chain_sub_rule_set_offsets.iter() {
                    let offset = *chain_sub_rule_set_offset as u64 + offset;
                    reader.seek(SeekFrom::Start(offset as u64))?;
                    let chain_sub_rule_count = reader.read_u16_be()?;
                    let mut chain_sub_rule_offsets = Vec::new();
                    for _ in 0..chain_sub_rule_count {
                        chain_sub_rule_offsets.push(reader.read_u16_be()?);
                    }
                    let mut chain_sub_rule = Vec::new();
                    for chain_sub_rule_offset in chain_sub_rule_offsets.iter() {
                        let offset = *chain_sub_rule_offset as u64 + offset;
                        reader.seek(SeekFrom::Start(offset as u64))?;
                        let backtrack_glyph_count = reader.read_u16_be()?;
                        let mut backtrack_glyph_ids = Vec::new();
                        for _ in 0..backtrack_glyph_count {
                            backtrack_glyph_ids.push(reader.read_u16_be()?);
                        }
                        let input_glyph_count = reader.read_u16_be()?;
                        let mut input_glyph_ids = Vec::new();
                        for _ in 0..input_glyph_count {
                            input_glyph_ids.push(reader.read_u16_be()?);
                        }
                        let lookahead_glyph_count = reader.read_u16_be()?;
                        let mut lookahead_glyph_ids = Vec::new();
                        for _ in 0..lookahead_glyph_count {
                            lookahead_glyph_ids.push(reader.read_u16_be()?);
                        }
                        let lookup_count = reader.read_u16_be()?;
                        let mut lookup_indexes = Vec::new();
                        for _ in 0..lookup_count {
                            lookup_indexes.push(reader.read_u16_be()?);
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
                let coverage = Self::get_coverage(reader, offset)?;
                Ok(LookupSubstitution::ChainingContextSubstitution(
                    ChainingContextSubstitutionFormat1 {
                        subst_format,
                        coverage,
                        chain_sub_rule_set_count,
                        chain_sub_rule_set,
                    },
                ))
            }
            2 => {
                let coverage_offset = reader.read_u16_be()?;
                let class_range_count = reader.read_u16_be()?;
                let mut class_range_records = Vec::new();
                for _ in 0..class_range_count {
                    let start_glyph_id = reader.read_u16_be()?;
                    let end_glyph_id = reader.read_u16_be()?;
                    let class = reader.read_u16_be()?;
                    class_range_records.push(ClassRangeRecord {
                        start_glyph_id,
                        end_glyph_id,
                        class,
                    });
                }
                let coverage = Self::get_coverage(reader, offset + coverage_offset as u64)?;

                Ok(LookupSubstitution::ChainingContextSubstitution2(
                    ChainingContextSubstitutionFormat2 {
                        subst_format,
                        class_range_count,
                        class_range_records,
                        coverage,
                    },
                ))
            }
            3 => {
                let backtrack_glyph_count = reader.read_u16_be()?;
                let mut backtrack_coverage_offsets = Vec::new();
                for _ in 0..backtrack_glyph_count {
                    let coverage_offset = reader.read_u16_be()?;
                    backtrack_coverage_offsets.push(coverage_offset);
                }
                let input_glyph_count = reader.read_u16_be()?;
                let mut input_coverage_offsets = Vec::new();
                for _ in 0..input_glyph_count {
                    let coverage_offset = reader.read_u16_be()?;
                    input_coverage_offsets.push(coverage_offset);
                }
                let lookahead_glyph_count = reader.read_u16_be()?;
                let mut lookahead_coverage_offsets = Vec::new();
                for _ in 0..lookahead_glyph_count {
                    let coverage_offset = reader.read_u16_be()?;
                    lookahead_coverage_offsets.push(coverage_offset);
                }
                let seq_lookup_count = reader.read_u16_be()?;
                let mut lookup_records = Vec::new();
                for _ in 0..seq_lookup_count {
                    let sequence_index = reader.read_u16_be()?;
                    let lookup_list_index = reader.read_u16_be()?;
                    lookup_records.push(LookupRecord {
                        sequence_index,
                        lookup_list_index,
                    });
                }
                let seq_lookup_records = SequenceLookupRecords { lookup_records };

                let mut backtrack_coverages = Vec::new();
                for coverage_offset in backtrack_coverage_offsets.iter() {
                    let coverage = Self::get_coverage(reader, offset + *coverage_offset as u64)?;
                    backtrack_coverages.push(coverage);
                }

                let mut input_coverages = Vec::new();
                for coverage_offset in input_coverage_offsets.iter() {
                    let coverage = Self::get_coverage(reader, offset + *coverage_offset as u64)?;
                    input_coverages.push(coverage);
                }

                let mut lookahead_coverages = Vec::new();
                for coverage_offset in lookahead_coverage_offsets.iter() {
                    let coverage = Self::get_coverage(reader, offset + *coverage_offset as u64)?;
                    lookahead_coverages.push(coverage);
                }

                Ok(LookupSubstitution::ChainingContextSubstitution3(
                    ChainingContextSubstitutionFormat3 {
                        format: subst_format,
                        backtrack_glyph_count,
                        backtrack_coverages,
                        input_glyph_count,
                        input_coverages,
                        lookahead_glyph_count,
                        lookahead_coverages,
                        seq_lookup_count,
                        seq_lookup_records,
                    },
                ))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unknown chaining context format"),
            )),
        }
    }

    fn get_coverage<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<Coverage, std::io::Error> {
        Coverage::new(reader, offset)
    }

    fn get_extension<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<LookupSubstitution, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let _ = reader.read_u16_be(); // 1
        let extension_lookup_type = reader.read_u16_be()?;
        let extension_offset = reader.read_u32_be()?;

        let offset = offset + extension_offset as u64;
        reader.seek(SeekFrom::Start(offset as u64))?;
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
    ) -> Result<LookupSubstitution, std::io::Error> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let subst_format = reader.read_u16_be()?;
        let coverage_offset = reader.read_u16_be()?;
        let backtrack_glyph_count = reader.read_u16_be()?;
        let mut backtrack_glyph_ids = Vec::new();
        for _ in 0..backtrack_glyph_count {
            backtrack_glyph_ids.push(reader.read_u16_be()?);
        }
        let input_glyph_count = reader.read_u16_be()?;
        let mut input_glyph_ids = Vec::new();
        for _ in 0..input_glyph_count {
            input_glyph_ids.push(reader.read_u16_be()?);
        }
        let lookahead_glyph_count = reader.read_u16_be()?;
        let mut lookahead_glyph_ids = Vec::new();
        for _ in 0..lookahead_glyph_count {
            lookahead_glyph_ids.push(reader.read_u16_be()?);
        }
        let substitute_glyph_id = reader.read_u16_be()?;
        Ok(LookupSubstitution::ReverseChainSingle(
            ReverseChainSingleSubstitutionFormat1 {
                subst_format,
                coverage_offset,
                backtrack_glyph_count,
                backtrack_glyph_ids,
                input_glyph_count,
                input_glyph_ids,
                lookahead_glyph_count,
                lookahead_glyph_ids,
                substitute_glyph_id,
            },
        ))
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
    ContextSubstitution2(ContextSubstitutionFormat2),
    // 5.3
    ContextSubstitution3(ContextSubstitutionFormat3),
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
    Context(Vec<SequenceRule>),
    Chaining(Vec<ChainSubRule>),
    Chaing63(Vec<ChainSubRule>),
    None,
}

impl LookupSubstitution {
    pub(crate) fn to_string(&self) -> String {
        format!("{:?}", self)
    }

    pub(crate) fn get_coverage(
        &self,
    ) -> (
        &Coverage,
        Option<(Vec<Coverage>, Vec<Coverage>, Vec<Coverage>)>,
    ) {
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
                    let rule_set = &context.rule_sets[id];
                    let result = rule_set.rules.clone();
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
            Self::ChainingContextSubstitution3(_chaining3) => {
                panic!("ChainingContextSubstitution3 is not implemented")
            }
            Self::ExtensionSubstitution(_) => {
                panic!("ExtensionSubstitution is not implemented")
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
    pub(crate) rule_sets: Vec<SequenceRuleSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct SequenceRuleSet {
    pub(crate) rule_count: u16,
    pub(crate) rules: Vec<SequenceRule>,
}

#[derive(Debug, Clone)]
pub(crate) struct SequenceRule {
    pub(crate) glyph_count: u16,
    pub(crate) input_sequence: Vec<u16>,
    pub(crate) lookup_count: u16,
    pub(crate) lookup_indexes: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContextSubstitutionFormat2 {
    pub(crate) subst_format: u16,
    pub(crate) coverage: Coverage,
    pub(crate) class_def: ClassDef,
    pub(crate) class_seq_rule_set_count: u16,
    pub(crate) class_seq_rule_sets: Vec<ClassSequenceRuleSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct ClassSequenceRuleSet {
    pub(crate) class_seq_rule_count: u16,
    pub(crate) class_seq_rules: Vec<ClassSequenceRule>,
}

#[derive(Debug, Clone)]
pub(crate) struct ClassSequenceRule {
    pub(crate) glyph_count: u16,
    pub(crate) seq_lookup_count: u16,
    pub(crate) input_sequences: Vec<u16>,
    pub(crate) seq_lookup_records: SequenceLookupRecords,
}

#[derive(Debug, Clone)]
pub(crate) struct ChaineClassSequenceRuleSet {
    pub(crate) chain_sub_class_set_count: u16,
    pub(crate) chained_class_seq_rules: Vec<ChaineClassSequenceRule>,
}

#[derive(Debug, Clone)]
pub(crate) struct ChaineClassSequenceRule {
    pub(crate) backtrack_glyph_count: u16,
    pub(crate) backtrack_sequences: Vec<u16>,
    pub(crate) input_glyph_count: u16,
    pub(crate) input_sequences: Vec<u16>,
    pub(crate) lookahead_glyph_count: u16,
    pub(crate) lookahead_class_ids: Vec<u16>,
    pub(crate) lookup_count: u16,
    pub(crate) lookup_indexes: Vec<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContextSubstitutionFormat3 {
    pub(crate) subst_format: u16,
    pub(crate) glyph_count: u16,
    pub(crate) seq_lookup_count: u16,
    pub(crate) coverages: Vec<Coverage>,
    pub(crate) seq_lookup_records: SequenceLookupRecords,
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
