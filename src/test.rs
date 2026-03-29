#[allow(deprecated)]
mod tests {
    #[cfg(feature = "layout")]
    use crate::opentype::layouts::{
        classdef::ClassRangeRecord,
        coverage::{Coverage, CoverageFormat1, CoverageFormat2, RangeRecord},
        lookup::{
            AlternateSet, AlternateSubstitutionFormat1, ChainSubRule, ChainSubRuleSet,
            ChainingContextSubstitutionFormat1, ChainingContextSubstitutionFormat2,
            ChainingContextSubstitutionFormat3, ContextSubstitutionFormat1, LigatureSet,
            LigatureSubstitutionFormat1, LigatureTable, Lookup, LookupList, LookupResult,
            LookupSubstitution, LookupType, MultipleSubstitutionFormat1, SequenceLookupRecords,
            SequenceRule, SequenceRuleSet, SequenceTable, SingleSubstitutionFormat1,
            SingleSubstitutionFormat2,
        },
    };

    #[cfg(feature = "layout")]
    fn coverage_format1(glyph_ids: &[u16]) -> Coverage {
        Coverage::Format1(CoverageFormat1 {
            coverage_format: 1,
            glyph_count: glyph_ids.len() as u16,
            glyph_ids: glyph_ids.to_vec(),
        })
    }

    #[cfg(feature = "layout")]
    fn coverage_format2(ranges: &[(u16, u16, u16)]) -> Coverage {
        Coverage::Format2(CoverageFormat2 {
            coverage_format: 2,
            range_count: ranges.len() as u16,
            range_records: ranges
                .iter()
                .map(
                    |(start_glyph_id, end_glyph_id, start_coverage_index)| RangeRecord {
                        start_glyph_id: *start_glyph_id,
                        end_glyph_id: *end_glyph_id,
                        start_coverage_index: *start_coverage_index,
                    },
                )
                .collect(),
        })
    }

    #[cfg(any())]
    fn convert() {
        use crate::fontheader::f2dot14_to_f32;
        use crate::fontheader::fixed_to_f32;
        /*1.999939	0x7fff	1	16383/16384
        1.75	0x7000	1	12288/16384
        0.000061	0x0001	0	1/16384
        0.0	0x0000	0	0/16384
        -0.000061	0xffff	-1	16383/16384
        -2.0	0x8000	-2	0/16384 */
        let value = f2dot14_to_f32(0x7fff);
        assert_eq!(value, 1.999939);
        let value = f2dot14_to_f32(0x7000);
        assert_eq!(value, 1.75);
        let value = f2dot14_to_f32(0x0001);
        assert_eq!(value, 0.000061);
        let value = f2dot14_to_f32(0x0000);
        assert_eq!(value, 0.0);
        let value = f2dot14_to_f32(0xffff);
        assert_eq!(value, -0.000061);
        let value = f2dot14_to_f32(0x8000);
        assert_eq!(value, -2.0);
        let value = fixed_to_f32(0x7fff_ffff);
        assert_eq!(value, 1.9999998807907104);
        let value = fixed_to_f32(0x7000_0000);
        assert_eq!(value, 1.75);
        let value = fixed_to_f32(0x0000_0001);
        assert_eq!(value, 0.00000005960464477539);
        let value = fixed_to_f32(0x0000_0000);
        assert_eq!(value, 0.0);
        let value = fixed_to_f32(0xffff_ffff);
        assert_eq!(value, -0.00000005960464477539);
        let value = fixed_to_f32(0x8000_0000);
        assert_eq!(value, -2.0);
    }

