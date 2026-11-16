use super::*;
use crate::commands::FontVariant;
use crate::fontreader::ResolvedTextUnit;

fn font(index: usize) -> Font {
    let mut font = Font::get_font_from_buffer(include_bytes!(
        "../../tests/fixtures/hangul-composition.ttc"
    ))
    .unwrap();
    font.set_font(index).unwrap();
    font
}

#[test]
fn all_11172_modern_syllables_compose_from_exact_canonical_forms() {
    // Enumerate the Unicode Hangul syllable order independently of the helper.
    let mut expected = 0xac00;
    for leading in '\u{1100}'..='\u{1112}' {
        for vowel in '\u{1161}'..='\u{1175}' {
            let lv = char::from_u32(expected).unwrap();
            assert_eq!(
                modern_hangul_syllable(&format!("{leading}{vowel}")),
                Some(lv)
            );
            for trailing in '\u{11a8}'..='\u{11c2}' {
                expected += 1;
                let lvt = char::from_u32(expected).unwrap();
                assert_eq!(
                    modern_hangul_syllable(&format!("{leading}{vowel}{trailing}")),
                    Some(lvt)
                );
                assert_eq!(
                    modern_hangul_syllable(&format!("{lv}{trailing}")),
                    Some(lvt)
                );
            }
            expected += 1;
        }
    }
    assert_eq!(expected, 0xd7a4);
}

#[test]
fn composition_retains_original_source_intervals_not_ligature_components() {
    for index in [0, 1, 6] {
        let font = font(index);
        for text in ["가", "각", "각"] {
            let source_len = text.chars().count();
            let input = format!("A{text}{text}B");
            let units = font
                .shape_text_units(&input, false, false, None, FontVariant::Normal)
                .unwrap();
            assert_eq!(units.len(), 4);
            for (i, unit) in units.iter().enumerate() {
                let ResolvedTextUnit::Glyph(glyph) = unit else {
                    panic!("glyph expected")
                };
                assert_eq!(glyph.ligature_components, 1);
                let span = match i {
                    0 => (0, 1),
                    1 => (1, 1 + source_len),
                    2 => (1 + source_len, 1 + 2 * source_len),
                    _ => (1 + 2 * source_len, 2 + 2 * source_len),
                };
                assert_eq!((glyph.source_start, glyph.source_end), span);
                if i == 1 || i == 2 {
                    assert_eq!(glyph.ch, text.chars().next().unwrap());
                    assert_eq!(glyph.shaping_ch, modern_hangul_syllable(text).unwrap());
                    assert_eq!(glyph.tibetan_cluster, None);
                }
            }
            // Source-bearing exports must not report normalized Hangul scalars.
            let exported = font.text2commands(&input).unwrap();
            assert_eq!(exported.len(), 4);
            assert_eq!(exported[1].ch, text.chars().next().unwrap());
            assert_eq!(exported[2].ch, text.chars().next().unwrap());
        }
    }
}

#[test]
fn exact_fixture_ids_cover_composed_only_and_no_partial_fallback() {
    for index in [0, 1, 4] {
        assert_eq!(font(index).debug_shape_glyph_ids("각", None).unwrap(), [5]);
        assert_eq!(font(index).debug_shape_glyph_ids("각", None).unwrap(), [5]);
    }
    assert_eq!(font(0).debug_shape_glyph_ids("힣", None).unwrap(), [6]);
    for index in [2, 3, 7] {
        assert_eq!(
            font(index).debug_shape_glyph_ids("각", None).unwrap(),
            [1, 2, 3]
        );
    }
    assert_eq!(font(3).debug_shape_glyph_ids("각", None).unwrap(), [4, 3]);
    assert_eq!(
        font(5).debug_shape_glyph_ids("각", None).unwrap(),
        [1, 0, 0]
    );
    assert_eq!(font(6).debug_shape_glyph_ids("각", None).unwrap(), [12]);
    assert_eq!(
        font(6).debug_shape_glyph_ids("A각B", None).unwrap(),
        [8, 13, 9]
    );
}

