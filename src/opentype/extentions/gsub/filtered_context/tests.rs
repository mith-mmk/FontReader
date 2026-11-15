use super::*;
use crate::opentype::extentions::gdef::MarkGlyphSetsDef;
use crate::opentype::layouts::{classdef::ClassDefFormat1, coverage::CoverageFormat1, lookup::*};

fn coverage(ids: &[u16]) -> Coverage {
    Coverage::Format1(CoverageFormat1 {
        coverage_format: 1,
        glyph_count: ids.len() as u16,
        glyph_ids: ids.to_vec(),
    })
}
fn classes(values: &[u16]) -> ClassDef {
    ClassDef::Format1(ClassDefFormat1 {
        class_format: 1,
        start_glyph_id: 1,
        glyph_count: values.len() as u16,
        class_value_array: values.to_vec(),
    })
}
fn gdef() -> Gdef {
    Gdef {
        major_versionn: 1,
        minor_version: 2,
        glyph_class_def: Some(classes(&[1, 3, 3, 1, 2, 3, 1])),
        attach_list: None,
        lig_caret_list: None,
        mark_attach_class_def: Some(classes(&[0, 1, 2, 0, 0, 1, 0])),
        mark_glyph_sets_def: Some(MarkGlyphSetsDef {
            mark_set_table_format: 1,
            mark_set_count: 1,
            coverages: vec![coverage(&[2, 6])],
        }),
        item_var_store_offset: None,
    }
}
fn lookup(kind: u16, flags: u16, subtables: Vec<Subtable>) -> Lookup {
    Lookup {
        lookup_type: kind,
        lookup_flag: flags,
        mark_filtering_set: (flags & 16 != 0).then_some(0),
        subtables,
    }
}
fn chain(input: &[u16], back: &[u16], ahead: &[u16], records: &[(u16, u16)]) -> Subtable {
    Subtable::ChainingContextSubstitution3(ChainingContextSubstitutionFormat3 {
        format: 3,
        backtrack_glyph_count: back.len() as u16,
        backtrack_coverages: back.iter().map(|x| coverage(&[*x])).collect(),
        input_glyph_count: input.len() as u16,
        input_coverages: input.iter().map(|x| coverage(&[*x])).collect(),
        lookahead_glyph_count: ahead.len() as u16,
        lookahead_coverages: ahead.iter().map(|x| coverage(&[*x])).collect(),
        seq_lookup_count: records.len() as u16,
        seq_lookup_records: SequenceLookupRecords {
            lookup_records: records
                .iter()
                .map(|&(sequence_index, lookup_list_index)| LookupRecord {
                    sequence_index,
                    lookup_list_index,
                })
                .collect(),
        },
    })
}
fn lig(input: &[u16], output: u16) -> Subtable {
    Subtable::Ligature(LigatureSubstitutionFormat1 {
        subst_format: 1,
        coverage: coverage(&input[..1]),
        ligature_set_count: 1,
        ligature_set: vec![LigatureSet {
            ligature_count: 1,
            ligature_table: vec![LigatureTable {
                ligature_glyph: output,
                component_count: input.len() as u16,
                component_glyph_ids: input[1..].to_vec(),
            }],
        }],
    })
}
fn single(from: u16, to: u16) -> Subtable {
    Subtable::Single2(SingleSubstitutionFormat2 {
        subst_format: 2,
        coverage: coverage(&[from]),
        glyph_count: 1,
        substitute_glyph_ids: vec![to],
    })
}
fn unsupported(first: u16) -> Subtable {
    Subtable::ContextSubstitution3(ContextSubstitutionFormat3 {
        subst_format: 3,
        glyph_count: 1,
        seq_lookup_count: 0,
        coverages: vec![coverage(&[first])],
        seq_lookup_records: SequenceLookupRecords {
            lookup_records: vec![],
        },
    })
}
fn extension(subtable: Subtable, kind: u16) -> Subtable {
    Subtable::ExtensionSubstitution(ExtensionSubstitutionFormat1 {
        subst_format: 1,
        extension_lookup_type: kind,
        extension_offset: 8,
        subtable: Box::new(subtable),
    })
}
fn table(lookups: Vec<Lookup>) -> Gsub {
    Gsub {
        major_version: 1,
        minor_version: 0,
        scripts: Box::new(ScriptList {
            script_count: 0,
            scripts: vec![],
        }),
        features: Box::new(FeatureList {
            feature_count: 0,
            features: vec![],
        }),
        lookups: Box::new(LookupList { lookups }),
        feature_variations: None,
    }
}
fn glyphs(ids: &[usize]) -> Vec<GsubGlyph> {
    Gsub::with_metadata(
        &ids.iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect::<Vec<_>>(),
    )
}
fn execute(
    t: &Gsub,
    input: &[usize],
    selected: &[usize],
    g: Option<&Gdef>,
    enabled: bool,
) -> ((bool, bool), Vec<GsubGlyph>) {
    let mut glyphs = glyphs(input);
    let lookups: Vec<_> = selected.iter().map(|&i| &t.lookups.lookups[i]).collect();
    let result = t.apply_tibetan_selected_glyphs_with_filtering(
        &mut glyphs,
        lookups.clone(),
        &lookups,
        g,
        enabled,
    );
    (result, glyphs)
}
fn pair(extra: Vec<Subtable>) -> Gsub {
    let mut subtables = extra;
    subtables.push(chain(&[1, 2], &[], &[], &[(0, 1)]));
    table(vec![
        lookup(6, 16, subtables),
        lookup(4, 16, vec![lig(&[1, 2], 5)]),
    ])
}
fn assert_rollback(t: &Gsub, input: &[usize]) {
    let (result, output) = execute(t, input, &[0], Some(&gdef()), true);
    assert_eq!(result, (false, false));
    assert_eq!(output, glyphs(input));
}