    #[test]
    #[cfg(feature = "cff")]
    fn operand_encoding_test() -> Result<(), Box<dyn std::error::Error>> {
        use crate::opentype::outline::cff::{operand_encoding, Operand};
        let b = [0x8b];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, 0);
            assert_eq!(len, 1);
        } else {
            panic!("not integer");
        }

        for b in 32..246 {
            let buf = [b];
            let (value, len) = operand_encoding(&buf)?;
            if let Operand::Integer(value) = value {
                assert_eq!(value, b as i32 - 139);
                assert_eq!(len, 1);
            } else {
                panic!("not integer");
            }
        }

        let b = [0xef];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, 100);
            assert_eq!(len, 1);
        } else {
            panic!("not integer");
        }

        let b = [0x27];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, -100);
            assert_eq!(len, 1);
        } else {
            panic!("not integer");
        }
        let b = [0xfa, 0x7c];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, 1000);
            assert_eq!(len, 2);
        } else {
            panic!("not real");
        }
        let b = [0xfe, 0x7c];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, -1000);
            assert_eq!(len, 2);
        } else {
            panic!("not integer");
        }
        let b = [0x1c, 0x27, 0x10];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, 10000);
            assert_eq!(len, 3);
        } else {
            panic!("not integer");
        }
        let b = [0x1c, 0xd8, 0xf0];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, -10000);
            assert_eq!(len, 3);
        } else {
            panic!("not integer");
        }
        let b = [0x1d, 0x00, 0x01, 0x86, 0xa0];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, 100000);
            assert_eq!(len, 5);
        } else {
            panic!("not integer");
        }
        let b = [0x1d, 0xff, 0xfe, 0x79, 0x60];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Integer(value) = value {
            assert_eq!(value, -100000);
            assert_eq!(len, 5);
        } else {
            panic!("not integer");
        }
        let b = [31];
        let value = operand_encoding(&b);
        assert!(value.is_ok());

        let b = [0x1e, 0x2e, 0xa2, 0x5f];
        let value = operand_encoding(&b);
        assert!(value.is_ok());

        let b = [0x1e, 0xe2, 0xa2, 0x5f];
        let (value, len) = operand_encoding(&b)?;
        // -2.25
        if let Operand::Real(value) = value {
            assert_eq!(value, -2.25);
            assert_eq!(len, 4);
        } else {
            panic!("not real");
        }

        // 0.140541e-3
        let b = [0x1e, 0x0a, 0x14, 0x05, 0x41, 0xc3, 0xff];
        let (value, len) = operand_encoding(&b)?;
        if let Operand::Real(value) = value {
            assert_eq!(value, 0.140541e-3);
            assert_eq!(len, 7);
        } else {
            panic!("not real");
        }

        Ok(())
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_single_substitution_expands_glyphs() {
        let single = LookupSubstitution::Single(SingleSubstitutionFormat1 {
            subst_format: 1,
            coverage: coverage_format1(&[10, 12]),
            delta_glyph_id: 3,
        });
        assert_eq!(single.get_single_glyph_id(10), Some(13));
        assert_eq!(single.get_single_glyph_id(11), None);

        let single2 = LookupSubstitution::Single2(SingleSubstitutionFormat2 {
            subst_format: 2,
            coverage: coverage_format2(&[(20, 21, 0), (30, 30, 2)]),
            glyph_count: 3,
            substitute_glyph_ids: vec![120, 121, 130],
        });
        assert_eq!(single2.get_single_glyph_id(20), Some(120));
        assert_eq!(single2.get_single_glyph_id(21), Some(121));
        assert_eq!(single2.get_single_glyph_id(30), Some(130));
        assert_eq!(single2.get_single_glyph_id(31), None);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_multiple_and_alternate_expand_sequences() {
        let multiple = LookupSubstitution::Multiple(MultipleSubstitutionFormat1 {
            subst_format: 1,
            coverage: coverage_format1(&[40, 41]),
            sequence_count: 2,
            sequence_tables: vec![
                SequenceTable {
                    glyph_count: 2,
                    substitute_glyph_ids: vec![400, 401],
                },
                SequenceTable {
                    glyph_count: 3,
                    substitute_glyph_ids: vec![410, 411, 412],
                },
            ],
        });
        match multiple.get_lookup(41) {
            LookupResult::Multiple(ids) => assert_eq!(ids, vec![410, 411, 412]),
            _ => panic!("unexpected lookup result"),
        }

        let alternate = LookupSubstitution::Alternate(AlternateSubstitutionFormat1 {
            subst_format: 1,
            coverage: coverage_format1(&[50]),
            alternate_set_count: 1,
            alternate_set: vec![AlternateSet {
                glyph_count: 2,
                alternate_glyph_ids: vec![500, 501],
            }],
        });
        match alternate.get_lookup(50) {
            LookupResult::Multiple(ids) => assert_eq!(ids, vec![500, 501]),
            _ => panic!("unexpected lookup result"),
        }
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_ligature_and_context_expand_records() {
        let ligature = LookupSubstitution::Ligature(LigatureSubstitutionFormat1 {
            subst_format: 1,
            coverage: coverage_format1(&[60]),
            ligature_set_count: 1,
            ligature_set: vec![LigatureSet {
                ligature_count: 1,
                ligature_table: vec![LigatureTable {
                    ligature_glyph: 600,
                    component_count: 3,
                    component_glyph_ids: vec![61, 62],
                }],
            }],
        });
        match ligature.get_lookup(60) {
            LookupResult::Ligature(records) => {
                assert_eq!(records.len(), 1);
                assert_eq!(records[0].ligature_glyph, 600);
                assert_eq!(records[0].component_glyph_ids, vec![61, 62]);
            }
            _ => panic!("unexpected lookup result"),
        }

        let context = LookupSubstitution::ContextSubstitution(ContextSubstitutionFormat1 {
            subst_format: 1,
            coverage: coverage_format1(&[70]),
            rule_set_count: 1,
            rule_sets: vec![SequenceRuleSet {
                rule_count: 1,
                rules: vec![SequenceRule {
                    glyph_count: 2,
                    input_sequence: vec![71],
                    lookup_count: 1,
                    lookup_indexes: vec![9],
                }],
            }],
        });
        match context.get_lookup(70) {
            LookupResult::Context(rules) => {
                assert_eq!(rules.len(), 1);
                assert_eq!(rules[0].input_sequence, vec![71]);
                assert_eq!(rules[0].lookup_indexes, vec![9]);
            }
            _ => panic!("unexpected lookup result"),
        }
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_chaining_variants_expand_expected_payloads() {
        let chaining =
            LookupSubstitution::ChainingContextSubstitution(ChainingContextSubstitutionFormat1 {
                subst_format: 1,
                coverage: coverage_format1(&[80]),
                chain_sub_rule_set_count: 1,
                chain_sub_rule_set: vec![ChainSubRuleSet {
                    chain_sub_rule_count: 1,
                    chain_sub_rule: vec![ChainSubRule {
                        backtrack_glyph_count: 1,
                        backtrack_glyph_ids: vec![79],
                        input_glyph_count: 2,
                        input_glyph_ids: vec![81],
                        lookahead_glyph_count: 1,
                        lookahead_glyph_ids: vec![82],
                        lookup_count: 1,
                        lookup_indexes: vec![7],
                    }],
                }],
            });
        match chaining.get_lookup(80) {
            LookupResult::Chaining(rules) => {
                assert_eq!(rules.len(), 1);
                assert_eq!(rules[0].backtrack_glyph_ids, vec![79]);
                assert_eq!(rules[0].lookup_indexes, vec![7]);
            }
            _ => panic!("unexpected lookup result"),
        }

        let chaining2 =
            LookupSubstitution::ChainingContextSubstitution2(ChainingContextSubstitutionFormat2 {
                subst_format: 2,
                class_range_count: 1,
                class_range_records: vec![ClassRangeRecord {
                    start_glyph_id: 90,
                    end_glyph_id: 90,
                    class: 4,
                }],
                coverage: coverage_format1(&[90]),
                backtrack_class_def: None,
                input_class_def: None,
                lookahead_class_def: None,
                chain_sub_class_set_count: 0,
                chain_sub_class_sets: vec![],
            });
        match chaining2.get_lookup(90) {
            LookupResult::Multiple(classes) => assert_eq!(classes, vec![4]),
            _ => panic!("unexpected lookup result"),
        }

        let chaining3 =
            LookupSubstitution::ChainingContextSubstitution3(ChainingContextSubstitutionFormat3 {
                format: 3,
                backtrack_glyph_count: 1,
                backtrack_coverages: vec![coverage_format1(&[100])],
                input_glyph_count: 1,
                input_coverages: vec![coverage_format1(&[101, 102])],
                lookahead_glyph_count: 1,
                lookahead_coverages: vec![coverage_format1(&[103])],
                seq_lookup_count: 1,
                seq_lookup_records: SequenceLookupRecords {
                    lookup_records: vec![],
                },
            });
        let (first, coverages) = chaining3.get_coverage();
        assert_eq!(first.contains(100), Some(0));
        let (backtrack, input, lookahead) = coverages.expect("coverage tuple");
        assert_eq!(backtrack[0].contains(100), Some(0));
        assert_eq!(input[0].contains(102), Some(1));
        assert_eq!(lookahead[0].contains(103), Some(0));
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_reports_none_for_non_covered_glyph() {
        let single = LookupSubstitution::Single(SingleSubstitutionFormat1 {
            subst_format: 1,
            coverage: coverage_format1(&[200]),
            delta_glyph_id: 1,
        });
        match single.get_lookup(201) {
            LookupResult::None => {}
            _ => panic!("expected none"),
        }
    }

    #[cfg(feature = "layout")]
    fn build_lookup_list(tables: Vec<Vec<u8>>) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, tables.len() as u16);

        let offsets_pos = buffer.len();
        buffer.resize(buffer.len() + tables.len() * 2, 0);

        let mut offsets = Vec::new();
        for table in tables {
            offsets.push(buffer.len() as u16);
            buffer.extend_from_slice(&table);
        }

        for (index, offset) in offsets.iter().enumerate() {
            let start = offsets_pos + index * 2;
            buffer[start..start + 2].copy_from_slice(&offset.to_be_bytes());
        }

        buffer
    }

    #[cfg(feature = "layout")]
    fn lookup_single_subtable(glyph_id: u16, delta_glyph_id: i16) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 6);
        push_u16(&mut buffer, delta_glyph_id as u16);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, glyph_id);
        buffer
    }

    #[cfg(feature = "layout")]
    fn build_lookup_record(lookup_type: u16, subtable: Vec<u8>) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, lookup_type);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 8);
        buffer.extend_from_slice(&subtable);
        buffer
    }

    #[cfg(feature = "layout")]
    fn lookup_extension_subtable(glyph_id: u16, delta_glyph_id: i16) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, LookupType::SingleSubstitution as u16);
        push_u32(&mut buffer, 8);
        buffer.extend_from_slice(&lookup_single_subtable(glyph_id, delta_glyph_id));
        build_lookup_record(LookupType::ExtensionSubstitution as u16, buffer)
    }

    #[cfg(feature = "layout")]
    fn lookup_reverse_chain_subtable(
        coverage_glyph_id: u16,
        substitute_glyph_id: u16,
        backtrack_glyph_id: u16,
        input_glyph_id: u16,
        lookahead_glyph_id: u16,
    ) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, backtrack_glyph_id);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, input_glyph_id);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, lookahead_glyph_id);
        push_u16(&mut buffer, substitute_glyph_id);

        let coverage_offset = buffer.len() as u16;
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, coverage_glyph_id);
        buffer[2..4].copy_from_slice(&coverage_offset.to_be_bytes());
        build_lookup_record(
            LookupType::ReverseChainingContextualSingleSubstitution as u16,
            buffer,
        )
    }

    #[cfg(feature = "layout")]
    fn parse_lookup_list(tables: Vec<Vec<u8>>) -> LookupList {
        let buffer = build_lookup_list(tables);
        let mut reader = BytesReader::new(&buffer);
        LookupList::new(&mut reader, 0, buffer.len() as u32).unwrap()
    }

    #[cfg(feature = "layout")]
    fn coverage_table(glyph_ids: &[u16]) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, glyph_ids.len() as u16);
        for glyph_id in glyph_ids {
            push_u16(&mut buffer, *glyph_id);
        }
        buffer
    }

    #[cfg(feature = "layout")]
    fn build_gpos_pair_format1_subtable(left: u16, right: u16, x_advance: i16) -> Vec<u8> {
        let coverage = coverage_table(&[left]);
        let mut pair_set = Vec::new();
        push_u16(&mut pair_set, 1);
        push_u16(&mut pair_set, right);
        push_u16(&mut pair_set, x_advance as u16);

        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 12);
        push_u16(&mut buffer, 0x0004);
        push_u16(&mut buffer, 0x0000);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, (12 + coverage.len()) as u16);
        buffer.extend_from_slice(&coverage);
        buffer.extend_from_slice(&pair_set);
        buffer
    }

    #[cfg(feature = "layout")]
    fn class_def_format1(start_glyph_id: u16, class_values: &[u16]) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, start_glyph_id);
        push_u16(&mut buffer, class_values.len() as u16);
        for class_value in class_values {
            push_u16(&mut buffer, *class_value);
        }
        buffer
    }

    #[cfg(feature = "layout")]
    fn build_gpos_pair_format2_subtable(left: u16, right: u16, x_advance: i16) -> Vec<u8> {
        let coverage = coverage_table(&[left]);
        let class_def1 = class_def_format1(left, &[1]);
        let class_def2 = class_def_format1(right, &[1]);

        let class1_count = 2u16;
        let class2_count = 2u16;

        let mut class_records = Vec::new();
        for class1 in 0..class1_count {
            for class2 in 0..class2_count {
                let value = if class1 == 1 && class2 == 1 {
                    x_advance as u16
                } else {
                    0
                };
                push_u16(&mut class_records, value);
            }
        }

        let coverage_offset = 16u16 + class_records.len() as u16;
        let class_def1_offset = coverage_offset + coverage.len() as u16;
        let class_def2_offset = class_def1_offset + class_def1.len() as u16;

        let mut buffer = Vec::new();
        push_u16(&mut buffer, 2);
        push_u16(&mut buffer, coverage_offset);
        push_u16(&mut buffer, 0x0004);
        push_u16(&mut buffer, 0x0000);
        push_u16(&mut buffer, class_def1_offset);
        push_u16(&mut buffer, class_def2_offset);
        push_u16(&mut buffer, class1_count);
        push_u16(&mut buffer, class2_count);
        buffer.extend_from_slice(&class_records);
        buffer.extend_from_slice(&coverage);
        buffer.extend_from_slice(&class_def1);
        buffer.extend_from_slice(&class_def2);
        buffer
    }

    #[cfg(feature = "layout")]
    fn build_script_list_with_language_systems(
        scripts: &[([u8; 4], &[(u32, u16, &[u16])])],
    ) -> Vec<u8> {
        let mut script_list = Vec::new();
        push_u16(&mut script_list, scripts.len() as u16);

        let script_records_pos = script_list.len();
        script_list.resize(script_list.len() + scripts.len() * 6, 0);

        let mut script_offsets = Vec::new();
        for (_, language_systems) in scripts {
            let script_start = script_list.len();
            script_offsets.push(script_start as u16);

            let default_index = language_systems.iter().position(|(tag, _, _)| *tag == 0);
            let non_default_count = language_systems.len() - usize::from(default_index.is_some());

            push_u16(&mut script_list, 0);
            push_u16(&mut script_list, non_default_count as u16);

            let language_records_pos = script_list.len();
            script_list.resize(script_list.len() + non_default_count * 6, 0);

            let mut default_offset = 0u16;
            let mut non_default_offsets = Vec::new();

            for (tag, required_feature_index, feature_indices) in *language_systems {
                let offset = (script_list.len() - script_start) as u16;
                push_u16(&mut script_list, 0);
                push_u16(&mut script_list, *required_feature_index);
                push_u16(&mut script_list, feature_indices.len() as u16);
                for feature_index in *feature_indices {
                    push_u16(&mut script_list, *feature_index);
                }

                if *tag == 0 {
                    default_offset = offset;
                } else {
                    non_default_offsets.push((*tag, offset));
                }
            }

            script_list[script_start..script_start + 2]
                .copy_from_slice(&default_offset.to_be_bytes());

            for (index, (tag, offset)) in non_default_offsets.iter().enumerate() {
                let start = language_records_pos + index * 6;
                script_list[start..start + 4].copy_from_slice(&tag.to_be_bytes());
                script_list[start + 4..start + 6].copy_from_slice(&offset.to_be_bytes());
            }
        }

        for (index, (script_tag, _)) in scripts.iter().enumerate() {
            let start = script_records_pos + index * 6;
            script_list[start..start + 4].copy_from_slice(script_tag);
            script_list[start + 4..start + 6].copy_from_slice(&script_offsets[index].to_be_bytes());
        }

        script_list
    }

    #[cfg(feature = "layout")]
    fn build_script_list_with_default_lang_systems(scripts: &[([u8; 4], u16, &[u16])]) -> Vec<u8> {
        let mut script_list = Vec::new();
        push_u16(&mut script_list, scripts.len() as u16);

        let script_records_pos = script_list.len();
        script_list.resize(script_list.len() + scripts.len() * 6, 0);

        let mut script_offsets = Vec::new();
        for (_, required_feature_index, feature_indices) in scripts {
            script_offsets.push(script_list.len() as u16);
            push_u16(&mut script_list, 4);
            push_u16(&mut script_list, 0);
            push_u16(&mut script_list, 0);
            push_u16(&mut script_list, *required_feature_index);
            push_u16(&mut script_list, feature_indices.len() as u16);
            for feature_index in *feature_indices {
                push_u16(&mut script_list, *feature_index);
            }
        }

        for (index, (script_tag, _, _)) in scripts.iter().enumerate() {
            let start = script_records_pos + index * 6;
            script_list[start..start + 4].copy_from_slice(script_tag);
            script_list[start + 4..start + 6].copy_from_slice(&script_offsets[index].to_be_bytes());
        }

        script_list
    }

    #[cfg(feature = "layout")]
    fn build_feature_list_with_entries(features: &[([u8; 4], &[u16])]) -> Vec<u8> {
        let mut feature_list = Vec::new();
        push_u16(&mut feature_list, features.len() as u16);

        let feature_records_pos = feature_list.len();
        feature_list.resize(feature_list.len() + features.len() * 6, 0);

        let mut feature_offsets = Vec::new();
        for (_, lookup_indices) in features {
            feature_offsets.push(feature_list.len() as u16);
            push_u16(&mut feature_list, 0);
            push_u16(&mut feature_list, lookup_indices.len() as u16);
            for lookup_index in *lookup_indices {
                push_u16(&mut feature_list, *lookup_index);
            }
        }

        for (index, (feature_tag, _)) in features.iter().enumerate() {
            let start = feature_records_pos + index * 6;
            feature_list[start..start + 4].copy_from_slice(feature_tag);
            feature_list[start + 4..start + 6]
                .copy_from_slice(&feature_offsets[index].to_be_bytes());
        }

        feature_list
    }

    #[cfg(feature = "layout")]
    fn build_gpos_table(feature_tag: [u8; 4], lookup_type: u16, subtable: Vec<u8>) -> Vec<u8> {
        build_gpos_table_with_scripted_features(
            &[(*b"DFLT", 0xFFFF, &[0])],
            &[(feature_tag, &[0])],
            lookup_type,
            vec![subtable],
        )
    }

    #[cfg(feature = "layout")]
    fn build_gpos_table_with_scripted_features(
        scripts: &[([u8; 4], u16, &[u16])],
        features: &[([u8; 4], &[u16])],
        lookup_type: u16,
        subtables: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        let script_list = build_script_list_with_default_lang_systems(scripts);
        let feature_list = build_feature_list_with_entries(features);

        let mut lookup_list = Vec::new();
        push_u16(&mut lookup_list, subtables.len() as u16);
        let lookup_offsets_pos = lookup_list.len();
        lookup_list.resize(lookup_list.len() + subtables.len() * 2, 0);

        let mut lookup_offsets = Vec::new();
        for subtable in subtables {
            lookup_offsets.push(lookup_list.len() as u16);
            push_u16(&mut lookup_list, lookup_type);
            push_u16(&mut lookup_list, 0);
            push_u16(&mut lookup_list, 1);
            push_u16(&mut lookup_list, 8);
            lookup_list.extend_from_slice(&subtable);
        }

        for (index, offset) in lookup_offsets.iter().enumerate() {
            let start = lookup_offsets_pos + index * 2;
            lookup_list[start..start + 2].copy_from_slice(&offset.to_be_bytes());
        }

        let script_list_offset = 10u16;
        let feature_list_offset = script_list_offset + script_list.len() as u16;
        let lookup_list_offset = feature_list_offset + feature_list.len() as u16;

        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, script_list_offset);
        push_u16(&mut buffer, feature_list_offset);
        push_u16(&mut buffer, lookup_list_offset);
        buffer.extend_from_slice(&script_list);
        buffer.extend_from_slice(&feature_list);
        buffer.extend_from_slice(&lookup_list);
        buffer
    }

    #[cfg(feature = "layout")]
    fn parse_gpos(buffer: Vec<u8>) -> crate::opentype::extentions::gpos::GPOS {
        let mut reader = BytesReader::new(&buffer);
        crate::opentype::extentions::gpos::GPOS::new(&mut reader, 0, buffer.len() as u32).unwrap()
    }

    #[cfg(feature = "layout")]
    fn build_gsub_table(feature_tag: [u8; 4], lookups: Vec<Vec<u8>>) -> Vec<u8> {
        let feature_lookup_indices: Vec<u16> = (0..lookups.len() as u16).collect();
        build_gsub_table_with_feature_lookups(feature_tag, &feature_lookup_indices, lookups)
    }

    #[cfg(feature = "layout")]
    fn build_gsub_table_with_feature_lookups(
        feature_tag: [u8; 4],
        feature_lookup_indices: &[u16],
        lookups: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        let mut script_list = Vec::new();
        push_u16(&mut script_list, 1);
        script_list.extend_from_slice(b"DFLT");
        push_u16(&mut script_list, 8);
        push_u16(&mut script_list, 4);
        push_u16(&mut script_list, 0);
        push_u16(&mut script_list, 0);
        push_u16(&mut script_list, 0xFFFF);
        push_u16(&mut script_list, 1);
        push_u16(&mut script_list, 0);

        let mut feature_list = Vec::new();
        push_u16(&mut feature_list, 1);
        feature_list.extend_from_slice(&feature_tag);
        push_u16(&mut feature_list, 8);
        push_u16(&mut feature_list, 0);
        push_u16(&mut feature_list, feature_lookup_indices.len() as u16);
        for lookup_index in feature_lookup_indices {
            push_u16(&mut feature_list, *lookup_index);
        }

        let lookup_list = build_lookup_list(lookups);
        let script_list_offset = 10u16;
        let feature_list_offset = script_list_offset + script_list.len() as u16;
        let lookup_list_offset = feature_list_offset + feature_list.len() as u16;

        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, script_list_offset);
        push_u16(&mut buffer, feature_list_offset);
        push_u16(&mut buffer, lookup_list_offset);
        buffer.extend_from_slice(&script_list);
        buffer.extend_from_slice(&feature_list);
        buffer.extend_from_slice(&lookup_list);
        buffer
    }

    #[cfg(feature = "layout")]
    fn build_gsub_table_with_scripted_feature_lookups(
        scripts: &[([u8; 4], &[u16])],
        feature_tag: [u8; 4],
        feature_lookup_indices: &[Vec<u16>],
        lookups: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        let scripts_with_required = scripts
            .iter()
            .map(|(script_tag, feature_indices)| (*script_tag, 0xFFFF, *feature_indices))
            .collect::<Vec<_>>();
        let features = feature_lookup_indices
            .iter()
            .map(|lookup_indices| (feature_tag, lookup_indices.as_slice()))
            .collect::<Vec<_>>();
        let script_list = build_script_list_with_default_lang_systems(&scripts_with_required);
        let feature_list = build_feature_list_with_entries(&features);

        let lookup_list = build_lookup_list(lookups);
        let script_list_offset = 10u16;
        let feature_list_offset = script_list_offset + script_list.len() as u16;
        let lookup_list_offset = feature_list_offset + feature_list.len() as u16;

        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, script_list_offset);
        push_u16(&mut buffer, feature_list_offset);
        push_u16(&mut buffer, lookup_list_offset);
        buffer.extend_from_slice(&script_list);
        buffer.extend_from_slice(&feature_list);
        buffer.extend_from_slice(&lookup_list);
        buffer
    }

    #[cfg(feature = "layout")]
    fn build_gsub_table_with_scripted_features(
        scripts: &[([u8; 4], u16, &[u16])],
        features: &[([u8; 4], &[u16])],
        lookups: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        let script_list = build_script_list_with_default_lang_systems(scripts);
        let feature_list = build_feature_list_with_entries(features);
        let lookup_list = build_lookup_list(lookups);
        let script_list_offset = 10u16;
        let feature_list_offset = script_list_offset + script_list.len() as u16;
        let lookup_list_offset = feature_list_offset + feature_list.len() as u16;

        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, script_list_offset);
        push_u16(&mut buffer, feature_list_offset);
        push_u16(&mut buffer, lookup_list_offset);
        buffer.extend_from_slice(&script_list);
        buffer.extend_from_slice(&feature_list);
        buffer.extend_from_slice(&lookup_list);
        buffer
    }

    #[cfg(feature = "layout")]
    fn lookup_context_format3_record(
        coverages: &[Vec<u8>],
        sequence_index: u16,
        lookup_list_index: u16,
    ) -> Vec<u8> {
        let mut subtable = Vec::new();
        push_u16(&mut subtable, 3);
        push_u16(&mut subtable, coverages.len() as u16);
        push_u16(&mut subtable, 1);

        let offsets_pos = subtable.len();
        subtable.resize(subtable.len() + coverages.len() * 2, 0);
        push_u16(&mut subtable, sequence_index);
        push_u16(&mut subtable, lookup_list_index);

        let mut offsets = Vec::new();
        for coverage in coverages {
            offsets.push(subtable.len() as u16);
            subtable.extend_from_slice(coverage);
        }

        for (index, offset) in offsets.iter().enumerate() {
            let start = offsets_pos + index * 2;
            subtable[start..start + 2].copy_from_slice(&offset.to_be_bytes());
        }

        build_lookup_record(LookupType::ContextSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn class_def_format1_table(start_glyph_id: u16, class_values: &[u16]) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, start_glyph_id);
        push_u16(&mut buffer, class_values.len() as u16);
        for class_value in class_values {
            push_u16(&mut buffer, *class_value);
        }
        buffer
    }

    #[cfg(feature = "layout")]
    fn lookup_context_format1_record(
        coverage_glyph_id: u16,
        input_sequence: &[u16],
        lookup_indexes: &[u16],
    ) -> Vec<u8> {
        let coverage = coverage_table(&[coverage_glyph_id]);
        let mut rule = Vec::new();
        push_u16(&mut rule, (input_sequence.len() + 1) as u16);
        push_u16(&mut rule, lookup_indexes.len() as u16);
        for glyph_id in input_sequence {
            push_u16(&mut rule, *glyph_id);
        }
        for lookup_index in lookup_indexes {
            push_u16(&mut rule, *lookup_index);
        }

        let mut rule_set = Vec::new();
        push_u16(&mut rule_set, 1);
        push_u16(&mut rule_set, 4);
        rule_set.extend_from_slice(&rule);

        let rule_set_offset = 8u16;
        let coverage_offset = rule_set_offset + rule_set.len() as u16;
        let mut subtable = Vec::new();
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, coverage_offset);
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, rule_set_offset);
        subtable.extend_from_slice(&rule_set);
        subtable.extend_from_slice(&coverage);

        build_lookup_record(LookupType::ContextSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn lookup_context_format2_record(
        coverage_glyph_id: u16,
        class_values: &[u16],
        input_classes: &[u16],
        sequence_index: u16,
        lookup_list_index: u16,
    ) -> Vec<u8> {
        let coverage = coverage_table(&[coverage_glyph_id]);
        let class_def = class_def_format1_table(coverage_glyph_id, class_values);

        let mut rule = Vec::new();
        push_u16(&mut rule, (input_classes.len() + 1) as u16);
        push_u16(&mut rule, 1);
        for class_id in input_classes {
            push_u16(&mut rule, *class_id);
        }
        push_u16(&mut rule, sequence_index);
        push_u16(&mut rule, lookup_list_index);

        let mut empty_rule_set = Vec::new();
        push_u16(&mut empty_rule_set, 0);

        let mut active_rule_set = Vec::new();
        push_u16(&mut active_rule_set, 1);
        push_u16(&mut active_rule_set, 4);
        active_rule_set.extend_from_slice(&rule);

        let mut subtable = Vec::new();
        push_u16(&mut subtable, 2);
        let coverage_offset_pos = subtable.len();
        push_u16(&mut subtable, 0);
        let class_def_offset_pos = subtable.len();
        push_u16(&mut subtable, 0);
        push_u16(&mut subtable, 3);
        let class_set_offsets_pos = subtable.len();
        subtable.resize(subtable.len() + 3 * 2, 0);
        let class0_offset = subtable.len() as u16;
        subtable.extend_from_slice(&empty_rule_set);
        let class1_offset = subtable.len() as u16;
        subtable.extend_from_slice(&active_rule_set);
        let class2_offset = subtable.len() as u16;
        subtable.extend_from_slice(&empty_rule_set);
        let class_def_offset = subtable.len() as u16;
        subtable.extend_from_slice(&class_def);
        let coverage_offset = subtable.len() as u16;
        subtable.extend_from_slice(&coverage);

        subtable[coverage_offset_pos..coverage_offset_pos + 2]
            .copy_from_slice(&coverage_offset.to_be_bytes());
        subtable[class_def_offset_pos..class_def_offset_pos + 2]
            .copy_from_slice(&class_def_offset.to_be_bytes());
        subtable[class_set_offsets_pos..class_set_offsets_pos + 2]
            .copy_from_slice(&class0_offset.to_be_bytes());
        subtable[class_set_offsets_pos + 2..class_set_offsets_pos + 4]
            .copy_from_slice(&class1_offset.to_be_bytes());
        subtable[class_set_offsets_pos + 4..class_set_offsets_pos + 6]
            .copy_from_slice(&class2_offset.to_be_bytes());

        build_lookup_record(LookupType::ContextSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn lookup_chaining_context_format3_record(
        backtrack_coverages: &[Vec<u8>],
        input_coverages: &[Vec<u8>],
        lookahead_coverages: &[Vec<u8>],
        sequence_index: u16,
        lookup_list_index: u16,
    ) -> Vec<u8> {
        let mut subtable = Vec::new();
        push_u16(&mut subtable, 3);
        push_u16(&mut subtable, backtrack_coverages.len() as u16);

        let backtrack_offsets_pos = subtable.len();
        subtable.resize(subtable.len() + backtrack_coverages.len() * 2, 0);

        push_u16(&mut subtable, input_coverages.len() as u16);
        let input_offsets_pos = subtable.len();
        subtable.resize(subtable.len() + input_coverages.len() * 2, 0);

        push_u16(&mut subtable, lookahead_coverages.len() as u16);
        let lookahead_offsets_pos = subtable.len();
        subtable.resize(subtable.len() + lookahead_coverages.len() * 2, 0);

        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, sequence_index);
        push_u16(&mut subtable, lookup_list_index);

        let mut backtrack_offsets = Vec::new();
        for coverage in backtrack_coverages {
            backtrack_offsets.push(subtable.len() as u16);
            subtable.extend_from_slice(coverage);
        }
        let mut input_offsets = Vec::new();
        for coverage in input_coverages {
            input_offsets.push(subtable.len() as u16);
            subtable.extend_from_slice(coverage);
        }
        let mut lookahead_offsets = Vec::new();
        for coverage in lookahead_coverages {
            lookahead_offsets.push(subtable.len() as u16);
            subtable.extend_from_slice(coverage);
        }

        for (index, offset) in backtrack_offsets.iter().enumerate() {
            let start = backtrack_offsets_pos + index * 2;
            subtable[start..start + 2].copy_from_slice(&offset.to_be_bytes());
        }
        for (index, offset) in input_offsets.iter().enumerate() {
            let start = input_offsets_pos + index * 2;
            subtable[start..start + 2].copy_from_slice(&offset.to_be_bytes());
        }
        for (index, offset) in lookahead_offsets.iter().enumerate() {
            let start = lookahead_offsets_pos + index * 2;
            subtable[start..start + 2].copy_from_slice(&offset.to_be_bytes());
        }

        build_lookup_record(LookupType::ChainingContextSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn lookup_chaining_context_format1_record(
        coverage_glyph_id: u16,
        backtrack_glyph_ids: &[u16],
        input_glyph_ids: &[u16],
        lookahead_glyph_ids: &[u16],
        lookup_indexes: &[u16],
    ) -> Vec<u8> {
        let coverage = coverage_table(&[coverage_glyph_id]);
        let mut rule = Vec::new();
        push_u16(&mut rule, backtrack_glyph_ids.len() as u16);
        for glyph_id in backtrack_glyph_ids {
            push_u16(&mut rule, *glyph_id);
        }
        push_u16(&mut rule, input_glyph_ids.len() as u16);
        for glyph_id in input_glyph_ids {
            push_u16(&mut rule, *glyph_id);
        }
        push_u16(&mut rule, lookahead_glyph_ids.len() as u16);
        for glyph_id in lookahead_glyph_ids {
            push_u16(&mut rule, *glyph_id);
        }
        push_u16(&mut rule, lookup_indexes.len() as u16);
        for lookup_index in lookup_indexes {
            push_u16(&mut rule, *lookup_index);
        }

        let mut rule_set = Vec::new();
        push_u16(&mut rule_set, 1);
        push_u16(&mut rule_set, 4);
        rule_set.extend_from_slice(&rule);

        let rule_set_offset = 8u16;
        let coverage_offset = rule_set_offset + rule_set.len() as u16;
        let mut subtable = Vec::new();
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, coverage_offset);
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, rule_set_offset);
        subtable.extend_from_slice(&rule_set);
        subtable.extend_from_slice(&coverage);

        build_lookup_record(LookupType::ChainingContextSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn lookup_chaining_context_format2_record(
        coverage_glyph_id: u16,
        backtrack_classes: &[u16],
        input_classes: &[u16],
        lookahead_classes: &[u16],
        sequence_index: u16,
        lookup_list_index: u16,
    ) -> Vec<u8> {
        let coverage = coverage_table(&[coverage_glyph_id]);
        let backtrack_class_def =
            class_def_format1_table(coverage_glyph_id - 1, &[backtrack_classes[0]]);
        let input_class_def = class_def_format1_table(coverage_glyph_id, &[1, input_classes[0]]);
        let lookahead_class_def =
            class_def_format1_table(coverage_glyph_id + 2, &[lookahead_classes[0]]);

        let mut rule = Vec::new();
        push_u16(&mut rule, backtrack_classes.len() as u16);
        for class_id in backtrack_classes {
            push_u16(&mut rule, *class_id);
        }
        push_u16(&mut rule, (input_classes.len() + 1) as u16);
        for class_id in input_classes {
            push_u16(&mut rule, *class_id);
        }
        push_u16(&mut rule, lookahead_classes.len() as u16);
        for class_id in lookahead_classes {
            push_u16(&mut rule, *class_id);
        }
        push_u16(&mut rule, 1);
        push_u16(&mut rule, sequence_index);
        push_u16(&mut rule, lookup_list_index);

        let mut empty_rule_set = Vec::new();
        push_u16(&mut empty_rule_set, 0);

        let mut active_rule_set = Vec::new();
        push_u16(&mut active_rule_set, 1);
        push_u16(&mut active_rule_set, 4);
        active_rule_set.extend_from_slice(&rule);

        let mut subtable = Vec::new();
        push_u16(&mut subtable, 2);
        let coverage_offset_pos = subtable.len();
        push_u16(&mut subtable, 0);
        let backtrack_class_def_offset_pos = subtable.len();
        push_u16(&mut subtable, 0);
        let input_class_def_offset_pos = subtable.len();
        push_u16(&mut subtable, 0);
        let lookahead_class_def_offset_pos = subtable.len();
        push_u16(&mut subtable, 0);
        push_u16(&mut subtable, 3);
        let class_set_offsets_pos = subtable.len();
        subtable.resize(subtable.len() + 3 * 2, 0);
        let class0_offset = subtable.len() as u16;
        subtable.extend_from_slice(&empty_rule_set);
        let class1_offset = subtable.len() as u16;
        subtable.extend_from_slice(&active_rule_set);
        let class2_offset = subtable.len() as u16;
        subtable.extend_from_slice(&empty_rule_set);
        let backtrack_class_def_offset = subtable.len() as u16;
        subtable.extend_from_slice(&backtrack_class_def);
        let input_class_def_offset = subtable.len() as u16;
        subtable.extend_from_slice(&input_class_def);
        let lookahead_class_def_offset = subtable.len() as u16;
        subtable.extend_from_slice(&lookahead_class_def);
        let coverage_offset = subtable.len() as u16;
        subtable.extend_from_slice(&coverage);

        subtable[coverage_offset_pos..coverage_offset_pos + 2]
            .copy_from_slice(&coverage_offset.to_be_bytes());
        subtable[backtrack_class_def_offset_pos..backtrack_class_def_offset_pos + 2]
            .copy_from_slice(&backtrack_class_def_offset.to_be_bytes());
        subtable[input_class_def_offset_pos..input_class_def_offset_pos + 2]
            .copy_from_slice(&input_class_def_offset.to_be_bytes());
        subtable[lookahead_class_def_offset_pos..lookahead_class_def_offset_pos + 2]
            .copy_from_slice(&lookahead_class_def_offset.to_be_bytes());
        subtable[class_set_offsets_pos..class_set_offsets_pos + 2]
            .copy_from_slice(&class0_offset.to_be_bytes());
        subtable[class_set_offsets_pos + 2..class_set_offsets_pos + 4]
            .copy_from_slice(&class1_offset.to_be_bytes());
        subtable[class_set_offsets_pos + 4..class_set_offsets_pos + 6]
            .copy_from_slice(&class2_offset.to_be_bytes());

        build_lookup_record(LookupType::ChainingContextSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn lookup_multiple_record(glyph_id: u16, substitute_glyph_ids: &[u16]) -> Vec<u8> {
        let coverage = coverage_table(&[glyph_id]);
        let mut sequence = Vec::new();
        push_u16(&mut sequence, substitute_glyph_ids.len() as u16);
        for substitute_glyph_id in substitute_glyph_ids {
            push_u16(&mut sequence, *substitute_glyph_id);
        }

        let sequence_offset = 8u16;
        let coverage_offset = sequence_offset + sequence.len() as u16;
        let mut subtable = Vec::new();
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, coverage_offset);
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, sequence_offset);
        subtable.extend_from_slice(&sequence);
        subtable.extend_from_slice(&coverage);
        build_lookup_record(LookupType::MultipleSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn lookup_single_record(glyph_id: u16, substitute_glyph_id: u16) -> Vec<u8> {
        let coverage = coverage_table(&[glyph_id]);
        let mut subtable = Vec::new();
        push_u16(&mut subtable, 2);
        push_u16(&mut subtable, 8);
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, substitute_glyph_id);
        subtable.extend_from_slice(&coverage);
        build_lookup_record(LookupType::SingleSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn lookup_ligature_record(
        glyph_id: u16,
        component_glyph_ids: &[u16],
        ligature_glyph: u16,
    ) -> Vec<u8> {
        let coverage = coverage_table(&[glyph_id]);
        let mut ligature_table = Vec::new();
        push_u16(&mut ligature_table, ligature_glyph);
        push_u16(&mut ligature_table, (component_glyph_ids.len() + 1) as u16);
        for component_glyph_id in component_glyph_ids {
            push_u16(&mut ligature_table, *component_glyph_id);
        }

        let mut ligature_set = Vec::new();
        push_u16(&mut ligature_set, 1);
        push_u16(&mut ligature_set, 4);
        ligature_set.extend_from_slice(&ligature_table);

        let ligature_set_offset = 8u16;
        let coverage_offset = ligature_set_offset + ligature_set.len() as u16;
        let mut subtable = Vec::new();
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, coverage_offset);
        push_u16(&mut subtable, 1);
        push_u16(&mut subtable, ligature_set_offset);
        subtable.extend_from_slice(&ligature_set);
        subtable.extend_from_slice(&coverage);
        build_lookup_record(LookupType::LigatureSubstitution as u16, subtable)
    }

    #[cfg(feature = "layout")]
    fn parse_gsub(buffer: Vec<u8>) -> crate::opentype::extentions::gsub::GSUB {
        let mut reader = BytesReader::new(&buffer);
        crate::opentype::extentions::gsub::GSUB::new(&mut reader, 0, buffer.len() as u32).unwrap()
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_extension_and_reverse_chain_parse_and_resolve() {
        let lookup_list = parse_lookup_list(vec![
            lookup_extension_subtable(0x0041, 4),
            lookup_reverse_chain_subtable(0x0042, 0x0201, 0x0030, 0x0043, 0x0044),
        ]);

        match &lookup_list.lookups[0].subtables[0] {
            LookupSubstitution::ExtensionSubstitution(extension) => {
                assert_eq!(extension.subst_format, 1);
                assert_eq!(
                    extension.extension_lookup_type,
                    LookupType::SingleSubstitution as u16
                );
                assert_eq!(extension.extension_offset, 8);
                match extension.subtable.as_ref() {
                    LookupSubstitution::Single(single) => {
                        assert_eq!(single.delta_glyph_id, 4);
                        assert_eq!(single.coverage.contains(0x0041), Some(0));
                    }
                    _ => panic!("expected nested single substitution"),
                }
                match extension.subtable.get_lookup(0x0041) {
                    LookupResult::Single(glyph_id) => assert_eq!(glyph_id, 4),
                    _ => panic!("expected single result"),
                }
            }
            _ => panic!("expected extension substitution"),
        }

        match &lookup_list.lookups[1].subtables[0] {
            LookupSubstitution::ReverseChainSingle(reverse) => {
                assert_eq!(reverse.subst_format, 1);
                assert_eq!(reverse.coverage.contains(0x0042), Some(0));
                assert_eq!(reverse.backtrack_glyph_ids, vec![0x0030]);
                assert_eq!(reverse.input_glyph_ids, vec![0x0043]);
                assert_eq!(reverse.lookahead_glyph_ids, vec![0x0044]);
                match lookup_list.lookups[1].subtables[0].get_lookup(0x0042) {
                    LookupResult::Single(glyph_id) => assert_eq!(glyph_id, 0x0201),
                    _ => panic!("expected single result"),
                }
                match lookup_list.lookups[1].subtables[0].get_lookup(0x0041) {
                    LookupResult::None => {}
                    _ => panic!("expected no result"),
                }
            }
            _ => panic!("expected reverse chain substitution"),
        }
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gpos_pair_adjustment_format1_parses_and_resolves() {
        let gpos = parse_gpos(build_gpos_table(
            *b"kern",
            2,
            build_gpos_pair_format1_subtable(10, 20, -50),
        ));

        let adjustment = gpos
            .lookup_pair_adjustment(10, 20, false, None)
            .expect("pair adjustment");
        assert_eq!(adjustment.first.x_advance, -50);
        assert_eq!(adjustment.second.x_advance, 0);
        assert!(gpos.lookup_pair_adjustment(10, 21, false, None).is_none());
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gpos_pair_adjustment_format2_parses_and_resolves() {
        let gpos = parse_gpos(build_gpos_table(
            *b"kern",
            2,
            build_gpos_pair_format2_subtable(30, 40, -80),
        ));

        let adjustment = gpos
            .lookup_pair_adjustment(30, 40, false, None)
            .expect("class pair adjustment");
        assert_eq!(adjustment.first.x_advance, -80);
        assert!(gpos.lookup_pair_adjustment(31, 40, false, None).is_none());
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gpos_locale_specific_script_and_required_feature_take_priority_over_dflt() {
        let gpos = parse_gpos(build_gpos_table_with_scripted_features(
            &[(*b"DFLT", 0xFFFF, &[0]), (*b"arab", 1, &[])],
            &[(*b"kern", &[0]), (*b"kern", &[1])],
            2,
            vec![
                build_gpos_pair_format1_subtable(10, 20, -10),
                build_gpos_pair_format1_subtable(10, 20, -30),
            ],
        ));

        let default_adjustment = gpos
            .lookup_pair_adjustment(10, 20, false, Some("default"))
            .expect("default pair adjustment");
        assert_eq!(default_adjustment.first.x_advance, -10);

        let arabic_adjustment = gpos
            .lookup_pair_adjustment(10, 20, false, Some("ar"))
            .expect("arabic pair adjustment");
        assert_eq!(arabic_adjustment.first.x_advance, -30);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_lookup_once_supports_multiple_and_ligature_sequences() {
        let multiple_lookup = Lookup {
            lookup_type: LookupType::MultipleSubstitution as u16,
            lookup_flag: 0,
            subtables: vec![LookupSubstitution::Multiple(MultipleSubstitutionFormat1 {
                subst_format: 1,
                coverage: coverage_format1(&[10]),
                sequence_count: 1,
                sequence_tables: vec![SequenceTable {
                    glyph_count: 2,
                    substitute_glyph_ids: vec![20, 21],
                }],
            })],
        };
        let ligature_lookup = Lookup {
            lookup_type: LookupType::LigatureSubstitution as u16,
            lookup_flag: 0,
            subtables: vec![LookupSubstitution::Ligature(LigatureSubstitutionFormat1 {
                subst_format: 1,
                coverage: coverage_format1(&[20]),
                ligature_set_count: 1,
                ligature_set: vec![LigatureSet {
                    ligature_count: 1,
                    ligature_table: vec![LigatureTable {
                        ligature_glyph: 99,
                        component_count: 2,
                        component_glyph_ids: vec![21],
                    }],
                }],
            })],
        };

        let mut glyphs = vec![(10usize, 0usize)];
        assert!(crate::opentype::extentions::gsub::GSUB::apply_lookup_once(
            &multiple_lookup,
            &mut glyphs,
        ));
        assert_eq!(glyphs, vec![(20, 0), (21, 0)]);

        assert!(crate::opentype::extentions::gsub::GSUB::apply_lookup_once(
            &ligature_lookup,
            &mut glyphs,
        ));
        assert_eq!(glyphs, vec![(99, 0)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_ccmp_sequence_supports_multiple_then_ligature() {
        let gsub = parse_gsub(build_gsub_table(
            *b"ccmp",
            vec![
                lookup_multiple_record(10, &[20, 21]),
                lookup_ligature_record(20, &[21], 99),
            ],
        ));
        let mut glyphs = vec![(10usize, 0usize)];

        gsub.apply_ccmp_sequence(&mut glyphs);

        assert_eq!(glyphs, vec![(99, 0)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_feature_sequence_supports_context_format3() {
        let gsub = parse_gsub(build_gsub_table_with_feature_lookups(
            *b"calt",
            &[0],
            vec![
                lookup_context_format3_record(
                    &[coverage_table(&[10]), coverage_table(&[11])],
                    1,
                    1,
                ),
                lookup_single_record(11, 77),
            ],
        ));
        let mut glyphs = vec![(10usize, 0usize), (11usize, 1usize)];

        gsub.apply_feature_sequence(&mut glyphs, None, &[*b"calt"]);

        assert_eq!(glyphs, vec![(10, 0), (77, 1)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_feature_sequence_supports_context_format1_and_format2() {
        let gsub = parse_gsub(build_gsub_table_with_feature_lookups(
            *b"calt",
            &[0, 1],
            vec![
                lookup_context_format1_record(10, &[11], &[2]),
                lookup_context_format2_record(20, &[1, 2], &[2], 1, 3),
                lookup_single_record(10, 70),
                lookup_single_record(21, 99),
            ],
        ));

        let mut format1_glyphs = vec![(10usize, 0usize), (11usize, 1usize)];
        gsub.apply_feature_sequence(&mut format1_glyphs, None, &[*b"calt"]);
        assert_eq!(format1_glyphs, vec![(70, 0), (11, 1)]);

        let mut format2_glyphs = vec![(20usize, 0usize), (21usize, 1usize)];
        gsub.apply_feature_sequence(&mut format2_glyphs, None, &[*b"calt"]);
        assert_eq!(format2_glyphs, vec![(20, 0), (99, 1)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_feature_sequence_supports_chaining_context_format3() {
        let gsub = parse_gsub(build_gsub_table_with_feature_lookups(
            *b"calt",
            &[0],
            vec![
                lookup_chaining_context_format3_record(
                    &[coverage_table(&[10])],
                    &[coverage_table(&[11])],
                    &[coverage_table(&[12])],
                    0,
                    1,
                ),
                lookup_single_record(11, 88),
            ],
        ));
        let mut glyphs = vec![(10usize, 0usize), (11usize, 1usize), (12usize, 2usize)];

        gsub.apply_feature_sequence(&mut glyphs, None, &[*b"calt"]);

        assert_eq!(glyphs, vec![(10, 0), (88, 1), (12, 2)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_feature_sequence_supports_chaining_context_format1() {
        let gsub = parse_gsub(build_gsub_table_with_feature_lookups(
            *b"calt",
            &[0],
            vec![
                lookup_chaining_context_format1_record(11, &[10], &[], &[12], &[1]),
                lookup_single_record(11, 66),
            ],
        ));
        let mut glyphs = vec![(10usize, 0usize), (11usize, 1usize), (12usize, 2usize)];

        gsub.apply_feature_sequence(&mut glyphs, None, &[*b"calt"]);

        assert_eq!(glyphs, vec![(10, 0), (66, 1), (12, 2)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_feature_sequence_supports_chaining_context_format2() {
        let gsub = parse_gsub(build_gsub_table_with_feature_lookups(
            *b"calt",
            &[0],
            vec![
                lookup_chaining_context_format2_record(20, &[1], &[2], &[1], 1, 1),
                lookup_single_record(21, 123),
            ],
        ));
        let mut glyphs = vec![
            (19usize, 0usize),
            (20usize, 1usize),
            (21usize, 2usize),
            (22usize, 3usize),
        ];

        gsub.apply_feature_sequence(&mut glyphs, None, &[*b"calt"]);

        assert_eq!(glyphs, vec![(19, 0), (20, 1), (123, 2), (22, 3)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_rtl_contextual_sequence_supports_rclt() {
        let gsub = parse_gsub(build_gsub_table_with_feature_lookups(
            *b"rclt",
            &[0],
            vec![
                lookup_context_format3_record(
                    &[coverage_table(&[10]), coverage_table(&[11])],
                    1,
                    1,
                ),
                lookup_single_record(11, 144),
            ],
        ));
        let mut glyphs = vec![(10usize, 0usize), (11usize, 1usize)];

        gsub.apply_rtl_contextual_sequence(&mut glyphs, None);

        assert_eq!(glyphs, vec![(10, 0), (144, 1)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_apply_rtl_contextual_sequence_supports_clig() {
        let gsub = parse_gsub(build_gsub_table_with_feature_lookups(
            *b"clig",
            &[0],
            vec![lookup_ligature_record(20, &[21], 220)],
        ));
        let mut glyphs = vec![(20usize, 0usize), (21usize, 1usize)];

        gsub.apply_rtl_contextual_sequence(&mut glyphs, None);

        assert_eq!(glyphs, vec![(220, 0)]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_locale_specific_script_lookups_take_priority_over_dflt() {
        let gsub = parse_gsub(build_gsub_table_with_scripted_feature_lookups(
            &[(*b"DFLT", &[0]), (*b"arab", &[1])],
            *b"isol",
            &[vec![0], vec![1]],
            vec![lookup_single_record(10, 100), lookup_single_record(10, 200)],
        ));

        let default_forms = gsub.lookup_joining_forms(10, None);
        assert_eq!(default_forms.isolated, Some(100));

        let arabic_forms = gsub.lookup_joining_forms(10, Some("ar"));
        assert_eq!(arabic_forms.isolated, Some(200));
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_required_feature_is_applied_for_locale_specific_script() {
        let gsub = parse_gsub(build_gsub_table_with_scripted_features(
            &[(*b"DFLT", 0xFFFF, &[0]), (*b"arab", 1, &[])],
            &[(*b"isol", &[0]), (*b"isol", &[1])],
            vec![lookup_single_record(10, 100), lookup_single_record(10, 200)],
        ));

        let default_forms = gsub.lookup_joining_forms(10, None);
        assert_eq!(default_forms.isolated, Some(100));

        let arabic_forms = gsub.lookup_joining_forms(10, Some("ar"));
        assert_eq!(arabic_forms.isolated, Some(200));
    }

    #[test]
    #[cfg(feature = "layout")]
    fn gsub_language_specific_lookup_uses_full_locale_subtags() {
        let script_list = build_script_list_with_language_systems(&[(
            *b"arab",
            &[
                (0u32, 0xFFFF, &[0][..]),
                (u32::from_be_bytes(*b"URD "), 0xFFFF, &[1][..]),
            ],
        )]);
        let feature_list = build_feature_list_with_entries(&[(*b"isol", &[0]), (*b"isol", &[1])]);
        let lookup_list = build_lookup_list(vec![
            lookup_single_record(10, 100),
            lookup_single_record(10, 300),
        ]);

        let script_list_offset = 10u16;
        let feature_list_offset = script_list_offset + script_list.len() as u16;
        let lookup_list_offset = feature_list_offset + feature_list.len() as u16;

        let mut buffer = Vec::new();
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, script_list_offset);
        push_u16(&mut buffer, feature_list_offset);
        push_u16(&mut buffer, lookup_list_offset);
        buffer.extend_from_slice(&script_list);
        buffer.extend_from_slice(&feature_list);
        buffer.extend_from_slice(&lookup_list);

        let gsub = parse_gsub(buffer);

        let default_forms = gsub.lookup_joining_forms(10, Some("ar"));
        assert_eq!(default_forms.isolated, Some(100));

        let urdu_forms = gsub.lookup_joining_forms(10, Some("ur-Arab-PK"));
        assert_eq!(urdu_forms.isolated, Some(300));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn fontload_from_net_works() {
        let path = sample_font_path();
        let bytes = std::fs::read(&path).expect("read font bytes");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind tcp listener");
        let addr = listener.local_addr().expect("local addr");

        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().expect("accept");
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            loop {
                let read = std::io::Read::read(&mut socket, &mut buf).expect("read request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            std::io::Write::write_all(&mut socket, response.as_bytes()).expect("write header");
            std::io::Write::write_all(&mut socket, &bytes).expect("write body");
        });

        let url = format!("http://127.0.0.1:{}/font.ttf", addr.port());
        let font = crate::load_font_from_net(&url).expect("load from net");
        assert!(font.font().get_font_count() >= 1);

        server.join().expect("server thread");
    }

    #[test]
    fn emoji_font_renders_svg() {
        let path = test_fonts_dir().join("NotoColorEmoji-Regular.ttf");
        let font = crate::fontload_file(&path).expect("load emoji font");
        let svg = font.font().get_svg('😀', 32.0, "px").expect("emoji svg");
        assert!(svg.contains("<svg"));
    }

    #[test]
    #[cfg(feature = "layout")]
    fn ligature_lookup_returns_multiple_alternatives() {
        let ligature = LookupSubstitution::Ligature(LigatureSubstitutionFormat1 {
            subst_format: 1,
            coverage: coverage_format1(&[0x0066]),
            ligature_set_count: 1,
            ligature_set: vec![LigatureSet {
                ligature_count: 2,
                ligature_table: vec![
                    LigatureTable {
                        ligature_glyph: 0xfb01,
                        component_count: 2,
                        component_glyph_ids: vec![0x0069],
                    },
                    LigatureTable {
                        ligature_glyph: 0xfb02,
                        component_count: 2,
                        component_glyph_ids: vec![0x006c],
                    },
                ],
            }],
        });

        match ligature.get_lookup(0x0066) {
            LookupResult::Ligature(records) => {
                assert_eq!(records.len(), 2);
                assert_eq!(records[0].ligature_glyph, 0xfb01);
                assert_eq!(records[1].ligature_glyph, 0xfb02);
            }
            _ => panic!("unexpected lookup result"),
        }
    }

    use crate::opentype::requires::cmap::{self, CmapEncodings, CmapSubtable, EncodingRecord};
    use bin_rs::reader::BytesReader;

    fn push_u16(buffer: &mut Vec<u8>, value: u16) {
        buffer.extend_from_slice(&value.to_be_bytes());
    }

    fn push_u24(buffer: &mut Vec<u8>, value: u32) {
        buffer.push(((value >> 16) & 0xFF) as u8);
        buffer.push(((value >> 8) & 0xFF) as u8);
        buffer.push((value & 0xFF) as u8);
    }

    fn push_u32(buffer: &mut Vec<u8>, value: u32) {
        buffer.extend_from_slice(&value.to_be_bytes());
    }

    fn build_cmap(records: Vec<(u16, u16, Vec<u8>)>) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, records.len() as u16);

        let header_len = 4 + records.len() * 8;
        let mut offset = header_len as u32;
        let mut tables = Vec::new();

        for (platform_id, encoding_id, table) in records {
            push_u16(&mut buffer, platform_id);
            push_u16(&mut buffer, encoding_id);
            push_u32(&mut buffer, offset);
            offset += table.len() as u32;
            tables.push(table);
        }

        for table in tables {
            buffer.extend_from_slice(&table);
        }

        buffer
    }

    fn cmap_encodings(records: Vec<(u16, u16, Vec<u8>)>) -> CmapEncodings {
        let buffer = build_cmap(records);
        let mut reader = BytesReader::new(&buffer);
        CmapEncodings::new(&mut reader, 0, buffer.len() as u32).unwrap()
    }

    fn cmap_subtable(table: Vec<u8>) -> CmapSubtable {
        let record = EncodingRecord {
            platform_id: 3,
            encoding_id: 1,
            subtable_offset: 0,
        };
        let buffer = table;
        cmap::get_subtable(&record, &buffer)
    }

    fn format0_table() -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, 262);
        push_u16(&mut buffer, 0);
        for gid in 0u8..=255 {
            buffer.push(gid);
        }
        buffer
    }

    fn format2_table() -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 2);
        push_u16(&mut buffer, 2566);
        push_u16(&mut buffer, 0);
        for _ in 0..256 {
            push_u16(&mut buffer, 0);
        }
        for _ in 0..256 {
            push_u16(&mut buffer, 0);
            push_u16(&mut buffer, 0);
            push_u16(&mut buffer, 0);
            push_u16(&mut buffer, 0);
        }
        buffer
    }

    fn format4_table(
        start_code: u16,
        end_code: u16,
        delta: i16,
        range_offset: u16,
        glyphs: &[u16],
    ) -> Vec<u8> {
        let seg_count = 2u16;
        let seg_count_x2 = seg_count * 2;
        let search_range = 4u16;
        let entry_selector = 1u16;
        let range_shift = 0u16;
        let length = 16 + 8 * seg_count as usize + glyphs.len() * 2;

        let mut buffer = Vec::new();
        push_u16(&mut buffer, 4);
        push_u16(&mut buffer, length as u16);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, seg_count_x2);
        push_u16(&mut buffer, search_range);
        push_u16(&mut buffer, entry_selector);
        push_u16(&mut buffer, range_shift);
        push_u16(&mut buffer, end_code);
        push_u16(&mut buffer, 0xFFFF);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, start_code);
        push_u16(&mut buffer, 0xFFFF);
        push_u16(&mut buffer, delta as u16);
        push_u16(&mut buffer, 1);
        push_u16(&mut buffer, range_offset);
        push_u16(&mut buffer, 0);
        for glyph_id in glyphs {
            push_u16(&mut buffer, *glyph_id);
        }
        buffer
    }

    fn format6_table(first_code: u16, glyphs: &[u16]) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 6);
        push_u16(&mut buffer, (10 + glyphs.len() * 2) as u16);
        push_u16(&mut buffer, 0);
        push_u16(&mut buffer, first_code);
        push_u16(&mut buffer, glyphs.len() as u16);
        for glyph_id in glyphs {
            push_u16(&mut buffer, *glyph_id);
        }
        buffer
    }

    fn format8_table() -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 8);
        push_u16(&mut buffer, 0);
        push_u32(&mut buffer, 8220);
        push_u32(&mut buffer, 0);
        buffer.extend_from_slice(&[0u8; 8192]);
        push_u32(&mut buffer, 1);
        push_u32(&mut buffer, 0x0001_F600);
        push_u32(&mut buffer, 0x0001_F600);
        push_u32(&mut buffer, 42);
        buffer
    }

    fn format10_table(start_char_code: u32, glyphs: &[u16]) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 10);
        push_u16(&mut buffer, 0);
        push_u32(&mut buffer, (20 + glyphs.len() * 2) as u32);
        push_u32(&mut buffer, 0);
        push_u32(&mut buffer, start_char_code);
        push_u32(&mut buffer, glyphs.len() as u32);
        for glyph_id in glyphs {
            push_u16(&mut buffer, *glyph_id);
        }
        buffer
    }

    fn format12_table(groups: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 12);
        push_u16(&mut buffer, 0);
        push_u32(&mut buffer, (16 + groups.len() * 12) as u32);
        push_u32(&mut buffer, 0);
        push_u32(&mut buffer, groups.len() as u32);
        for (start_char_code, end_char_code, start_glyph_id) in groups {
            push_u32(&mut buffer, *start_char_code);
            push_u32(&mut buffer, *end_char_code);
            push_u32(&mut buffer, *start_glyph_id);
        }
        buffer
    }

    fn format13_table(groups: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 13);
        push_u16(&mut buffer, 0);
        push_u32(&mut buffer, (16 + groups.len() * 12) as u32);
        push_u32(&mut buffer, 0);
        push_u32(&mut buffer, groups.len() as u32);
        for (start_char_code, end_char_code, glyph_id) in groups {
            push_u32(&mut buffer, *start_char_code);
            push_u32(&mut buffer, *end_char_code);
            push_u32(&mut buffer, *glyph_id);
        }
        buffer
    }

    fn format14_table(code: u32, selector: u32, glyph_id: u16) -> Vec<u8> {
        let mut buffer = Vec::new();
        push_u16(&mut buffer, 14);
        push_u32(&mut buffer, 0);
        push_u32(&mut buffer, 1);

        let records_start = 10 + 11;
        let non_default_offset = records_start as u32;
        push_u24(&mut buffer, selector);
        push_u32(&mut buffer, 0);
        push_u32(&mut buffer, non_default_offset);

        push_u32(&mut buffer, 1);
        push_u24(&mut buffer, code);
        push_u16(&mut buffer, glyph_id);

        let length = buffer.len() as u32;
        buffer[2..6].copy_from_slice(&length.to_be_bytes());
        buffer
    }

    #[test]
    fn cmap_format0_parses_byte_encoding() {
        let table = cmap_subtable(format0_table());
        assert_eq!(table.get_format(), 0);
        let text = table.get_part_of_string(4);
        assert!(text.contains("Format 0"));
        assert!(text.contains("glyph_id_array"));
    }

    #[test]
    fn cmap_format2_parses_high_byte_mapping() {
        let table = cmap_subtable(format2_table());
        match table {
            CmapSubtable::Format2(format2) => {
                assert_eq!(format2.format, 2);
                assert_eq!(format2.length, 2566);
                assert_eq!(format2.language, 0);
                assert_eq!(format2.sub_header_keys.len(), 256);
                assert_eq!(format2.sub_headers.len(), 256);
                assert!(format2.glyph_id_array.is_empty());
            }
            _ => panic!("expected format 2"),
        }
    }

    #[test]
    fn cmap_format4_single_substitution_uses_delta() {
        let cmap = cmap_encodings(vec![(3, 1, format4_table(0x0041, 0x0041, -60, 0, &[]))]);
        assert_eq!(cmap.get_glyph_position(0x0041), 5);
        assert_eq!(cmap.get_glyph_position(0x0042), 0);
    }

    #[test]
    fn cmap_format4_single_substitution_uses_glyph_array() {
        let cmap = cmap_encodings(vec![(3, 1, format4_table(0x0042, 0x0042, 0, 4, &[99]))]);
        assert_eq!(cmap.get_glyph_position(0x0042), 99);
    }

    #[test]
    fn cmap_format6_parses_trimmed_table() {
        let table = cmap_subtable(format6_table(0x0030, &[10, 11, 12]));
        match table {
            CmapSubtable::Format6(format6) => {
                assert_eq!(format6.format, 6);
                assert_eq!(format6.first_code, 0x0030);
                assert_eq!(format6.entry_count, 3);
                assert_eq!(format6.glyph_id_array, vec![10, 11, 12]);
            }
            _ => panic!("expected format 6"),
        }
    }

    #[test]
    fn cmap_format8_parses_mixed_coverage() {
        let table = cmap_subtable(format8_table());
        match table {
            CmapSubtable::Format8(format8) => {
                assert_eq!(format8.format, 8);
                assert_eq!(format8.reserved, 0);
                assert_eq!(format8.num_groups, 1);
                assert_eq!(format8.groups[0].start_char_code, 0x0001_F600);
                assert_eq!(format8.groups[0].start_glyph_id, 42);
            }
            _ => panic!("expected format 8"),
        }
    }

    #[test]
    fn cmap_format10_parses_trimmed_array() {
        let table = cmap_subtable(format10_table(0x0002_0000, &[7, 8, 9]));
        match table {
            CmapSubtable::Format10(format10) => {
                assert_eq!(format10.format, 10);
                assert_eq!(format10.start_char_code, 0x0002_0000);
                assert_eq!(format10.num_chars, 3);
                assert_eq!(format10.glyph_id_array, vec![7, 8, 9]);
            }
            _ => panic!("expected format 10"),
        }
    }

    #[test]
    fn cmap_format12_takes_priority_over_format4() {
        let cmap = cmap_encodings(vec![
            (3, 1, format4_table(0x0041, 0x0041, 4, 0, &[])),
            (3, 10, format12_table(&[(0x0041, 0x0041, 200)])),
        ]);
        assert_eq!(cmap.get_glyph_position(0x0041), 200);
    }

    #[test]
    fn cmap_format13_maps_ranges_to_constant_glyphs() {
        let cmap = cmap_encodings(vec![(3, 1, format13_table(&[(0x3400, 0x3402, 55)]))]);
        assert_eq!(cmap.get_glyph_position(0x3401), 55);
    }

    #[test]
    fn cmap_format14_resolves_unicode_variation_sequences() {
        let cmap = cmap_encodings(vec![
            (3, 1, format4_table(0x2764, 0x2764, 0, 4, &[20])),
            (0, 5, format14_table(0x2764, 0xFE0F, 77)),
        ]);
        assert_eq!(cmap.get_glyph_position(0x2764), 20);
        assert_eq!(cmap.get_glyph_position_from_uvs(0x2764, 0xFE0F), 77);
        assert_eq!(cmap.get_glyph_position_from_uvs(0x2764, 0xFE0E), 20);
    }

    fn test_fonts_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("_test_fonts")
    }

    fn sample_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("ZenMaruGothic-Regular.ttf")
    }

    fn woff2_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("notosanswoff2.woff2")
    }

    fn woff_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("MS-Gothic.ttf.woff")
    }

    fn svg_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("EmojiOneColor.otf")
    }

    fn fira_sans_black_path() -> std::path::PathBuf {
        test_fonts_dir()
            .join("Fira_Sans")
            .join("FiraSans-Black.ttf")
    }

    fn fira_sans_regular_path() -> std::path::PathBuf {
        test_fonts_dir()
            .join("Fira_Sans")
            .join("FiraSans-Regular.ttf")
    }

    fn segoe_emoji_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("windows").join("seguiemj.ttf")
    }

    fn noto_color_emoji_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("NotoColorEmoji-Regular.ttf")
    }

    fn twemoji_sbix_font_path() -> std::path::PathBuf {
        test_fonts_dir()
            .join("sbix")
            .join("TwemojiMozilla-sbix.woff2")
    }

    fn collection_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("windows").join("msgothic.ttc")
    }

    fn rtl_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("windows").join("arial.ttf")
    }

    #[cfg(feature = "layout")]
    fn rtl_contextual_font_paths() -> Vec<std::path::PathBuf> {
        vec![
            rtl_font_path(),
            test_fonts_dir()
                .join("Noto_Sans")
                .join("static")
                .join("NotoSans-Regular.ttf"),
        ]
    }

    #[cfg(feature = "layout")]
    fn collapse_ligatures_like_text_api(
        gsub: &crate::opentype::extentions::gsub::GSUB,
        glyphs: &[(usize, usize)],
        locale: Option<&str>,
        is_right_to_left: bool,
    ) -> Vec<usize> {
        const MAX_LIGATURE_COMPONENTS: usize = 8;

        let glyph_ids: Vec<usize> = glyphs.iter().map(|glyph| glyph.0).collect();
        let mut collapsed = Vec::new();
        let mut index = 0;

        while index < glyph_ids.len() {
            let max_len = (glyph_ids.len() - index).min(MAX_LIGATURE_COMPONENTS);
            let mut matched = None;
            for len in (2..=max_len).rev() {
                if is_right_to_left {
                    if let Some(glyph_id) =
                        gsub.lookup_rlig_sequence(&glyph_ids[index..index + len], locale)
                    {
                        matched = Some((glyph_id, len));
                        break;
                    }
                }
                if let Some(glyph_id) = gsub.lookup_liga_sequence(&glyph_ids[index..index + len]) {
                    matched = Some((glyph_id, len));
                    break;
                }
            }

            if let Some((glyph_id, len)) = matched {
                collapsed.push(glyph_id);
                index += len;
            } else {
                collapsed.push(glyph_ids[index]);
                index += 1;
            }
        }

        collapsed
    }

    #[cfg(feature = "layout")]
    fn first_real_arabic_joining_pair(font: &crate::LoadedFont) -> Option<(String, Vec<usize>)> {
        let gsub = font.font().gsub.as_ref()?;
        let cmap = font.font().cmap.as_ref()?;
        let candidates: Vec<char> = (0x0621u32..=0x064Au32)
            .filter_map(char::from_u32)
            .filter(|ch| cmap.get_glyph_position(*ch as u32) != 0)
            .collect();

        for left in candidates.iter().copied() {
            let left_glyph = cmap.get_glyph_position(left as u32) as usize;
            let left_forms = gsub.lookup_joining_forms(left_glyph, Some("ar"));
            if !left_forms.can_join_to_next() {
                continue;
            }

            for right in candidates.iter().copied() {
                let right_glyph = cmap.get_glyph_position(right as u32) as usize;
                let right_forms = gsub.lookup_joining_forms(right_glyph, Some("ar"));
                if !right_forms.can_join_to_prev() {
                    continue;
                }

                let expected = vec![
                    left_forms.substitute(left_glyph, false, true),
                    right_forms.substitute(right_glyph, true, false),
                ];
                if expected[0] != left_glyph || expected[1] != right_glyph {
                    return Some((format!("{left}{right}"), expected));
                }
            }
        }

        None
    }

    #[cfg(feature = "layout")]
    fn first_real_arabic_rlig_sequence(font: &crate::LoadedFont) -> Option<(String, usize)> {
        let gsub = font.font().gsub.as_ref()?;
        let cmap = font.font().cmap.as_ref()?;
        let candidates: Vec<char> = (0x0621u32..=0x064Au32)
            .filter_map(char::from_u32)
            .filter(|ch| cmap.get_glyph_position(*ch as u32) != 0)
            .collect();

        for left in candidates.iter().copied() {
            let left_glyph = cmap.get_glyph_position(left as u32) as usize;
            let left_forms = gsub.lookup_joining_forms(left_glyph, Some("ar"));
            for right in candidates.iter().copied() {
                let right_glyph = cmap.get_glyph_position(right as u32) as usize;
                let right_forms = gsub.lookup_joining_forms(right_glyph, Some("ar"));

                let joined = [
                    left_forms.substitute(
                        left_glyph,
                        false,
                        left_forms.can_join_to_next() && right_forms.can_join_to_prev(),
                    ),
                    right_forms.substitute(
                        right_glyph,
                        left_forms.can_join_to_next() && right_forms.can_join_to_prev(),
                        false,
                    ),
                ];

                if let Some(ligature) = gsub.lookup_rlig_sequence(&joined, Some("ar")) {
                    return Some((format!("{left}{right}"), ligature));
                }
            }
        }

        None
    }

    #[cfg(feature = "layout")]
    fn first_real_arabic_contextual_sequence_in_font(
        font: &crate::LoadedFont,
    ) -> Option<(String, Vec<usize>)> {
        let gsub = font.font().gsub.as_ref()?;
        let cmap = font.font().cmap.as_ref()?;
        let candidates: Vec<char> = (0x0621u32..=0x064Au32)
            .filter_map(char::from_u32)
            .filter(|ch| cmap.get_glyph_position(*ch as u32) != 0)
            .collect();

        let try_sequence = |chars: &[char]| -> Option<(String, Vec<usize>)> {
            let mut joined = chars
                .iter()
                .enumerate()
                .map(|(index, ch)| (cmap.get_glyph_position(*ch as u32) as usize, index))
                .collect::<Vec<_>>();
            gsub.apply_joining_sequence(&mut joined, Some("ar"));
            let baseline = joined.iter().map(|glyph| glyph.0).collect::<Vec<_>>();

            let mut contextual = joined.clone();
            gsub.apply_feature_sequence(&mut contextual, Some("ar"), &[*b"rclt", *b"calt"]);
            let contextual_ids = contextual.iter().map(|glyph| glyph.0).collect::<Vec<_>>();
            if contextual_ids == baseline || contextual_ids.len() != baseline.len() {
                return None;
            }

            let final_ids = collapse_ligatures_like_text_api(gsub, &contextual, Some("ar"), true);
            if final_ids == baseline {
                return None;
            }

            Some((chars.iter().collect::<String>(), final_ids))
        };

        for left in candidates.iter().copied() {
            for right in candidates.iter().copied() {
                if let Some(found) = try_sequence(&[left, right]) {
                    return Some(found);
                }
            }
        }

        for first in candidates.iter().copied() {
            for second in candidates.iter().copied() {
                for third in candidates.iter().copied() {
                    if let Some(found) = try_sequence(&[first, second, third]) {
                        return Some(found);
                    }
                }
            }
        }

        None
    }

    #[cfg(feature = "layout")]
    fn first_real_arabic_contextual_sequence() -> Option<(std::path::PathBuf, String, Vec<usize>)> {
        for path in rtl_contextual_font_paths() {
            if !path.exists() {
                continue;
            }
            let Ok(font) = crate::load_font_from_file(&path) else {
                continue;
            };
            if let Some((text, glyph_ids)) = first_real_arabic_contextual_sequence_in_font(&font) {
                return Some((path, text, glyph_ids));
            }
        }

        None
    }

    #[cfg(feature = "layout")]
    fn first_real_kern_pair(font: &crate::LoadedFont) -> Option<(char, char, i16)> {
        let gpos = font.font().gpos.as_ref()?;
        let cmap = font.font().cmap.as_ref()?;
        let candidates = "AVWToYLT.,abcdefghijklmnopqrstuvwxyz";

        for left in candidates.chars() {
            let left_glyph = cmap.get_glyph_position(left as u32) as u16;
            if left_glyph == 0 {
                continue;
            }
            for right in candidates.chars() {
                let right_glyph = cmap.get_glyph_position(right as u32) as u16;
                if right_glyph == 0 {
                    continue;
                }
                let Some(adjustment) =
                    gpos.lookup_pair_adjustment(left_glyph, right_glyph, false, None)
                else {
                    continue;
                };
                let total_advance = adjustment
                    .first
                    .x_advance
                    .saturating_add(adjustment.second.x_advance);
                if total_advance != 0 {
                    return Some((left, right, total_advance));
                }
            }
        }

        None
    }

    #[cfg(feature = "cff")]
    fn cff_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("NotoSansJP-Black.otf")
    }

    fn japanese_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("NotoSansJP-Regular.otf")
    }

    #[cfg(feature = "layout")]
    fn japanese_layout_font_paths() -> Vec<std::path::PathBuf> {
        vec![
            japanese_font_path(),
            test_fonts_dir().join("NotoSansCJK-Regular.ttc"),
            test_fonts_dir().join("windows").join("msgothic.ttc"),
            test_fonts_dir().join("windows").join("YuGothR.ttc"),
        ]
    }

    #[cfg(feature = "layout")]
    fn first_real_variant_substitution(
        font_variant: crate::FontVariant,
    ) -> Option<(std::path::PathBuf, char, usize, usize)> {
        for path in japanese_layout_font_paths() {
            if !path.exists() {
                continue;
            }

            let Ok(font) = crate::load_font_from_file(&path) else {
                continue;
            };
            let Some(cmap) = font.font().cmap.as_ref() else {
                continue;
            };
            let Some(gsub) = font.font().gsub.as_ref() else {
                continue;
            };

            for codepoint in 0x20u32..=0xFFFF {
                let Some(ch) = char::from_u32(codepoint) else {
                    continue;
                };
                if ch.is_control() {
                    continue;
                }

                let glyph_id = cmap.get_glyph_position(codepoint) as usize;
                if glyph_id == 0 {
                    continue;
                }

                let mut variant_glyphs = vec![(glyph_id, 0usize)];
                gsub.apply_variant_sequence(&mut variant_glyphs, Some("ja-JP"), font_variant);
                if let Some((variant_glyph_id, _)) = variant_glyphs.first().copied() {
                    if variant_glyph_id != glyph_id {
                        return Some((path, ch, glyph_id, variant_glyph_id));
                    }
                }
            }
        }

        None
    }

    #[cfg(feature = "layout")]
    fn first_real_vertical_substitution(font: &crate::LoadedFont) -> Option<(char, u16, u16)> {
        let gsub = font.font().gsub.as_ref()?;
        let cmap = font.font().cmap.as_ref()?;

        let candidates = [
            '(', ')', '[', ']', '{', '}', '!', '?', ',', '.', ':', ';', '、', '。', '「', '」',
            '（', '）', 'ー', '〜', '＜', '＞',
        ];

        for ch in candidates {
            let glyph_id = cmap.get_glyph_position(ch as u32) as u16;
            if glyph_id == 0 {
                continue;
            }

            let vertical = gsub.lookup_vertical(glyph_id).unwrap_or(glyph_id);
            if vertical != glyph_id {
                return Some((ch, glyph_id, vertical));
            }
        }

        None
    }

    fn legacy_variation_selector_font_candidates() -> Vec<std::path::PathBuf> {
        vec![
            test_fonts_dir().join("windows").join("msgothic.ttc"),
            test_fonts_dir().join("windows").join("msjh.ttc"),
            test_fonts_dir().join("windows").join("msyh.ttc"),
            test_fonts_dir().join("windows").join("YuGothR.ttc"),
            test_fonts_dir().join("ZenMaruGothic-Regular.ttf"),
        ]
    }

    fn first_real_sbix_font_path() -> Option<std::path::PathBuf> {
        let preferred_dir = test_fonts_dir().join("sbix");
        if let Ok(entries) = std::fs::read_dir(&preferred_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    return Some(path);
                }
            }
        }

        let mut stack = vec![test_fonts_dir()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
                    continue;
                };
                if !matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "ttf" | "otf" | "ttc" | "woff" | "woff2"
                ) {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                if bytes.windows(4).any(|window| window == b"sbix") {
                    return Some(path);
                }
            }
        }
        None
    }

    fn emoji_ligature_font_candidates() -> Vec<std::path::PathBuf> {
        vec![
            noto_color_emoji_font_path(),
            segoe_emoji_font_path(),
            twemoji_sbix_font_path(),
        ]
    }

    fn emoji_ligature_sequence_candidates() -> [&'static str; 7] {
        ["👩‍💻", "👨‍👩‍👧‍👦", "🏳️‍🌈", "❤️", "🇯🇵", "1️⃣", "👩🏽‍💻"]
    }

    fn first_real_emoji_ligature() -> Option<(std::path::PathBuf, &'static str)> {
        for path in emoji_ligature_font_candidates() {
            if !path.exists() {
                continue;
            }
            let Ok(font) = crate::load_font_from_file(&path) else {
                continue;
            };
            for sequence in emoji_ligature_sequence_candidates() {
                let Ok(glyph_ids) = font.font().debug_shape_glyph_ids(sequence, None) else {
                    continue;
                }; 
                if glyph_ids.len() != 1 || glyph_ids[0] == 0 {
                    continue;
                }
                let Ok(run) = font.text2glyph_run(
                    sequence,
                    crate::FontOptions::from_font_ref(crate::FontRef::Loaded(&font))
                        .with_font_size(32.0),
                ) else {
                    continue;
                };
                if run.glyphs.len() == 1 && !run.glyphs[0].glyph.layers.is_empty() {
                    return Some((path, sequence));
                }
            }
        }
        None
    }

    fn first_real_emoji_ligature_for_legacy() -> Option<(std::path::PathBuf, &'static str)> {
        for path in emoji_ligature_font_candidates() {
            if !path.exists() {
                continue;
            }
            let Ok(font) = crate::load_font_from_file(&path) else {
                continue;
            };
            for sequence in emoji_ligature_sequence_candidates() {
                let Ok(commands) = font.text2command(sequence) else {
                    continue;
                };
                if commands.len() == 1
                    && (commands[0].bitmap.is_some() || !commands[0].commands.is_empty())
                {
                    return Some((path, sequence));
                }
            }
        }
        None
    }

    fn first_truetype_variation_selector_font_path() -> Option<std::path::PathBuf> {
        for path in legacy_variation_selector_font_candidates() {
            if !path.exists() {
                continue;
            }
            let Ok(Ok(font)) = std::panic::catch_unwind(|| crate::load_font_from_file(&path))
            else {
                continue;
            };
            if font.font().glyf.is_none() {
                continue;
            }
            let Some(cmap) = font.font().cmap.as_ref() else {
                continue;
            };
            let has_format14 = cmap.cmap_encodings.iter().any(|encoding| {
                matches!(
                    encoding.cmap_subtable.as_ref(),
                    CmapSubtable::Format14(format14)
                        if !format14.var_selector_records.is_empty()
                )
            });
            if has_format14 {
                return Some(path);
            }
        }
        None
    }

    fn real_variation_sequence(font: &crate::LoadedFont) -> (String, usize) {
        let cmap = font.font().cmap.as_ref().expect("cmap");
        let format14 = cmap
            .cmap_encodings
            .iter()
            .find_map(|encoding| match encoding.cmap_subtable.as_ref() {
                CmapSubtable::Format14(format14) => Some(format14),
                _ => None,
            })
            .expect("expected format 14 cmap");
        let var_selector_record = format14
            .var_selector_records
            .first()
            .expect("expected at least one var selector record");
        let mapping = var_selector_record
            .non_default_uvs
            .unicode_value_ranges
            .first()
            .expect("expected at least one UVS mapping");
        let text = format!(
            "{}{}",
            char::from_u32(mapping.unicode_value).expect("unicode scalar"),
            char::from_u32(var_selector_record.var_selector).expect("variation selector")
        );
        (text, mapping.glyph_id as usize)
    }

    #[test]
    fn fontload_from_file_works() {
        let path = sample_font_path();
        let font = crate::fontload_file(&path).expect("load from file");
        assert!(font.font().get_font_count() >= 1);
        assert!(!font.font().get_info().is_err());
    }

    #[test]
    fn fontload_from_buffer_works() {
        let path = sample_font_path();
        let bytes = std::fs::read(&path).expect("read font bytes");
        let font = crate::fontload_buffer(&bytes).expect("load from buffer");
        assert!(font.font().get_font_count() >= 1);
    }

    #[test]
    fn load_font_from_buffer_alias_works() {
        let path = sample_font_path();
        let bytes = std::fs::read(&path).expect("read font bytes");
        let font = crate::load_font_from_buffer(&bytes).expect("load from buffer alias");
        assert!(font.font().get_font_count() >= 1);
    }

    #[test]
    fn load_font_from_source_buffer_works() {
        let path = sample_font_path();
        let bytes = std::fs::read(&path).expect("read font bytes");
        let font = crate::load_font(crate::FontSource::Buffer(&bytes)).expect("load source buffer");
        assert!(font.font().get_font_count() >= 1);
    }

    #[test]
    fn fontload_from_collection_file_works() {
        let font = crate::fontload_file(collection_font_path()).expect("load font collection");
        assert!(font.font().get_font_count() > 1);
    }

    #[test]
    fn fontload_from_collection_buffer_works() {
        let bytes = std::fs::read(collection_font_path()).expect("read collection bytes");
        let font = crate::fontload_buffer(&bytes).expect("load font collection from buffer");
        assert!(font.font().get_font_count() > 1);
    }

    #[test]
    fn fontload_from_woff2_file_works() {
        let path = woff2_font_path();
        let font = crate::fontload_file(&path).expect("load woff2 from file");
        let svg = font.text2svg("A", 24.0, "px").expect("render woff2 text");
        assert!(svg.starts_with("<svg"));
    }

    #[test]
    fn fontload_from_woff2_buffer_works() {
        let path = woff2_font_path();
        let bytes = std::fs::read(&path).expect("read woff2 bytes");
        let font = crate::fontload_buffer(&bytes).expect("load woff2 from buffer");
        let svg = font.text2svg("A", 24.0, "px").expect("render woff2 text");
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn chunked_font_buffer_reports_missing_ranges() {
        let mut buffer = crate::ChunkedFontBuffer::new(10).expect("create chunked buffer");
        assert_eq!(buffer.missing_ranges(), vec![(0, 10)]);

        buffer.append(2, &[1, 2, 3]).expect("append middle chunk");
        assert_eq!(buffer.filled_len(), 3);
        assert_eq!(buffer.missing_ranges(), vec![(0, 2), (5, 10)]);

        buffer.append(0, &[9, 8]).expect("append front chunk");
        buffer
            .append(5, &[7, 6, 5, 4, 3])
            .expect("append tail chunk");
        assert!(buffer.is_complete());
        assert!(buffer.missing_ranges().is_empty());
    }

    #[test]
    fn chunked_font_buffer_reassembles_woff2_out_of_order() {
        let bytes = std::fs::read(woff2_font_path()).expect("read woff2 bytes");
        let mut buffer =
            crate::ChunkedFontBuffer::new(bytes.len()).expect("create chunked font buffer");
        let chunk_size = (bytes.len() / 5).max(1);
        let mut chunks = Vec::new();
        let mut offset = 0usize;
        while offset < bytes.len() {
            let end = (offset + chunk_size).min(bytes.len());
            chunks.push((offset, bytes[offset..end].to_vec()));
            offset = end;
        }

        for (offset, chunk) in chunks.into_iter().rev() {
            buffer.append(offset, &chunk).expect("append chunk");
        }

        assert!(buffer.is_complete());
        let font = buffer.into_loaded_font().expect("load reconstructed woff2");
        let svg = font
            .text2svg("A", 24.0, "px")
            .expect("render reconstructed woff2");
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn chunked_font_buffer_rejects_incomplete_decode() {
        let bytes = std::fs::read(woff2_font_path()).expect("read woff2 bytes");
        let mut buffer =
            crate::ChunkedFontBuffer::new(bytes.len()).expect("create chunked font buffer");
        let halfway = bytes.len() / 2;
        buffer
            .append(0, &bytes[..halfway])
            .expect("append partial bytes");

        match buffer.load_font() {
            Ok(_) => panic!("incomplete buffer should not decode"),
            Err(err) => assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock),
        }
    }

    #[test]
    fn chunked_font_buffer_rejects_conflicting_overlaps() {
        let mut buffer = crate::ChunkedFontBuffer::new(8).expect("create chunked buffer");
        buffer.append(2, &[1, 2, 3]).expect("append initial bytes");
        let err = buffer
            .append(3, &[9, 3])
            .expect_err("overlapping conflicting bytes should fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn font_family_selects_best_cached_face() {
        let regular =
            crate::load_font_from_file(fira_sans_regular_path()).expect("load regular fira sans");
        let black =
            crate::load_font_from_file(fira_sans_black_path()).expect("load black fira sans");

        let mut family = crate::FontFamily::new("Fira Sans");
        family.add_face(
            crate::FontFaceDescriptor::new("Fira Sans")
                .with_font_name("Fira Sans Regular")
                .with_font_weight(crate::FontWeight::NORMAL),
            regular,
        );
        family.add_face(
            crate::FontFaceDescriptor::new("Fira Sans")
                .with_font_name("Fira Sans Black")
                .with_font_weight(crate::FontWeight::BLACK),
            black,
        );

        let descriptor = family
            .resolve_descriptor(
                Some("Fira Sans"),
                None,
                crate::FontWeight::BLACK,
                crate::FontStyle::Normal,
                crate::FontStretch::NORMAL,
            )
            .expect("resolve cached face");
        assert_eq!(descriptor.font_name.as_deref(), Some("Fira Sans Black"));

        let run = crate::text2commands(
            "A",
            crate::FontOptions::from_family(&family)
                .with_font_family("Fira Sans")
                .with_font_weight(crate::FontWeight::BLACK)
                .with_font_size(24.0),
        )
        .expect("render from family");
        assert_eq!(run.glyphs.len(), 1);

        let run = family
            .text2commands(
                "A",
                family
                    .options()
                    .with_font_weight(crate::FontWeight::BLACK)
                    .with_font_size(24.0),
            )
            .expect("render from family method");
        assert_eq!(run.glyphs.len(), 1);

        let svg = family.text2svg("A", 24.0, "px").expect("svg from family");
        assert!(svg.starts_with("<svg"));

        let width = family.measure("A").expect("measure from family");
        assert!(width > 0.0);
    }

    #[test]
    fn font_family_promotes_chunked_face_into_cache() {
        let bytes = std::fs::read(woff2_font_path()).expect("read woff2 bytes");
        let mut family = crate::FontFamily::new("Noto Sans");
        family
            .begin_chunked_face(
                "noto-regular",
                crate::FontFaceDescriptor::new("Noto Sans")
                    .with_font_name("Noto Sans WOFF2")
                    .with_font_weight(crate::FontWeight::NORMAL),
                bytes.len(),
            )
            .expect("begin chunked face");

        let split = bytes.len() / 3;
        family
            .append_chunk("noto-regular", split, &bytes[split..split * 2])
            .expect("append middle chunk");
        assert!(!family
            .missing_ranges("noto-regular")
            .expect("missing ranges")
            .is_empty());
        family
            .append_chunk("noto-regular", 0, &bytes[..split])
            .expect("append first chunk");
        family
            .append_chunk("noto-regular", split * 2, &bytes[split * 2..])
            .expect("append tail chunk");

        let font = family
            .finalize_chunked_face("noto-regular")
            .expect("finalize chunked face");
        let width = font.measure("A").expect("measure finalized font");
        assert!(width > 0.0);
        assert_eq!(family.pending_faces_len(), 0);
        assert_eq!(family.cached_faces_len(), 1);
    }

    #[test]
    fn font_family_falls_back_to_cached_face_per_glyph() {
        let regular =
            crate::load_font_from_file(fira_sans_regular_path()).expect("load regular fira sans");
        let emoji = crate::load_font_from_file(segoe_emoji_font_path()).expect("load segoe emoji");

        let mut family = crate::FontFamily::new("Fira Sans");
        family.add_loaded_font(regular);
        family.add_loaded_font(emoji);

        let run = crate::text2commands(
            "A🥺B",
            crate::FontOptions::from_family(&family)
                .with_font_family("Fira Sans")
                .with_font_size(32.0),
        )
        .expect("render mixed fallback text");

        assert_eq!(run.glyphs.len(), 3);
        assert!(matches!(
            run.glyphs[1].glyph.layers.first(),
            Some(crate::GlyphLayer::Path(layer))
                if !layer.commands.is_empty()
                    && matches!(layer.paint, crate::GlyphPaint::Solid(_))
        ));
    }

    #[test]
    fn font_family_text2svg_uses_fallback_layers() {
        let regular =
            crate::load_font_from_file(fira_sans_regular_path()).expect("load regular fira sans");
        let emoji = crate::load_font_from_file(segoe_emoji_font_path()).expect("load segoe emoji");

        let mut family = crate::FontFamily::new("Fira Sans");
        family.add_loaded_font(regular);
        family.add_loaded_font(emoji);

        let svg = family
            .text2svg("A🥺B", 32.0, "px")
            .expect("render mixed fallback svg");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("fill=\"#"));
    }

    #[test]
    fn fontload_from_woff_file_works() {
        let path = woff_font_path();
        let font = crate::fontload_file(&path).expect("load woff font");
        let svg = font.text2svg("A", 24.0, "px").expect("render woff text");
        assert!(svg.contains("<svg"));
    }

    #[test]
    #[cfg(feature = "cff")]
    fn cff_cid_font_renders_svg() {
        let font = crate::fontload_file(cff_font_path()).expect("load cff font");
        let svg = font
            .font()
            .get_svg('漢', 24.0, "px")
            .expect("render cff text");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<path"));
    }

    #[test]
    fn fontload_from_source_file_works() {
        let path = sample_font_path();
        let font = crate::fontload(crate::FontSource::File(path.as_path())).expect("load source");
        assert!(font.font().get_font_count() >= 1);
    }

    #[test]
    fn text_to_command_and_svg_and_measure_work() {
        let path = sample_font_path();
        let font = crate::fontload_file(&path).expect("load font");

        let commands = font.text2command("A").expect("text2command");
        assert_eq!(commands.len(), 1);
        assert!(commands[0].advance_width > 0.0);
        assert!(!commands[0].commands.is_empty());

        let svg = font.text2svg("A", 24.0, "px").expect("text2svg");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<path"));

        let width = font.measure("A").expect("measure");
        assert!(width > 0.0);
        let two_line_width = font.measure("A\nB").expect("measure multiline");
        assert!(two_line_width >= width);
    }

    #[test]
    fn text2command_supports_sbix_bitmap_glyphs() {
        let path = first_real_sbix_font_path().expect("load real sbix font");
        let bytes = std::fs::read(&path).expect("read sbix font");
        let font = crate::load_font_from_buffer(&bytes).expect("load sbix font");
        let commands = font.text2command("🥺").expect("text2command sbix");

        assert_eq!(commands.len(), 1);
        assert!(commands[0].commands.is_empty());
        let bitmap = commands[0].bitmap.as_ref().expect("bitmap payload");
        assert!(matches!(
            bitmap.format,
            crate::fontreader::BitmapGlyphFormat::Png | crate::fontreader::BitmapGlyphFormat::Jpeg
        ));
        assert!(!bitmap.data.is_empty());

        let svg = font.text2svg("🥺", 32.0, "px").expect("svg from sbix");
        assert!(svg.contains("<image"));
        assert!(svg.contains("data:image/"));
    }

    #[test]
    fn twemoji_sbix_woff2_loads_without_oob_and_renders_bitmap() {
        let path = twemoji_sbix_font_path();
        assert!(path.exists(), "missing Twemoji sbix fixture");

        let bytes = std::fs::read(&path).expect("read Twemoji sbix font");
        let font = crate::load_font_from_buffer(&bytes).expect("load Twemoji sbix font");

        let mut rendered = None;
        for sequence in emoji_ligature_sequence_candidates()
            .into_iter()
            .chain(["🥺", "😀", "👍", "❤️"].into_iter())
        {
            let Ok(commands) = font.text2command(sequence) else {
                continue;
            };
            if commands.iter().any(|glyph| glyph.bitmap.is_some()) {
                rendered = Some((sequence, commands));
                break;
            }
        }

        let (sequence, commands) = rendered.expect("render any bitmap glyph from Twemoji sbix");
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|glyph| glyph.bitmap.is_some()));

        let svg = font.text2svg(sequence, 32.0, "px").expect("svg from Twemoji sbix");
        assert!(svg.contains("data:image/"));
    }

    #[test]
    fn parse_text_units_for_fallback_keeps_emoji_clusters_together() {
        for text in ["👩‍💻", "👨‍👩‍👧‍👦", "🇯🇵", "1️⃣"] {
            let units = crate::fontreader::Font::parse_text_units_for_fallback(text);
            assert_eq!(units.len(), 1, "cluster should stay whole for {text:?}");
            match &units[0] {
                crate::fontreader::ParsedTextUnit::Glyph { text: parsed, .. } => {
                    assert_eq!(parsed, text);
                }
                _ => panic!("expected glyph unit for {text:?}"),
            }
        }
    }

    #[test]
    fn loaded_font_text2glyph_run_keeps_real_emoji_ligature_cluster() {
        let (path, sequence) =
            first_real_emoji_ligature().expect("find real emoji ligature fixture");
        let font = crate::load_font_from_file(path).expect("load emoji ligature font");

        let run = font
            .text2glyph_run(
                sequence,
                crate::FontOptions::from_font_ref(crate::FontRef::Loaded(&font))
                    .with_font_size(32.0),
            )
            .expect("shape emoji ligature");

        assert_eq!(run.glyphs.len(), 1, "expected a single ligature glyph");
        assert!(!run.glyphs[0].glyph.layers.is_empty());
    }

    #[test]
    fn legacy_text2command_keeps_real_emoji_ligature_cluster() {
        let (path, sequence) =
            first_real_emoji_ligature_for_legacy().expect("find legacy emoji ligature fixture");
        let font = crate::load_font_from_file(path).expect("load emoji ligature font");

        let commands = font.text2command(sequence).expect("legacy emoji ligature");
        assert_eq!(commands.len(), 1, "expected a single ligature glyph");
        assert!(commands[0].bitmap.is_some() || !commands[0].commands.is_empty());
    }

    #[test]
    fn font_family_fallback_keeps_real_emoji_ligature_cluster() {
        let regular =
            crate::load_font_from_file(fira_sans_regular_path()).expect("load regular fira sans");
        let (path, sequence) =
            first_real_emoji_ligature().expect("find real emoji ligature fixture");
        let emoji = crate::load_font_from_file(path).expect("load emoji ligature font");

        let mut family = crate::FontFamily::new("Fira Sans");
        family.add_loaded_font(regular);
        family.add_loaded_font(emoji);

        let text = format!("A{}B", sequence);
        let run = crate::text2commands(
            &text,
            crate::FontOptions::from_family(&family)
                .with_font_family("Fira Sans")
                .with_font_size(32.0),
        )
        .expect("render mixed fallback ligature text");

        assert_eq!(run.glyphs.len(), 3, "emoji ligature cluster should stay a single glyph");
        assert!(!run.glyphs[1].glyph.layers.is_empty());
    }

    #[test]
    #[cfg(feature = "layout")]
    fn loaded_font_text2glyph_run_shapes_police_officer_female_emoji_when_supported() {
        let sequence = "👮🏻‍♀️";

        for path in emoji_ligature_font_candidates() {
            if !path.exists() {
                continue;
            }
            let font = crate::load_font_from_file(&path).expect("load emoji ligature font");
            let Ok(run) = font.text2glyph_run(
                sequence,
                crate::FontOptions::from_font_ref(crate::FontRef::Loaded(&font))
                    .with_font_size(32.0),
            ) else {
                continue;
            };
            if run.glyphs.len() == 1 {
                assert!(!run.glyphs[0].glyph.layers.is_empty());
                return;
            }
        }

        panic!("expected at least one real font to shape 👮🏻‍♀️ as a single glyph");
    }

    #[test]
    #[cfg(feature = "layout")]
    fn font_family_fallback_keeps_police_officer_female_emoji_cluster_when_supported() {
        let sequence = "👮🏻‍♀️";

        for path in emoji_ligature_font_candidates() {
            if !path.exists() {
                continue;
            }
            let regular =
                crate::load_font_from_file(fira_sans_regular_path()).expect("load regular fira sans");
            let emoji = crate::load_font_from_file(&path).expect("load emoji font");

            let mut family = crate::FontFamily::new("Fira Sans");
            family.add_loaded_font(regular);
            family.add_loaded_font(emoji);

            let text = format!("A{}B", sequence);
            let Ok(run) = crate::text2commands(
                &text,
                crate::FontOptions::from_family(&family)
                    .with_font_family("Fira Sans")
                    .with_font_size(32.0),
            ) else {
                continue;
            };

            if run.glyphs.len() == 3 {
                assert!(!run.glyphs[1].glyph.layers.is_empty());
                return;
            }
        }

        panic!("expected at least one fallback family to keep 👮🏻‍♀️ as a single glyph cluster");
    }

    #[test]
    fn load_font_from_padded_woff2_buffer_uses_declared_length() {
        let path = twemoji_sbix_font_path();
        assert!(path.exists(), "missing Twemoji sbix fixture");

        let mut bytes = std::fs::read(&path).expect("read woff2 font");
        let original_len = bytes.len();
        bytes.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef, 0, 1, 2, 3]);

        let font = crate::load_font_from_buffer(&bytes).expect("load padded woff2 buffer");
        let commands = font.text2command("🥺").expect("render from padded woff2 buffer");
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|glyph| glyph.bitmap.is_some()));
        assert!(bytes.len() > original_len);
    }

    #[test]
    #[ignore]
    fn investigate_police_officer_female_emoji_ligature() {
        let sequence = "👮🏻‍♀️";

        for path in emoji_ligature_font_candidates() {
            if !path.exists() {
                continue;
            }
            let font = crate::load_font_from_file(&path).expect("load candidate font");
            let glyph_ids = font
                .font()
                .debug_shape_glyph_ids(sequence, None)
                .expect("debug shape glyph ids");
            let ccmp_applied = glyph_ids
                .iter()
                .copied()
                .enumerate()
                .map(|(index, glyph_id)| (glyph_id, index))
                .collect::<Vec<_>>();
            #[cfg(feature = "layout")]
            let mut ccmp_applied = ccmp_applied;
            #[cfg(feature = "layout")]
            let liga = font.font().gsub.as_ref().and_then(|gsub| {
                gsub.apply_ccmp_sequence(&mut ccmp_applied);
                gsub.lookup_liga_sequence(
                    &ccmp_applied
                        .iter()
                        .map(|(glyph_id, _)| *glyph_id)
                        .collect::<Vec<_>>(),
                )
            });
            #[cfg(feature = "layout")]
            let rlig = font.font().gsub.as_ref().and_then(|gsub| {
                gsub.lookup_rlig_sequence(
                    &ccmp_applied
                        .iter()
                        .map(|(glyph_id, _)| *glyph_id)
                        .collect::<Vec<_>>(),
                    None,
                )
            });
            #[cfg(not(feature = "layout"))]
            let liga: Option<usize> = None;
            #[cfg(not(feature = "layout"))]
            let rlig: Option<usize> = None;
            let run = font.text2glyph_run(
                sequence,
                crate::FontOptions::from_font_ref(crate::FontRef::Loaded(&font))
                    .with_font_size(32.0),
            );
            let legacy = font.text2command(sequence);

            println!("font: {}", path.display());
            println!("  glyph_ids: {:?}", glyph_ids);
            println!(
                "  ccmp_glyph_ids: {:?}",
                ccmp_applied
                    .iter()
                    .map(|(glyph_id, _)| *glyph_id)
                    .collect::<Vec<_>>()
            );
            println!("  liga_lookup: {:?}", liga);
            println!("  rlig_lookup: {:?}", rlig);
            println!(
                "  glyph_run: {}",
                match &run {
                    Ok(run) => format!(
                        "ok glyphs={} layers0={}",
                        run.glyphs.len(),
                        run.glyphs
                            .first()
                            .map(|glyph| glyph.glyph.layers.len())
                            .unwrap_or(0)
                    ),
                    Err(err) => format!("err {err}"),
                }
            );
            println!(
                "  legacy: {}",
                match &legacy {
                    Ok(commands) => format!(
                        "ok glyphs={} first_bitmap={} first_cmds={}",
                        commands.len(),
                        commands
                            .first()
                            .and_then(|glyph| glyph.bitmap.as_ref())
                            .is_some(),
                        commands
                            .first()
                            .map(|glyph| glyph.commands.len())
                            .unwrap_or(0)
                    ),
                    Err(err) => format!("err {err}"),
                }
            );

            let regular =
                crate::load_font_from_file(fira_sans_regular_path()).expect("load regular fira");
            let mut family = crate::FontFamily::new("Fira Sans");
            family.add_loaded_font(regular);
            family.add_loaded_font(font);
            let family_run = crate::text2commands(
                &format!("A{}B", sequence),
                crate::FontOptions::from_family(&family)
                    .with_font_family("Fira Sans")
                    .with_font_size(32.0),
            );
            println!(
                "  family glyph_run: {}",
                match family_run {
                    Ok(run) => format!("ok glyphs={}", run.glyphs.len()),
                    Err(err) => format!("err {err}"),
                }
            );
        }
    }

    #[test]
    fn sniff_encoded_image_dimensions_supports_png_and_jpeg_headers() {
        let png = vec![
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0,
            0, 0, 16, 0, 0, 0, 32,
        ];
        let jpeg = vec![
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x00, 0x20, 0x00, 0x10, 0x03, 0x01, 0x11,
            0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00, 0xff, 0xd9,
        ];

        assert_eq!(
            crate::util::sniff_encoded_image_dimensions(&png),
            Some(("image/png", 16, 32))
        );
        assert_eq!(
            crate::util::sniff_encoded_image_dimensions(&jpeg),
            Some(("image/jpeg", 16, 32))
        );
    }

    #[test]
    fn glyph_run_from_truetype_outline_works() {
        let path = sample_font_path();
        let font = crate::load_font_from_file(&path).expect("load font");
        let run = crate::text2commands("A", crate::FontOptions::new(&font).with_font_size(24.0))
            .expect("glyph run");

        assert_eq!(run.glyphs.len(), 1);
        assert!(run.glyphs[0].glyph.metrics.advance_x > 0.0);
        assert!(matches!(
            run.glyphs[0].glyph.layers.first(),
            Some(crate::GlyphLayer::Path(_))
        ));
    }

    #[test]
    #[cfg(feature = "cff")]
    fn glyph_run_from_cff_outline_works() {
        let font = crate::load_font_from_file(cff_font_path()).expect("load cff font");
        let run = crate::text2commands("漢", crate::FontOptions::new(&font).with_font_size(24.0))
            .expect("glyph run");

        assert_eq!(run.glyphs.len(), 1);
        assert!(matches!(
            run.glyphs[0].glyph.layers.first(),
            Some(crate::GlyphLayer::Path(_))
        ));
    }

    #[test]
    fn glyph_run_respects_line_height() {
        let path = sample_font_path();
        let font = crate::load_font_from_file(&path).expect("load font");
        let run = crate::text2commands(
            "A\nB",
            crate::FontOptions::new(&font)
                .with_font_size(24.0)
                .with_line_height(40.0),
        )
        .expect("glyph run");

        assert_eq!(run.glyphs.len(), 2);
        assert_eq!(run.glyphs[0].y, 0.0);
        assert_eq!(run.glyphs[1].y, 40.0);
    }

    #[test]
    fn glyph_run_rejects_svg_glyphs() {
        let font = crate::load_font_from_file(svg_font_path()).expect("load svg font");
        let err = crate::text2commands("😀", crate::FontOptions::new(&font).with_font_size(32.0))
            .expect_err("svg glyphs should be rejected for now");

        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }

    #[test]
    fn fira_sans_black_i_and_j_have_outline_commands() {
        let font = crate::load_font_from_file(fira_sans_black_path()).expect("load fira sans");

        for ch in ['i', 'j'] {
            let commands = font.text2command(&ch.to_string()).expect("text2command");
            assert_eq!(commands.len(), 1, "expected one glyph for {ch}");
            assert!(
                !commands[0].commands.is_empty(),
                "expected outline commands for {ch}"
            );
        }
    }

    #[test]
    fn glyph_run_fira_sans_black_keeps_outline_layers() {
        let font = crate::load_font_from_file(fira_sans_black_path()).expect("load fira sans");
        let run = crate::text2commands("ij", crate::FontOptions::new(&font).with_font_size(32.0))
            .expect("glyph run");

        assert_eq!(run.glyphs.len(), 2);
        for (index, glyph) in run.glyphs.iter().enumerate() {
            assert!(
                glyph.glyph.metrics.bounds.is_some(),
                "expected bounds for glyph {index}"
            );
            match glyph.glyph.layers.first() {
                Some(crate::GlyphLayer::Path(path)) => {
                    assert!(
                        !path.commands.is_empty(),
                        "expected path commands for glyph {index}"
                    );
                }
                Some(crate::GlyphLayer::Raster(_)) => {
                    panic!("expected outline layer for Fira Sans glyph {index}");
                }
                None => panic!("expected at least one layer for glyph {index}"),
            }
        }
    }

    #[test]
    fn glyph_run_colr_layers_keep_cpal_argb32_paint() {
        let font = crate::load_font_from_file(segoe_emoji_font_path()).expect("load segoe emoji");
        let inner = font.font();
        let glyph_id = inner
            .cmap
            .as_ref()
            .expect("cmap")
            .get_glyph_position('🥺' as u32) as usize;
        let expected_layers = inner
            .colr
            .as_ref()
            .expect("colr")
            .get_layer_record(glyph_id as u16);
        let cpal = inner.cpal.as_ref().expect("cpal");
        let run = crate::text2commands("🥺", crate::FontOptions::new(&font).with_font_size(32.0))
            .expect("glyph run");

        assert_eq!(run.glyphs.len(), 1);
        assert_eq!(run.glyphs[0].glyph.layers.len(), expected_layers.len());

        for (actual, expected) in run.glyphs[0]
            .glyph
            .layers
            .iter()
            .zip(expected_layers.iter())
        {
            let color = cpal.get_pallet(expected.palette_index as usize);
            let expected_argb = ((color.alpha as u32) << 24)
                | ((color.red as u32) << 16)
                | ((color.green as u32) << 8)
                | color.blue as u32;

            match actual {
                crate::GlyphLayer::Path(path) => match path.paint {
                    crate::GlyphPaint::Solid(argb) => assert_eq!(argb, expected_argb),
                    crate::GlyphPaint::CurrentColor => {
                        panic!("expected COLR glyph layer to keep CPAL color")
                    }
                },
                crate::GlyphLayer::Raster(_) => {
                    panic!("expected COLR glyph to use only path layers")
                }
            }
        }
    }

    #[test]
    fn glyph_run_colr_layers_keep_non_empty_commands() {
        let font = crate::load_font_from_file(segoe_emoji_font_path()).expect("load segoe emoji");
        let run = crate::text2commands("🥺", crate::FontOptions::new(&font).with_font_size(32.0))
            .expect("glyph run");

        assert_eq!(run.glyphs.len(), 1);
        assert!(
            !run.glyphs[0].glyph.layers.is_empty(),
            "expected at least one COLR layer"
        );

        for (index, layer) in run.glyphs[0].glyph.layers.iter().enumerate() {
            match layer {
                crate::GlyphLayer::Path(path) => {
                    assert!(
                        !path.commands.is_empty(),
                        "expected non-empty path commands for COLR layer {index}"
                    );
                }
                crate::GlyphLayer::Raster(_) => {
                    panic!("expected COLR glyph to use only path layers");
                }
            }
        }
    }

    #[test]
    fn vertical_html_path_is_enabled() {
        let path = sample_font_path();
        let font = crate::fontload_file(&path).expect("load font");
        let html = font
            .font()
            .get_html_vert("A", 24.0, "px")
            .expect("html vert");
        assert!(html.contains("writing-mode: vertical-rl"));
        assert!(html.contains("<svg"));
    }

    #[test]
    fn variation_selector_real_font_uses_format14() {
        let font = crate::fontload_file(japanese_font_path()).expect("load uvs font");
        let cmap = font.font().cmap.as_ref().expect("cmap");
        let format14 = cmap
            .cmap_encodings
            .iter()
            .find_map(|encoding| match encoding.cmap_subtable.as_ref() {
                CmapSubtable::Format14(format14) => Some(format14),
                _ => None,
            })
            .expect("expected format 14 cmap");
        let var_selector_record = format14
            .var_selector_records
            .first()
            .expect("expected at least one var selector record");
        let mapping = var_selector_record
            .non_default_uvs
            .unicode_value_ranges
            .first()
            .expect("expected at least one UVS mapping");
        let var_selector = var_selector_record.var_selector;
        let unicode_value = mapping.unicode_value;
        let glyph_id = mapping.glyph_id;
        let base = cmap.get_glyph_position(unicode_value);
        let uvs = cmap.get_glyph_position_from_uvs(unicode_value, var_selector);
        assert_eq!(uvs, glyph_id);
        assert!(base > 0);
    }

    #[test]
    #[cfg(feature = "cff")]
    fn glyph_run_uses_real_variation_selector_as_single_cluster() {
        let font = crate::load_font_from_file(japanese_font_path()).expect("load uvs font");
        let (text, _) = real_variation_sequence(&font);

        let run = crate::text2commands(&text, crate::FontOptions::new(&font).with_font_size(32.0))
            .expect("glyph run");

        assert_eq!(run.glyphs.len(), 1);
        assert!(matches!(
            run.glyphs[0].glyph.layers.first(),
            Some(crate::GlyphLayer::Path(_))
        ));
    }

    #[test]
    #[cfg(feature = "cff")]
    fn measure_uses_real_variation_selector_glyph() {
        let font = crate::load_font_from_file(japanese_font_path()).expect("load uvs font");
        let (text, expected_glyph_id) = real_variation_sequence(&font);

        let glyph_ids = font
            .font()
            .debug_shape_glyph_ids(&text, None)
            .expect("shape glyph ids with uvs");
        assert_eq!(glyph_ids, vec![expected_glyph_id]);

        let width = font.measure(&text).expect("measure with uvs");
        assert!(width > 0.0);
    }

    #[test]
    #[cfg(feature = "cff")]
    fn html_uses_single_svg_for_variation_selector_cluster_and_ignores_stray_selector() {
        let font = crate::load_font_from_file(japanese_font_path()).expect("load uvs font");
        let (text, _) = real_variation_sequence(&font);

        let html = font
            .font()
            .get_html(&text, 32.0, "px")
            .expect("html with uvs");
        assert_eq!(html.matches("<svg").count(), 1);

        let html_with_stray = font
            .font()
            .get_html(&format!("\u{FE0F}{text}"), 32.0, "px")
            .expect("html with stray selector");
        assert_eq!(html_with_stray.matches("<svg").count(), 1);
    }

    #[test]
    #[cfg(feature = "cff")]
    fn font_family_text2commands_keeps_real_variation_selector_cluster() {
        let font = crate::load_font_from_file(japanese_font_path()).expect("load uvs font");
        let (text, expected_glyph_id) = real_variation_sequence(&font);

        let mut family = crate::FontFamily::new("Noto Sans JP");
        family.add_loaded_font(font);
        let run = family
            .text2commands(&text, family.options().with_font_size(32.0))
            .expect("family glyph run with uvs");

        assert_eq!(run.glyphs.len(), 1);
        let glyph_ids = family
            .resolve_loaded_font(
                Some("Noto Sans JP"),
                None,
                crate::FontWeight::default(),
                crate::FontStyle::default(),
                crate::FontStretch::default(),
            )
            .expect("resolved family font")
            .font()
            .debug_shape_glyph_ids(&text, None)
            .expect("shape glyph ids");
        assert_eq!(glyph_ids, vec![expected_glyph_id]);
    }

    #[test]
    fn legacy_text2command_keeps_real_variation_selector_cluster_for_truetype() {
        let Some(path) = first_truetype_variation_selector_font_path() else {
            return;
        };
        let font = crate::load_font_from_file(path).expect("load truetype uvs font");
        let (text, expected_glyph_id) = real_variation_sequence(&font);

        let commands = font
            .text2command(&text)
            .expect("legacy text2command with uvs");

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].glyph_id, expected_glyph_id);
        assert!(!commands[0].commands.is_empty() || commands[0].bitmap.is_some());
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_locale_uses_real_japanese_locl_data() {
        let path = japanese_font_path();
        let font = crate::fontload_file(&path).expect("load japanese font");
        let gsub = font.font().gsub.as_ref().expect("gsub");
        let max_glyphs = font.font().maxp.as_ref().expect("maxp").num_glyphs as usize;
        let locale = "ja-JP";

        let mut found = None;
        for glyph_id in 1..=max_glyphs {
            let localized = gsub.lookup_locale(glyph_id, locale);
            if localized != glyph_id {
                found = Some((glyph_id, localized));
                break;
            }
        }

        let (glyph_id, localized) = found.expect("expected at least one locl substitution");
        assert_ne!(glyph_id, localized);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn text_api_uses_real_japanese_locl_substitution_when_requested() {
        let font = crate::load_font_from_file(japanese_font_path()).expect("load japanese font");
        let cmap = font.font().cmap.as_ref().expect("cmap");
        let gsub = font.font().gsub.as_ref().expect("gsub");
        let locale = "ja-JP";

        let mut found = None;
        for codepoint in 0x20u32..=0xFFFF {
            let Some(ch) = char::from_u32(codepoint) else {
                continue;
            };
            if ch.is_control() {
                continue;
            }
            let glyph_id = cmap.get_glyph_position(codepoint) as usize;
            if glyph_id == 0 {
                continue;
            }

            let localized = gsub.lookup_locale(glyph_id, locale);
            if localized != glyph_id {
                found = Some((ch, glyph_id, localized));
                break;
            }
        }

        let (ch, glyph_id, localized) = found.expect("expected locl-mapped character");
        let glyph_ids = font
            .font()
            .debug_shape_glyph_ids(&ch.to_string(), Some(locale))
            .expect("shape glyph ids");

        assert_eq!(glyph_ids, vec![localized]);
        assert_ne!(glyph_id, localized);

        let run = crate::text2commands(
            &ch.to_string(),
            crate::FontOptions::new(&font)
                .with_font_size(32.0)
                .with_locale(locale),
        )
        .expect("glyph run");
        assert_eq!(run.glyphs.len(), 1);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn text_api_uses_real_jp78_variant_when_requested() {
        let Some((path, ch, glyph_id, variant_glyph_id)) =
            first_real_variant_substitution(crate::FontVariant::Jis78)
        else {
            return;
        };
        let font = crate::load_font_from_file(&path).expect("load japanese variant font");

        let default_ids = font
            .font()
            .debug_shape_glyph_ids_with_variant(
                &ch.to_string(),
                Some("ja-JP"),
                crate::FontVariant::Normal,
            )
            .expect("default glyph ids");
        assert_eq!(default_ids, vec![glyph_id]);

        let variant_ids = font
            .font()
            .debug_shape_glyph_ids_with_variant(
                &ch.to_string(),
                Some("ja-JP"),
                crate::FontVariant::Jis78,
            )
            .expect("jp78 glyph ids");
        assert_eq!(variant_ids, vec![variant_glyph_id]);
        assert_ne!(glyph_id, variant_glyph_id);

        let run = crate::text2commands(
            &ch.to_string(),
            crate::FontOptions::new(&font)
                .with_font_size(32.0)
                .with_locale("ja-JP")
                .with_font_variant(crate::FontVariant::Jis78),
        )
        .expect("jp78 glyph run");
        assert_eq!(run.glyphs.len(), 1);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn font_family_text2commands_uses_real_jp78_variant_when_requested() {
        let Some((path, ch, _, _variant_glyph_id)) =
            first_real_variant_substitution(crate::FontVariant::Jis78)
        else {
            return;
        };
        let font = crate::load_font_from_file(&path).expect("load japanese variant font");
        let mut family = crate::FontFamily::new("JIS Variant");
        family.add_loaded_font(font);

        let run = family
            .text2commands(
                &ch.to_string(),
                family
                    .options()
                    .with_font_size(32.0)
                    .with_locale("ja-JP")
                    .with_font_variant(crate::FontVariant::Jis78),
            )
            .expect("family jp78 glyph run");
        assert_eq!(run.glyphs.len(), 1);
        assert!(run.glyphs[0].glyph.metrics.advance_x > 0.0);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn measure_uses_real_gpos_kern_pair_when_layout_enabled() {
        let font = crate::load_font_from_file(fira_sans_regular_path()).expect("load fira sans");
        let (left, right, total_adjustment) =
            first_real_kern_pair(&font).expect("expected real kern pair in Fira Sans");

        let pair = format!("{left}{right}");
        let left_width = font.measure(&left.to_string()).expect("measure left");
        let right_width = font.measure(&right.to_string()).expect("measure right");
        let pair_width = font.measure(&pair).expect("measure kern pair");
        let observed_delta = pair_width - (left_width + right_width);

        assert!(
            (observed_delta - total_adjustment as f64).abs() <= 1.0,
            "expected delta {total_adjustment}, got {observed_delta} for {pair:?}",
        );
    }

    #[test]
    #[cfg(feature = "layout")]
    fn glyph_run_uses_real_gpos_kern_pair_when_layout_enabled() {
        let font = crate::load_font_from_file(fira_sans_regular_path()).expect("load fira sans");
        let (left, right, total_adjustment) =
            first_real_kern_pair(&font).expect("expected real kern pair in Fira Sans");
        let options = crate::FontOptions::new(&font).with_font_size(32.0);
        let pair = format!("{left}{right}");

        let left_run = crate::text2commands(&left.to_string(), options).expect("left glyph run");
        let right_run = crate::text2commands(&right.to_string(), options).expect("right glyph run");
        let pair_run = crate::text2commands(&pair, options).expect("pair glyph run");

        assert_eq!(pair_run.glyphs.len(), 2);
        let sum_single = left_run.glyphs[0].glyph.metrics.advance_x
            + right_run.glyphs[0].glyph.metrics.advance_x;
        let sum_pair = pair_run
            .glyphs
            .iter()
            .map(|glyph| glyph.glyph.metrics.advance_x)
            .sum::<f32>();
        let hhea = font.font().hhea.as_ref().expect("hhea");
        let default_line_height =
            (hhea.get_accender() - hhea.get_descender() + hhea.get_line_gap()) as f32;
        let scale_x = options.font_size / default_line_height.max(1.0);
        let expected_delta = total_adjustment as f32 * scale_x;
        let observed_delta = sum_pair - sum_single;

        assert!(
            (observed_delta - expected_delta).abs() <= 0.25,
            "expected scaled delta {expected_delta}, got {observed_delta} for {pair:?}",
        );
        if expected_delta < 0.0 {
            assert!(sum_pair < sum_single);
        } else {
            assert!(sum_pair > sum_single);
        }
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_liga_sequence_uses_real_font_data() {
        let path = japanese_font_path();
        let font = crate::fontload_file(&path).expect("load japanese font");
        let gsub = font.font().gsub.as_ref().expect("gsub");
        let cmap = font.font().cmap.as_ref().expect("cmap");
        let candidates = [
            vec!['f', 'i'],
            vec!['f', 'l'],
            vec!['f', 'f'],
            vec!['T', 'o'],
        ];

        for candidate in candidates.iter() {
            let glyph_ids: Vec<usize> = candidate
                .iter()
                .map(|ch| cmap.get_glyph_position(*ch as u32) as usize)
                .collect();
            if glyph_ids.iter().any(|glyph_id| *glyph_id == 0) {
                continue;
            }

            if let Some(ligature_glyph) = gsub.lookup_liga_sequence(&glyph_ids) {
                assert_ne!(ligature_glyph, glyph_ids[0]);
                return;
            }
        }

        panic!("expected at least one ligature sequence in real font data");
    }

    #[test]
    #[cfg(feature = "layout")]
    fn text2command_uses_real_ligature_glyph_when_layout_enabled() {
        let font = crate::load_font_from_file(fira_sans_regular_path()).expect("load fira sans");
        let cmap = font.font().cmap.as_ref().expect("cmap");
        let gsub = font.font().gsub.as_ref().expect("gsub");
        let glyph_ids = [
            cmap.get_glyph_position('f' as u32) as usize,
            cmap.get_glyph_position('i' as u32) as usize,
        ];
        let ligature_glyph = gsub
            .lookup_liga_sequence(&glyph_ids)
            .expect("expected fi ligature in Fira Sans");

        let commands = font.text2command("fi").expect("text2command");

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].glyph_id, ligature_glyph);
        assert!(!commands[0].commands.is_empty());
    }

    #[test]
    #[cfg(feature = "layout")]
    fn glyph_run_uses_real_ligature_glyph_when_layout_enabled() {
        let font = crate::load_font_from_file(fira_sans_regular_path()).expect("load fira sans");
        let run = crate::text2commands("fi", crate::FontOptions::new(&font).with_font_size(32.0))
            .expect("glyph run");

        assert_eq!(run.glyphs.len(), 1);
        assert!(run.glyphs[0].glyph.metrics.advance_x > 0.0);
        match run.glyphs[0].glyph.layers.first() {
            Some(crate::GlyphLayer::Path(path)) => assert!(!path.commands.is_empty()),
            Some(crate::GlyphLayer::Raster(_)) => panic!("expected outline ligature glyph"),
            None => panic!("expected ligature layer"),
        }
    }

    #[test]
    #[cfg(feature = "layout")]
    fn vertical_lookup_uses_real_font_data() {
        let font = crate::fontload_file(japanese_font_path()).expect("load japanese font");
        let (ch, horizontal, vertical) = first_real_vertical_substitution(&font)
            .expect("expected at least one vertical substitution");
        assert_ne!(horizontal, vertical, "vertical form should differ for {ch}");
    }

    #[test]
    #[cfg(feature = "layout")]
    fn glyph_run_uses_vertical_flow_and_positions_glyphs_vertically() {
        let font = crate::load_font_from_file(japanese_font_path()).expect("load japanese font");
        let (ch, horizontal, vertical) =
            first_real_vertical_substitution(&font).expect("expected vertical substitution");
        let text = format!("{ch}{ch}");
        let options = crate::FontOptions::new(&font)
            .with_font_size(32.0)
            .with_vertical_flow();
        let run = crate::text2commands(&text, options).expect("vertical glyph run");

        assert_eq!(run.glyphs.len(), 2);
        let first_font = run.glyphs[0].glyph.font.expect("font metrics");
        assert_eq!(first_font.flow, crate::GlyphFlow::Vertical);
        assert!(run.glyphs[0].glyph.metrics.advance_y > 0.0);
        assert_ne!(horizontal, vertical);
        assert!(run.glyphs[1].y > run.glyphs[0].y);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn measure_with_vertical_flow_reports_positive_inline_extent() {
        let font = crate::load_font_from_file(japanese_font_path()).expect("load japanese font");
        let (ch, _, _) =
            first_real_vertical_substitution(&font).expect("expected vertical substitution");
        let text = format!("{ch}{ch}");
        let options = crate::FontOptions::new(&font)
            .with_font_size(32.0)
            .with_vertical_flow();
        let run = crate::text2commands(&text, options).expect("vertical glyph run");
        let measure = font
            .measure_with_options(&text, options)
            .expect("measure vertical flow");

        assert!(measure > 0.0);
        let expected_min = run.glyphs[1].y as f64 + run.glyphs[1].glyph.metrics.advance_y as f64;
        assert!(measure >= expected_min - 1.0);
    }

    #[test]
    fn glyph_run_positions_hebrew_text_right_to_left() {
        let font = crate::load_font_from_file(rtl_font_path()).expect("load rtl font");
        let text = "אבג";
        let cmap = font.font().cmap.as_ref().expect("cmap");
        for ch in text.chars() {
            assert_ne!(
                cmap.get_glyph_position(ch as u32),
                0,
                "missing glyph for {ch}"
            );
        }

        let ltr_options = crate::FontOptions::new(&font).with_font_size(32.0);
        let rtl_options = crate::FontOptions::new(&font)
            .with_font_size(32.0)
            .with_right_to_left();
        let ltr_run = crate::text2commands(text, ltr_options).expect("ltr glyph run");
        let rtl_run = crate::text2commands(text, rtl_options).expect("rtl glyph run");

        assert_eq!(rtl_run.glyphs.len(), 3);
        assert_eq!(
            rtl_run.glyphs[0].glyph.font.expect("font metrics").flow,
            crate::GlyphFlow::Horizontal
        );
        assert!(rtl_run.glyphs[0].x > rtl_run.glyphs[1].x);
        assert!(rtl_run.glyphs[1].x > rtl_run.glyphs[2].x);

        let ltr_measure = font
            .measure_with_options(text, ltr_options)
            .expect("measure ltr");
        let rtl_measure = font
            .measure_with_options(text, rtl_options)
            .expect("measure rtl");
        assert!(rtl_measure > 0.0);
        assert!((ltr_measure - rtl_measure).abs() <= 1.0);
        assert!(ltr_run.glyphs[0].x < ltr_run.glyphs[1].x);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn rtl_shaping_uses_real_arabic_joining_forms() {
        let font = crate::load_font_from_file(rtl_font_path()).expect("load rtl font");
        let (text, expected_glyph_ids) =
            first_real_arabic_joining_pair(&font).expect("expected arabic joining pair");

        let glyph_ids = font
            .font()
            .debug_shape_glyph_ids_with_direction(&text, Some("ar"), true)
            .expect("shape rtl glyph ids");
        assert_eq!(glyph_ids, expected_glyph_ids);

        let run = crate::text2commands(
            &text,
            crate::FontOptions::new(&font)
                .with_font_size(32.0)
                .with_locale("ar")
                .with_right_to_left(),
        )
        .expect("rtl arabic glyph run");
        assert_eq!(run.glyphs.len(), expected_glyph_ids.len());
        assert!(run.glyphs[0].x > run.glyphs[1].x);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn font_family_text2commands_uses_real_arabic_joining_forms() {
        let font = crate::load_font_from_file(rtl_font_path()).expect("load rtl font");
        let (text, expected_glyph_ids) =
            first_real_arabic_joining_pair(&font).expect("expected arabic joining pair");
        let mut family = crate::FontFamily::new("Arial");
        family.add_loaded_font(font);

        let run = family
            .text2commands(
                &text,
                family
                    .options()
                    .with_font_size(32.0)
                    .with_locale("ar")
                    .with_right_to_left(),
            )
            .expect("family rtl arabic glyph run");
        assert_eq!(run.glyphs.len(), expected_glyph_ids.len());
        assert!(run.glyphs[0].x > run.glyphs[1].x);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn rtl_shaping_uses_real_arabic_required_ligature() {
        let font = crate::load_font_from_file(rtl_font_path()).expect("load rtl font");
        let (text, expected_ligature) =
            first_real_arabic_rlig_sequence(&font).expect("expected arabic required ligature");

        let glyph_ids = font
            .font()
            .debug_shape_glyph_ids_with_direction(&text, Some("ar"), true)
            .expect("shape rtl glyph ids");
        assert_eq!(glyph_ids, vec![expected_ligature]);

        let run = crate::text2commands(
            &text,
            crate::FontOptions::new(&font)
                .with_font_size(32.0)
                .with_locale("ar")
                .with_right_to_left(),
        )
        .expect("rtl arabic ligature glyph run");
        assert_eq!(run.glyphs.len(), 1);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn font_family_text2commands_uses_real_arabic_required_ligature() {
        let font = crate::load_font_from_file(rtl_font_path()).expect("load rtl font");
        let (text, expected_ligature) =
            first_real_arabic_rlig_sequence(&font).expect("expected arabic required ligature");
        let mut family = crate::FontFamily::new("Arial");
        family.add_loaded_font(font);

        let run = family
            .text2commands(
                &text,
                family
                    .options()
                    .with_font_size(32.0)
                    .with_locale("ar")
                    .with_right_to_left(),
            )
            .expect("family rtl arabic ligature glyph run");
        assert_eq!(run.glyphs.len(), 1);

        let glyph_ids = family
            .resolve_loaded_font(
                Some("Arial"),
                None,
                crate::FontWeight::default(),
                crate::FontStyle::default(),
                crate::FontStretch::default(),
            )
            .expect("resolved family font")
            .font()
            .debug_shape_glyph_ids_with_direction(&text, Some("ar"), true)
            .expect("shape rtl glyph ids");
        assert_eq!(glyph_ids, vec![expected_ligature]);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn rtl_shaping_uses_real_arabic_contextual_sequence() {
        let Some((path, text, expected_glyph_ids)) = first_real_arabic_contextual_sequence() else {
            return;
        };
        let font = crate::load_font_from_file(&path).expect("load rtl contextual font");

        let glyph_ids = font
            .font()
            .debug_shape_glyph_ids_with_direction(&text, Some("ar"), true)
            .expect("shape rtl contextual glyph ids");
        assert_eq!(glyph_ids, expected_glyph_ids);

        let run = crate::text2commands(
            &text,
            crate::FontOptions::new(&font)
                .with_font_size(32.0)
                .with_locale("ar")
                .with_right_to_left(),
        )
        .expect("rtl arabic contextual glyph run");
        assert_eq!(run.glyphs.len(), expected_glyph_ids.len());
    }

    #[test]
    #[cfg(feature = "layout")]
    fn font_family_text2commands_uses_real_arabic_contextual_sequence() {
        let Some((path, text, expected_glyph_ids)) = first_real_arabic_contextual_sequence() else {
            return;
        };
        let font = crate::load_font_from_file(&path).expect("load rtl contextual font");
        let mut family = crate::FontFamily::new("RTL Contextual");
        family.add_loaded_font(font);

        let run = family
            .text2commands(
                &text,
                family
                    .options()
                    .with_font_size(32.0)
                    .with_locale("ar")
                    .with_right_to_left(),
            )
            .expect("family rtl arabic contextual glyph run");
        assert_eq!(run.glyphs.len(), expected_glyph_ids.len());

        let glyph_ids = family
            .resolve_loaded_font(
                Some("RTL Contextual"),
                None,
                crate::FontWeight::default(),
                crate::FontStyle::default(),
                crate::FontStretch::default(),
            )
            .expect("resolved family font")
            .font()
            .debug_shape_glyph_ids_with_direction(&text, Some("ar"), true)
            .expect("shape rtl contextual glyph ids");
        assert_eq!(glyph_ids, expected_glyph_ids);
    }

    #[test]
    #[cfg(feature = "layout")]
    fn font_family_text2commands_supports_vertical_flow() {
        let font = crate::load_font_from_file(japanese_font_path()).expect("load japanese font");
        let (ch, _, _) =
            first_real_vertical_substitution(&font).expect("expected vertical substitution");
        let mut family = crate::FontFamily::new("Noto Sans JP");
        family.add_loaded_font(font);

        let run = family
            .text2commands(
                &format!("{ch}{ch}"),
                family.options().with_font_size(32.0).with_vertical_flow(),
            )
            .expect("family vertical glyph run");

        assert_eq!(run.glyphs.len(), 2);
        assert_eq!(
            run.glyphs[0].glyph.font.expect("font metrics").flow,
            crate::GlyphFlow::Vertical
        );
        assert!(run.glyphs[1].y > run.glyphs[0].y);
    }

    #[test]
    fn font_family_text2commands_supports_right_to_left() {
        let font = crate::load_font_from_file(rtl_font_path()).expect("load rtl font");
        let mut family = crate::FontFamily::new("Arial");
        family.add_loaded_font(font);

        let run = family
            .text2commands(
                "אבג",
                family.options().with_font_size(32.0).with_right_to_left(),
            )
            .expect("family rtl glyph run");

        assert_eq!(run.glyphs.len(), 3);
        assert!(run.glyphs[0].x > run.glyphs[1].x);
        assert!(run.glyphs[1].x > run.glyphs[2].x);

        let measure = family
            .measure_with_options(
                "אבג",
                family.options().with_font_size(32.0).with_right_to_left(),
            )
            .expect("family rtl measure");
        assert!(measure > 0.0);
    }
}
