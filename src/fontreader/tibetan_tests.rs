use super::*;
use crate::fontreader::ResolvedTextUnit;

fn font() -> Font {
    Font::get_font_from_buffer(include_bytes!(
        "../../tests/fixtures/tibetan-normalization.ttc"
    ))
    .unwrap()
}

#[test]
fn original_scalar_grapheme_and_normalized_ligature_components_remain_distinct() {
    let font = font();
    for (text, source_len, components) in [
        ("\u{f43}", 1, 2),
        ("\u{f42}\u{fb7}", 2, 2),
        ("\u{f40}\u{f93}", 2, 3),
        ("\u{f40}\u{f75}", 2, 3),
        ("\u{f40}\u{f74}\u{f71}", 3, 3),
    ] {
        let text = text.repeat(2);
        let units = font
            .shape_text_units(&text, false, false, None, FontVariant::Normal)
            .unwrap();
        assert_eq!(units.len(), 2);
        for (index, unit) in units.iter().enumerate() {
            let ResolvedTextUnit::Glyph(glyph) = unit else {
                panic!("glyph expected")
            };
            assert_eq!(glyph.ch, text.chars().next().unwrap());
            assert_eq!(
                glyph.shaping_ch,
                if text.starts_with('\u{f43}') {
                    '\u{f42}'
                } else {
                    glyph.ch
                }
            );
            assert_eq!(glyph.tibetan_cluster, Some(index));
            assert_eq!(
                (glyph.source_start, glyph.source_end),
                (source_len * index, source_len * (index + 1))
            );
            assert_eq!(glyph.ligature_components, components);
        }
        // Legacy source-bearing exports must not expose the normalized scalar.
        let exported = font.text2commands(&text).unwrap();
        assert_eq!(exported.len(), 2);
        assert!(exported
            .iter()
            .all(|g| g.ch == text.chars().next().unwrap()));
    }
}

#[test]
fn reordered_candidate_moves_original_ownership_with_each_scalar() {
    let font = font();
    let make = |ch, index| ResolvedGlyph {
        ch,
        shaping_ch: ch,
        glyph_id: font.resolve_glyph_id_with_uvs(ch, '\0', false).unwrap(),
        prefer_color: false,
        ligature_components: 1,
        tibetan_cluster: Some(7),
        source_start: index,
        source_end: index + 1,
    };
    let original = ['\u{f40}', '\u{f74}', '\u{f71}']
        .into_iter()
        .enumerate()
        .map(|(i, ch)| make(ch, i))
        .collect::<Vec<_>>();
    let normalized = font.normalized_tibetan_glyphs(&original).unwrap();
    assert_eq!(
        normalized
            .iter()
            .map(|g| (g.ch, g.shaping_ch, g.source_start, g.tibetan_cluster))
            .collect::<Vec<_>>(),
        [
            ('ཀ', 'ཀ', 0, Some(7)),
            ('ཱ', 'ཱ', 2, Some(7)),
            ('ུ', 'ུ', 1, Some(7))
        ]
    );
    assert_eq!(original[1].ch, '\u{f74}');
}

#[test]
fn long_unsupported_combining_sequence_is_rejected_before_sorting() {
    let font = font();
    let mut input = vec![ResolvedGlyph {
        ch: '\u{f40}',
        shaping_ch: '\u{f40}',
        glyph_id: 1,
        prefer_color: false,
        ligature_components: 1,
        tibetan_cluster: Some(0),
        source_start: 0,
        source_end: 1,
    }];
    for index in 1..100_001 {
        let ch = if index % 2 == 0 { '\u{f71}' } else { '\u{f74}' };
        input.push(ResolvedGlyph {
            ch,
            shaping_ch: ch,
            source_start: index,
            source_end: index + 1,
            ..input[0]
        });
    }
    assert!(font.normalized_tibetan_glyphs(&input).is_none());
}

#[test]
fn direct_face_can_normalize_missing_original_but_family_probe_cannot_assume_it() {
    let mut font = font();
    font.set_font(2).unwrap();
    let text = "\u{f43}";
    assert_eq!(
        font.resolve_glyph_id_with_uvs('\u{f43}', '\0', false)
            .unwrap(),
        0
    );
    let units = font
        .shape_text_units(text, false, false, Some("tibt"), FontVariant::Normal)
        .unwrap();
    assert_eq!(units.len(), 1);
    let ResolvedTextUnit::Glyph(glyph) = units[0] else {
        panic!("glyph expected")
    };
    assert_ne!(glyph.glyph_id, 0);
    assert_eq!(glyph.ch, '\u{f43}');
    assert!(!font.supports_text_unit(
        &Font::parse_text_units_for_fallback(text)[0],
        crate::commands::TextDirection::LeftToRight,
        Some("tibt"),
        FontVariant::Normal
    ));
}