#[test]
fn filtered_pair_preserves_retained_mark_and_metadata() {
    let t = pair(vec![]);
    let (result, output) = execute(&t, &[1, 2, 3], &[0], Some(&gdef()), true);
    assert_eq!(result, (true, true));
    assert_eq!(
        output.iter().map(|g| g.glyph_id).collect::<Vec<_>>(),
        [5, 3]
    );
    assert_eq!(output[1], glyphs(&[1, 2, 3])[2]);
    assert_eq!(
        (output[0].source_index, output[0].ligature_components),
        (0, 2)
    );
    assert!(output[0].substituted && output[0].mkmk_source_ambiguous);
}
#[test]
fn earlier_ignore_uses_filtered_lookahead_and_blocks_ligature() {
    let t = pair(vec![chain(&[1, 2], &[], &[4], &[])]);
    let input = [1, 2, 3, 4];
    let (result, output) = execute(&t, &input, &[0], Some(&gdef()), true);
    assert_eq!(result, (true, false));
    assert_eq!(output, glyphs(&input));
    // An excluded suffix alone does not satisfy the lookahead coverage.
    assert_eq!(
        execute(&t, &[1, 2, 3], &[0], Some(&gdef()), true).1[0].glyph_id,
        5
    );
}
#[test]
fn filtered_backtrack_is_nearest_first_and_current_coverage_is_not_backtrack() {
    let t = pair(vec![chain(&[1, 2], &[4, 7], &[], &[])]);
    let input = [7, 4, 3, 1, 2, 3];
    assert_eq!(
        execute(&t, &input, &[0], Some(&gdef()), true).1,
        glyphs(&input)
    );
    let output = execute(&t, &[4, 7, 3, 1, 2, 3], &[0], Some(&gdef()), true).1;
    assert_eq!(output[3].glyph_id, 5);
}
#[test]
fn input_hole_and_generic_skipped_mark_ligature_remain_unsupported() {
    assert_rollback(&pair(vec![]), &[1, 3, 2]);
    let t = table(vec![lookup(4, 8, vec![lig(&[1, 1], 5)])]);
    assert_rollback(&t, &[1, 3, 1]);
}
#[test]
fn unsupported_earlier_rule_cannot_fall_through_but_coverage_miss_can() {
    assert_rollback(&pair(vec![unsupported(1)]), &[1, 2, 3]);
    assert_eq!(
        execute(
            &pair(vec![unsupported(4)]),
            &[1, 2, 3],
            &[0],
            Some(&gdef()),
            true
        )
        .1[0]
            .glyph_id,
        5
    );
    let t = table(vec![lookup(5, 8, vec![unsupported(3)])]);
    assert_eq!(
        execute(&t, &[1, 2, 3], &[0], Some(&gdef()), true).0,
        (true, false)
    );
}
#[test]
fn independent_child_flags_keep_original_context_visible() {
    let t = table(vec![
        lookup(6, 16, vec![chain(&[1, 2], &[], &[], &[(0, 1)])]),
        lookup(6, 0, vec![chain(&[1, 2], &[], &[3], &[(0, 2)])]),
        lookup(4, 16, vec![lig(&[1, 2], 5)]),
    ]);
    assert_eq!(
        execute(&t, &[1, 2, 3], &[0], Some(&gdef()), true).1[0].glyph_id,
        5
    );
    // The child's IgnoreMarks excludes the parent's second input: no substitution.
    let t = table(vec![
        lookup(6, 16, vec![chain(&[1, 2], &[], &[], &[(1, 1)])]),
        lookup(1, 8, vec![single(2, 6)]),
    ]);
    assert_eq!(
        execute(&t, &[1, 2, 3], &[0], Some(&gdef()), true).0,
        (true, false)
    );
}
#[test]
fn child_cannot_consume_beyond_parent_or_jump_past_unsupported_ligature() {
    let t = table(vec![
        lookup(6, 16, vec![chain(&[1, 2], &[], &[], &[(0, 1)])]),
        lookup(4, 0, vec![lig(&[1, 2, 3], 5), lig(&[1, 2], 5)]),
    ]);
    assert_rollback(&t, &[1, 2, 3]);
}
#[test]
fn multi_record_and_invalid_record_are_transactional_failures() {
    for records in [vec![(0, 1), (1, 1)], vec![(2, 1)], vec![(0, 99)]] {
        let t = table(vec![
            lookup(6, 16, vec![chain(&[1, 2], &[], &[], &records)]),
            lookup(1, 0, vec![single(1, 4)]),
        ]);
        assert_rollback(&t, &[1, 2, 3]);
    }
}
#[test]
fn extensions_use_the_same_order_flags_and_boundary_checks() {
    let t = table(vec![
        lookup(
            7,
            16,
            vec![extension(chain(&[1, 2], &[], &[], &[(0, 1)]), 6)],
        ),
        lookup(7, 16, vec![extension(lig(&[1, 2], 5), 4)]),
    ]);
    assert_eq!(
        execute(&t, &[1, 2, 3], &[0], Some(&gdef()), true).1[0].glyph_id,
        5
    );
    let t = table(vec![lookup(
        7,
        16,
        vec![extension(extension(single(1, 4), 1), 7)],
    )]);
    assert_rollback(&t, &[1, 2, 3]);
}
#[test]
fn missing_filter_metadata_and_unknown_classes_fail_closed() {
    let t = pair(vec![]);
    let mut missing_set = gdef();
    missing_set.mark_glyph_sets_def = None;
    let mut missing_class = gdef();
    missing_class.glyph_class_def = None;
    let mut invalid_set = pair(vec![]);
    invalid_set.lookups.lookups[0].mark_filtering_set = Some(9);
    for g in [None, Some(&missing_set), Some(&missing_class)] {
        let (result, output) = execute(&t, &[1, 2, 3], &[0], g, true);
        assert_eq!(result, (false, false));
        assert_eq!(output, glyphs(&[1, 2, 3]));
    }
    assert_rollback(&invalid_set, &[1, 2, 3]);
    assert_rollback(&pair(vec![]), &[1, 9, 3]);
}
#[test]
fn flag_precedence_matches_no_skip_proof() {
    let g = gdef();
    let gs = glyphs(&[1, 2, 3]);
    let mut state = ApplyState::new(Some(&g), &gs);
    state.filtered.enabled = true;
    let mut l = lookup(1, 16 | 0x0200, vec![]);
    assert_eq!(state.filtered_includes(&l, 2), Ok(true));
    assert_eq!(state.filtered_includes(&l, 3), Ok(false));
    l.lookup_flag |= 8;
    l.mark_filtering_set = Some(99);
    assert_eq!(state.filtered_includes(&l, 2), Ok(false));
    l.lookup_flag = 0x0100;
    assert_eq!(state.filtered_includes(&l, 2), Ok(true));
    assert_eq!(state.filtered_includes(&l, 3), Ok(false));
    l.lookup_flag = 0x0100;
    assert_eq!(state.filtered_includes(&l, 3), Ok(false));
    // A defined class table's default class0 is excluded, not missing metadata.
    let mut zero_class = gdef();
    zero_class.mark_attach_class_def = Some(classes(&[0, 0, 0, 0, 0, 0, 0]));
    let mut zero_state = ApplyState::new(Some(&zero_class), &gs);
    assert_eq!(zero_state.filtered_includes(&l, 2), Ok(false));
}
#[test]
fn mutation_rechecks_membership_and_rollback_restores_prior_success() {
    let t = table(vec![
        lookup(1, 0, vec![single(2, 3)]),
        lookup(6, 16, vec![chain(&[1, 6], &[], &[], &[(0, 2)])]),
        lookup(4, 16, vec![lig(&[1, 6], 5)]),
    ]);
    let (result, output) = execute(&t, &[1, 2, 6, 3], &[0, 1], Some(&gdef()), true);
    assert_eq!(result, (false, false));
    assert_eq!(output, glyphs(&[1, 2, 6, 3]));
    let t = table(vec![
        lookup(1, 0, vec![single(3, 6)]),
        lookup(6, 16, vec![chain(&[1, 2], &[], &[], &[(0, 2)])]),
        lookup(4, 16, vec![lig(&[1, 2], 5)]),
    ]);
    assert_eq!(
        execute(&t, &[1, 2, 3], &[0, 1], Some(&gdef()), true)
            .1
            .iter()
            .map(|g| g.glyph_id)
            .collect::<Vec<_>>(),
        [5, 6]
    );
}
#[test]
fn traversal_budget_counts_misses_and_long_excluded_context() {
    let t = pair(vec![single(7, 4); MAX_FILTERED_WORK]);
    assert_rollback(&t, &[1, 2, 3]);
    let t = pair(vec![chain(&[1, 2], &[], &[4], &[])]);
    let mut input = vec![1, 2];
    input.extend(std::iter::repeat_n(3, MAX_FILTERED_WORK));
    input.push(4);
    assert_rollback(&t, &input);
    let mut state = FilteredState {
        enabled: true,
        work_left: 0,
        ..Default::default()
    };
    assert_eq!(state.charge(), Err(()));
    assert_eq!(state.work_left, 0);
}
#[test]
fn recursion_and_application_limits_are_sticky_and_restore_input() {
    let t = table(vec![lookup(
        6,
        16,
        vec![chain(&[1, 2], &[], &[], &[(0, 0)])],
    )]);
    assert_rollback(&t, &[1, 2, 3]);
    let g = gdef();
    let mut input = glyphs(&[1, 2, 3]);
    let mut state = ApplyState::new(Some(&g), &input);
    state.filtered.enabled = true;
    state.remaining = 0;
    assert!(!t.apply_lookup_index_at(0, &mut input, 0, &mut state));
    assert!(state.filtered.failed);
    assert_eq!(input, glyphs(&[1, 2, 3]));
}
#[test]
fn disabled_api_equals_existing_no_skip_path_and_keeps_metadata() {
    for input in [&[1, 2, 3][..], &[1, 2][..], &[1, 3, 2][..], &[3, 1, 2][..]] {
        let t = pair(vec![]);
        let g = gdef();
        let selected = vec![&t.lookups.lookups[0]];
        let mut legacy = glyphs(input);
        let expected =
            t.apply_tibetan_selected_glyphs(&mut legacy, selected.clone(), &selected, Some(&g));
        let (actual, disabled) = execute(&t, input, &[0], Some(&g), false);
        assert_eq!(actual, expected);
        assert_eq!(disabled, legacy);
    }
    let (result, disabled) = execute(&pair(vec![]), &[1, 2, 3], &[0], Some(&gdef()), false);
    assert_eq!(result, (true, false));
    assert_eq!(disabled, glyphs(&[1, 2, 3]));
}