#[test]
fn excluded_complete_graphemes_keep_every_nominal_glyph() {
    let font = font(0);
    for text in [
        "",
        "ᄀ",
        "ᅡ",
        "ᆨ",
        "가",
        "각",
        "ᄀ가",
        "가ᅡ",
        "각ᆨ",
        "가\u{d7cb}",
        "\u{a960}ᅡᆨ",
        "ᄀ\u{d7b0}ᆨ",
        "가\u{d7cb}",
        "\u{115f}ᅡ",
        "ᄀ\u{1160}",
        "ㄱㅏ",
        "가\u{302e}",
        "가\u{302f}",
        "가\u{fe00}",
        "가\u{e0100}",
        "가\u{200d}",
        "가\u{200c}",
        "가\u{301}",
        "가\u{34f}",
        "\u{600}가",
        "각\u{301}",
        "\u{1113}ᅡ",
        "ᄀ\u{1176}",
        "가\u{11c3}",
        "각ᆨ",
    ] {
        assert!(modern_hangul_syllable(text).is_none(), "{text:?}");
        let expected: Vec<_> = Font::cluster_glyph_scalars(text)
            .into_iter()
            .map(|(ch, vs)| font.resolve_glyph_id_with_uvs(ch, vs, false).unwrap())
            .collect();
        assert_eq!(
            font.debug_shape_glyph_ids(text, None).unwrap(),
            expected,
            "{text:?}"
        );
    }
    assert!(modern_hangul_syllable(&format!("가{}", "ᆨ".repeat(100_000))).is_none());
}

#[test]
fn raw_scalar_resolution_and_downstream_gsub_semantics_remain_unchanged() {
    let font = font(0);
    assert_eq!(
        font.resolve_glyph_id_with_uvs('ᄀ', '\0', false).unwrap(),
        1
    );
    assert_eq!(font.resolve_glyph_id_with_uvs('ᅡ', '\0', false).unwrap(), 2);
    assert_eq!(
        font.resolve_glyph_id_with_uvs('각', '\0', false).unwrap(),
        5
    );
    // Newly composed output must not lose usable original jamo through GSUB.
    // Ordinary NFC still retains the existing downstream missing-glyph policy.
    let font = self::font(9);
    assert_eq!(font.debug_shape_glyph_ids("각", None).unwrap(), [1, 2, 3]);
    assert_eq!(font.debug_shape_glyph_ids("각", None).unwrap(), [0]);
}

#[test]
fn contextual_rollback_preserves_composed_only_neighbors_and_existing_missing_glyphs() {
    let font = font(10);
    for (text, expected) in [
        ("A각B", vec![8, 1, 2, 3, 9]),
        ("힣A각B", vec![6, 8, 1, 2, 3, 9]),
        ("힣A각B?", vec![6, 8, 1, 2, 3, 9, 0]),
        ("A각B", vec![8, 4, 3, 9]),
    ] {
        assert_eq!(
            font.debug_shape_glyph_ids(text, None).unwrap(),
            expected,
            "{text:?}"
        );
    }
}

#[cfg(feature = "cff")]
fn assert_truncated_cff_target_is_not_composed(data: &[u8]) {
    let mut font = font(0);
    let mut cff_font = Font::get_font_from_buffer(data).unwrap();
    font.cff = cff_font.cff.take();
    font.cff.as_mut().unwrap().char_string.data.data.truncate(4);
    font.glyf = None;
    assert_eq!(
        font.resolve_glyph_id_with_uvs('각', '\0', false).unwrap(),
        5
    );
    assert!(font.composed_modern_hangul_glyph("각", 0).is_none());
    assert_eq!(font.debug_shape_glyph_ids("각", None).unwrap(), [1, 2, 3]);
    let run = font
        .text2glyph_run("각", &crate::commands::FontOptions::from_parsed(&font))
        .unwrap();
    assert_eq!(run.glyphs.len(), 3);
    assert!(run.glyphs.iter().all(|g| !g.glyph.layers.is_empty()));
}

#[test]
#[cfg(feature = "cff")]
fn truncated_cff_charstrings_do_not_promote_valid_jamo_to_invalid_target() {
    assert_truncated_cff_target_is_not_composed(include_bytes!(
        "../../tests/fixtures/cff-vertical-origin.ttc"
    ));
}

#[test]
#[cfg(feature = "cff2")]
fn truncated_cff2_charstrings_do_not_promote_valid_jamo_to_invalid_target() {
    assert_truncated_cff_target_is_not_composed(include_bytes!(
        "../../tests/fixtures/cff2-vertical-origin.ttc"
    ));
}

#[test]
fn noncomposed_runs_cannot_enter_hangul_coverage_or_rollback_checks() {
    let font = font(6);
    for text in ["AB", "각가", "가\u{301}", "ᄀ가", "😀", "ཀི"] {
        let units = font
            .shape_text_units(text, false, false, None, FontVariant::Normal)
            .unwrap();
        assert!(
            units.iter().all(|unit| match unit {
                ResolvedTextUnit::Glyph(glyph) => !Font::is_composed_modern_hangul(glyph),
                _ => true,
            }),
            "{text:?}"
        );
    }
}

