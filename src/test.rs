mod tests {
    #[cfg(feature = "layout")]
    use crate::opentype::layouts::{
        classdef::ClassRangeRecord,
        coverage::{Coverage, CoverageFormat1, CoverageFormat2, RangeRecord},
        lookup::{
            AlternateSet, AlternateSubstitutionFormat1, ChainSubRule, ChainSubRuleSet,
            ChainingContextSubstitutionFormat1, ChainingContextSubstitutionFormat2,
            ChainingContextSubstitutionFormat3, ContextSubstitutionFormat1, LigatureSet,
            LigatureSubstitutionFormat1, LigatureTable, LookupList, LookupResult,
            LookupSubstitution, LookupType, MultipleSubstitutionFormat1, SequenceRule,
            SequenceRuleSet, SequenceTable, SequenceLookupRecords, SingleSubstitutionFormat1,
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
                assert_eq!(extension.extension_lookup_type, LookupType::SingleSubstitution as u16);
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

    use bin_rs::reader::BytesReader;
    use crate::opentype::requires::cmap::{
        self, CmapEncodings, CmapSubtable, EncodingRecord,
    };

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

    fn format4_table(start_code: u16, end_code: u16, delta: i16, range_offset: u16, glyphs: &[u16]) -> Vec<u8> {
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

    fn collection_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("windows").join("msgothic.ttc")
    }

    #[cfg(feature = "cff")]
    fn cff_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("NotoSansJP-Black.otf")
    }

    fn japanese_font_path() -> std::path::PathBuf {
        test_fonts_dir().join("NotoSansJP-Regular.otf")
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
        let font =
            crate::load_font(crate::FontSource::Buffer(&bytes)).expect("load source buffer");
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
        let err = crate::text2commands(
            "😀",
            crate::FontOptions::new(&font).with_font_size(32.0),
        )
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
        let html = font.font().get_html_vert("A", 24.0, "px").expect("html vert");
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

        let run = crate::text2commands(&text, crate::FontOptions::new(&font).with_font_size(32.0))
            .expect("glyph run");

        assert_eq!(run.glyphs.len(), 1);
        assert!(matches!(
            run.glyphs[0].glyph.layers.first(),
            Some(crate::GlyphLayer::Path(_))
        ));
    }

    #[test]
    #[cfg(feature = "layout")]
    fn lookup_locale_uses_real_japanese_locl_data() {
        let path = japanese_font_path();
        let font = crate::fontload_file(&path).expect("load japanese font");
        let gsub = font.font().gsub.as_ref().expect("gsub");
        let max_glyphs = font.font().maxp.as_ref().expect("maxp").num_glyphs as usize;
        let locale = "ja-JP".to_string();

        let mut found = None;
        for glyph_id in 1..=max_glyphs {
            let localized = gsub.lookup_locale(glyph_id, &locale);
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
        let path = japanese_font_path();
        let font = crate::fontload_file(&path).expect("load japanese font");
        let gsub = font.font().gsub.as_ref().expect("gsub");
        let cmap = font.font().cmap.as_ref().expect("cmap");

        let candidates = [
            '(', ')', '[', ']', '{', '}', '!', '?', ',', '.', ':', ';', '、', '。', '「', '」',
            '（', '）', 'ー', '〜', '＜', '＞',
        ];
        let mut found = None;

        for ch in candidates.iter() {
            let glyph_id = cmap.get_glyph_position(*ch as u32) as u16;
            if glyph_id == 0 {
                continue;
            }

            let vertical = gsub.lookup_vertical(glyph_id).unwrap_or(glyph_id);
            if vertical != glyph_id {
                found = Some((*ch, glyph_id, vertical));
                break;
            }
        }

        let (ch, horizontal, vertical) = found.expect("expected at least one vertical substitution");
        assert_ne!(horizontal, vertical, "vertical form should differ for {ch}");
    }
}
