use super::*;

#[test]
fn all_pinned_canonical_pairs_and_only_those_pairs_are_admitted() {
    let source = include_str!("../../data/unicode/17.0.0/UnicodeData.txt");
    let rows: std::collections::HashMap<_, _> = source
        .lines()
        .map(|line| {
            let row: Vec<_> = line.split(';').collect();
            (u32::from_str_radix(row[0], 16).unwrap(), row)
        })
        .collect();
    let exclusions: Vec<_> = include_str!("../../data/unicode/17.0.0/CompositionExclusions.txt")
        .lines()
        .filter_map(|line| {
            let field = line.split('#').next().unwrap().trim();
            (!field.is_empty()).then(|| u32::from_str_radix(field, 16).unwrap())
        })
        .collect();
    let mut expected = Vec::new();
    for row in source
        .lines()
        .map(|line| line.split(';').collect::<Vec<_>>())
    {
        let cp = u32::from_str_radix(row[0], 16).unwrap();
        if !(0x3040..=0x30ff).contains(&cp) || row[5].is_empty() || row[5].starts_with('<') {
            continue;
        }
        let parts: String = row[5]
            .split_whitespace()
            .map(|s| char::from_u32(u32::from_str_radix(s, 16).unwrap()).unwrap())
            .collect();
        assert_eq!(parts.chars().count(), 2);
        assert_eq!(row[3], "0");
        let base = &rows[&(parts.chars().next().unwrap() as u32)];
        assert_eq!(base[3], "0");
        assert!(base[5].is_empty());
        assert!(!exclusions.contains(&cp));
        let target = char::from_u32(cp).unwrap();
        assert_eq!(composed_kana(&parts), Some(target));
        assert_eq!(composed_kana(&format!("{parts}\u{301}")), None);
        expected.push((target, parts.chars().collect::<Vec<_>>()));
    }
    assert_eq!(expected.len(), 58);
    assert_eq!(DECOMPOSITIONS.len(), expected.len());
    for (target, pair) in DECOMPOSITIONS {
        assert!(expected.contains(&(*target, pair.to_vec())));
    }
}

#[test]
fn admission_is_complete_cluster_bounded_and_excludes_ainu_named_sequences() {
    for text in [
        "",
        "ハ",
        "パ",
        "ハ\u{309a}\u{309a}",
        "ハ\u{309a}\u{fe00}",
        "ハ\u{309a}\u{e0100}",
        "ハ\u{309a}\u{200d}",
        "ハ\u{309a}\u{200c}",
        "\u{600}ハ\u{309a}",
        "ハ\u{309a}\u{34f}",
        "ﾊ\u{ff9f}",
        "A\u{3099}",
        "セ\u{309a}",
        "ツ\u{309a}",
        "ト\u{309a}",
        "ㇷ\u{309a}",
        "가",
        "ཀི",
    ] {
        assert_eq!(composed_kana(text), None, "{text:?}");
    }
    assert_eq!(
        composed_kana(&format!("ハ{}", "\u{309a}".repeat(100_000))),
        None
    );
}

fn font(index: usize) -> Font {
    let mut font =
        Font::get_font_from_buffer(include_bytes!("../../tests/fixtures/kana-composition.ttc"))
            .unwrap();
    font.set_font(index).unwrap();
    font
}

#[test]
fn sources_and_spans_survive_composition_and_mixed_rollback() {
    use crate::commands::FontVariant;
    use crate::fontreader::ResolvedTextUnit;
    for (index, text, spans) in [
        (0, "Aハ\u{309a}각B", vec![(0, 1), (1, 3), (3, 6), (6, 7)]),
        (
            6,
            "각ハ\u{309a}",
            vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)],
        ),
    ] {
        let units = font(index)
            .shape_text_units(text, false, false, None, FontVariant::Normal)
            .unwrap();
        assert_eq!(units.len(), spans.len());
        for (unit, (start, end)) in units.iter().zip(spans) {
            let ResolvedTextUnit::Glyph(g) = unit else {
                panic!("glyph expected")
            };
            assert_eq!((g.source_start, g.source_end), (start, end));
            assert_eq!(g.ch, text.chars().nth(start).unwrap());
            assert_eq!(g.ligature_components, 1);
            assert_eq!(g.tibetan_cluster, None);
        }
    }
}

#[cfg(feature = "cff")]
fn truncated_cff_target_is_not_composed(data: &[u8]) {
    let mut font = font(0);
    let mut donor = Font::get_font_from_buffer(data).unwrap();
    font.cff = donor.cff.take();
    let strings = &mut font.cff.as_mut().unwrap().char_string.data.data;
    let valid = strings[1].clone();
    strings.resize(99, valid);
    strings.truncate(99);
    font.glyf = None;
    assert!(font.composed_kana_glyph("ハ\u{309a}", 0).is_none());
    assert_eq!(
        font.debug_shape_glyph_ids("ハ\u{309a}", None).unwrap(),
        [97, 63]
    );
    let run = font
        .text2glyph_run(
            "ハ\u{309a}",
            &crate::commands::FontOptions::from_parsed(&font),
        )
        .unwrap();
    assert_eq!(run.glyphs.len(), 2);
    assert!(run.glyphs.iter().all(|g| !g.glyph.layers.is_empty()));
}

#[test]
#[cfg(feature = "cff")]
fn cff_charstrings_bounds_keep_original_kana() {
    truncated_cff_target_is_not_composed(include_bytes!(
        "../../tests/fixtures/cff-vertical-origin.ttc"
    ));
}

#[test]
#[cfg(feature = "cff2")]
fn cff2_charstrings_bounds_keep_original_kana() {
    truncated_cff_target_is_not_composed(include_bytes!(
        "../../tests/fixtures/cff2-vertical-origin.ttc"
    ));
}

#[test]
fn unrelated_missing_glyph_keeps_conservative_whole_run_rollback() {
    // Without GSUB provenance, a missing neighbor also restores valid new kana.
    // This inherits the bounded Hangul policy, not universal NFC/NFD equality.
    let font = font(4);
    assert_eq!(
        font.debug_shape_glyph_ids("ハ\u{309a}", None).unwrap(),
        [144]
    );
    assert_eq!(
        font.debug_shape_glyph_ids("ハ\u{309a}?", None).unwrap(),
        [97, 63, 0]
    );
    assert_eq!(font.debug_shape_glyph_ids("パ?", None).unwrap(), [144, 0]);
}
