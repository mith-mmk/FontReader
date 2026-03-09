mod tests {
    #[cfg(feature = "layout")]
    use crate::opentype::layouts::{
        classdef::ClassRangeRecord,
        coverage::{Coverage, CoverageFormat1, CoverageFormat2, RangeRecord},
        lookup::{
            AlternateSet, AlternateSubstitutionFormat1, ChainSubRule, ChainSubRuleSet,
            ChainingContextSubstitutionFormat1, ChainingContextSubstitutionFormat2,
            ChainingContextSubstitutionFormat3, ContextSubstitutionFormat1, LigatureSet,
            LigatureSubstitutionFormat1, LigatureTable, LookupResult, LookupSubstitution,
            MultipleSubstitutionFormat1, SequenceRule, SequenceRuleSet, SequenceTable,
            SequenceLookupRecords, SingleSubstitutionFormat1, SingleSubstitutionFormat2,
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
                .map(|(start_glyph_id, end_glyph_id, start_coverage_index)| RangeRecord {
                    start_glyph_id: *start_glyph_id,
                    end_glyph_id: *end_glyph_id,
                    start_coverage_index: *start_coverage_index,
                })
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
        let chaining = LookupSubstitution::ChainingContextSubstitution(
            ChainingContextSubstitutionFormat1 {
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
            },
        );
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
            });
        match chaining2.get_lookup(90) {
            LookupResult::Multiple(classes) => assert_eq!(classes, vec![4]),
            _ => panic!("unexpected lookup result"),
        }

        let chaining3 = LookupSubstitution::ChainingContextSubstitution3(
            ChainingContextSubstitutionFormat3 {
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
            },
        );
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
}