/// Optional external-font evidence, never an installed-font test dependency.
/// FONTCORE_NOTO_TIBETAN must name original NotoSerifTibetan.ttf SHA-256
/// 060ec022b04c306de3f58d051fb0e1cf81a5b610c5910fbfc43bba154c057cda.
#[test]
#[ignore = "requires the pinned external OFL Noto reference font"]
fn original_noto_opt_in_proof_and_work_measurements() {
    use bin_rs::reader::BytesReader;
    let bytes = std::fs::read(std::env::var("FONTCORE_NOTO_TIBETAN").unwrap()).unwrap();
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    let locate = |tag: &[u8; 4]| {
        let entry = bytes[12..12 + count * 16]
            .chunks_exact(16)
            .find(|e| &e[..4] == tag)
            .unwrap();
        (
            u32::from_be_bytes(entry[8..12].try_into().unwrap()),
            u32::from_be_bytes(entry[12..16].try_into().unwrap()),
        )
    };
    let (offset, length) = locate(b"GSUB");
    let t = Gsub::new(&mut BytesReader::new(&bytes), offset, length).unwrap();
    let (offset, length) = locate(b"GDEF");
    let g = Gdef::new(
        &mut BytesReader::new(&bytes),
        offset as u64,
        length as usize,
    )
    .unwrap();
    let mut enabled = glyphs(&[6, 1358, 1328]);
    assert!(t.apply_tibetan_stack_presentation_with_filtering(
        &mut enabled,
        Some("bo"),
        crate::commands::FontVariant::Normal,
        None,
        Some(&g),
        true
    ));
    assert_eq!(
        enabled.iter().map(|g| g.glyph_id).collect::<Vec<_>>(),
        [158, 1328]
    );
    let mut disabled = glyphs(&[6, 1358, 1328]);
    t.apply_tibetan_stack_presentation(
        &mut disabled,
        Some("bo"),
        crate::commands::FontVariant::Normal,
        None,
        Some(&g),
    );
    assert_ne!(disabled, enabled);
    for repeats in [1, 2, 8, 32, 128, 512, 1024] {
        let input = [6, 1358, 1328].repeat(repeats);
        let mut working = glyphs(&input);
        let mut state = ApplyState::new(Some(&g), &working);
        state.filtered.enabled = true;
        let mut counts = HashMap::new();
        for glyph in &working {
            *counts.entry(glyph.glyph_id).or_default() += 1;
        }
        state.exact_glyph_counts = Some(counts);
        let selected = t.collect_tibetan_stack_lookups(
            Some("bo"),
            &[
                *b"abvs", *b"blws", *b"rlig", *b"rclt", *b"calt", *b"clig", *b"liga",
            ],
        );
        let start = std::time::Instant::now();
        for lookup in &t.lookups.lookups {
            if selected.iter().any(|&l| std::ptr::eq(l, lookup)) {
                t.apply_lookup_all_with_tables(lookup, &mut working, &mut state);
                if state.filtered.failed {
                    break;
                }
            }
        }
        eprintln!(
            "Noto repeats={repeats} work={} failed={} glyphs={} micros={}",
            MAX_FILTERED_WORK - state.filtered.work_left,
            state.filtered.failed,
            working.len(),
            start.elapsed().as_micros()
        );
        let mut transaction = glyphs(&input);
        let accepted = t.apply_tibetan_stack_presentation_with_filtering(
            &mut transaction,
            Some("bo"),
            crate::commands::FontVariant::Normal,
            None,
            Some(&g),
            true,
        );
        if state.filtered.failed {
            assert!(!accepted);
            assert_eq!(transaction, glyphs(&input));
        } else {
            assert!(accepted);
            assert_eq!(transaction, working);
        }
    }
}