#[test]
fn old_hangul_preserves_source_intervals_and_disables_unproven_mark_ownership() {
    let mut font =
        Font::get_font_from_buffer(include_bytes!("../../tests/fixtures/bounded-hangul.ttc"))
            .unwrap();
    font.set_font(1).unwrap();
    let text = "\u{1100}\u{1161}\u{11eb}\u{a960}\u{d7b0}\u{d7cb}";
    let units = font
        .shape_text_units(text, false, false, None, FontVariant::Normal)
        .unwrap();
    assert_eq!(units.len(), 6);
    for (i, unit) in units.iter().enumerate() {
        let ResolvedTextUnit::Glyph(g) = unit else {
            panic!()
        };
        assert_eq!((g.source_start, g.source_end), (i, i + 1));
        assert_eq!(g.ch, text.chars().nth(i).unwrap());
        assert_eq!(g.ligature_components, 1);
        assert_eq!(g.simple_mkmk_cluster, None);
    }
}
#[test]
fn bounded_hangul_budget_and_length_fail_without_mutating_input() {
    use crate::opentype::extentions::gsub::GsubGlyph;
    let mut font =
        Font::get_font_from_buffer(include_bytes!("../../tests/fixtures/bounded-hangul.ttc"))
            .unwrap();
    font.set_font(1).unwrap();
    let mut gsub = font.current_gsub().unwrap().clone();
    let single = gsub.lookups.lookups[0].clone();
    gsub.lookups.lookups = vec![single; 100];
    for f in &mut gsub.features.features {
        f.lookup_list_indices = (0..100).collect();
    }
    let glyph = GsubGlyph {
        byte_start: 0,
        byte_end: 0,
        glyph_id: 1,
        source_index: 0,
        substituted: false,
        mkmk_source_ambiguous: false,
        ligature_components: 1,
    };
    let input = vec![glyph; 256];
    let original = input.clone();
    assert!(gsub
        .bounded_hangul(&input, &vec![1; 256], None, None)
        .is_none());
    assert_eq!(input, original);
    assert!(gsub
        .bounded_hangul(&vec![glyph; 257], &vec![1; 257], None, None)
        .is_none());
}

#[test]
fn bounded_vertical_hangul_retains_source_spans_and_raw_mapping() {
    let mut font = Font::get_font_from_buffer(include_bytes!(
        "../../tests/fixtures/bounded-vertical-hangul.ttc"
    ))
    .unwrap();
    font.set_font(2).unwrap();
    let text = "\u{1100}\u{1161}\u{11eb}";
    let units = font
        .shape_text_units(text, true, false, None, FontVariant::Normal)
        .unwrap();
    let ids = [7, 5, 6];
    for (i, unit) in units.iter().enumerate() {
        let ResolvedTextUnit::Glyph(g) = unit else {
            panic!()
        };
        assert_eq!(g.glyph_id, ids[i]);
        assert_eq!((g.source_start, g.source_end), (i, i + 1));
        assert_eq!(g.ch, text.chars().nth(i).unwrap());
        assert_eq!(g.simple_mkmk_cluster, None);
        assert_eq!(g.ligature_components, 1);
    }
    assert_eq!(
        font.resolve_glyph_id_with_uvs('\u{1100}', '\0', true)
            .unwrap(),
        1
    );
}

#[test]
#[cfg(feature = "layout")]
fn toned_old_hangul_keeps_source_ownership_after_visual_reordering() {
    let font =
        Font::get_font_from_buffer(include_bytes!("../../tests/fixtures/hangul-tone.ttc")).unwrap();
    let mut font = font;
    font.current_font = 1;
    let units = font
        .shape_text_units(
            "\u{1100}\u{1161}\u{11eb}\u{302e}\u{1100}\u{1161}\u{11eb}\u{302f}",
            false,
            false,
            None,
            crate::commands::FontVariant::Normal,
        )
        .unwrap();
    let spans: Vec<_> = units
        .iter()
        .filter_map(|g| {
            if let super::super::ResolvedTextUnit::Glyph(g) = g {
                Some((g.ch, g.source_start, g.source_end, g.simple_mkmk_cluster))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        spans,
        vec![
            ('\u{302e}', 3, 4, None),
            ('\u{1100}', 0, 1, None),
            ('\u{1161}', 1, 2, None),
            ('\u{11eb}', 2, 3, None),
            ('\u{302f}', 7, 8, None),
            ('\u{1100}', 4, 5, None),
            ('\u{1161}', 5, 6, None),
            ('\u{11eb}', 6, 7, None)
        ]
    );
}