#[test]
fn filtered_input_probe_distinguishes_definite_miss_from_unsupported_match() {
    // At A, skipped X followed by B definitively fails C. The later single
    // is permitted; no skipped-input substitution has been performed.
    let miss = table(vec![
        lookup(
            6,
            16,
            vec![
                chain(&[1, 4], &[], &[], &[]),
                chain(&[1], &[], &[], &[(0, 1)]),
            ],
        ),
        lookup(1, 0, vec![single(1, 7)]),
    ]);
    let (result, output) = execute(&miss, &[1, 3, 2], &[0], Some(&gdef()), true);
    assert_eq!(result, (true, true));
    assert_eq!(output[0].glyph_id, 7);
    let hit = table(vec![
        lookup(
            6,
            16,
            vec![
                chain(&[1, 2], &[], &[], &[]),
                chain(&[1], &[], &[], &[(0, 1)]),
            ],
        ),
        lookup(1, 0, vec![single(1, 7)]),
    ]);
    assert_rollback(&hit, &[1, 3, 2]);
}

#[test]
fn proven_legacy_record_budget_exhaustion_also_rolls_back_opt_in() {
    let context = Subtable::ContextSubstitution3(ContextSubstitutionFormat3 {
        subst_format: 3,
        glyph_count: 1,
        seq_lookup_count: 4097,
        coverages: vec![coverage(&[1])],
        seq_lookup_records: SequenceLookupRecords {
            lookup_records: vec![
                LookupRecord {
                    sequence_index: 0,
                    lookup_list_index: 1
                };
                MAX_GSUB_LOOKUP_APPLICATIONS + 1
            ],
        },
    });
    let t = table(vec![
        lookup(5, 0, vec![context]),
        lookup(1, 0, vec![single(1, 4)]),
    ]);
    assert_rollback(&t, &[1, 3]);
    let (result, output) = execute(&t, &[1, 3], &[0], Some(&gdef()), false);
    assert_eq!(result, (true, true));
    assert_eq!(output[0].glyph_id, 4);
}
