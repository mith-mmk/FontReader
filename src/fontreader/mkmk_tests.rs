use super::*;

fn font() -> Font {
    Font::get_font_from_buffer(include_bytes!("../../tests/fixtures/mark-coordinates.ttf")).unwrap()
}

fn units(font: &Font, text: &str) -> Vec<ResolvedTextUnit> {
    font.shape_text_units(
        text,
        false,
        false,
        None,
        crate::commands::FontVariant::Normal,
    )
    .unwrap()
}

#[test]
fn simple_mkmk_guard_requires_complete_untouched_owner_span() {
    let font = font();
    let original = units(&font, "A\u{301}\u{301}\u{301}");
    let emitted = [Some(0), Some(1), Some(2), None];
    assert!(font.simple_mkmk_span(&original, &emitted, 3));
    // Mutation at the base must block too, not just target..current mutations.
    for index in 0..4 {
        for change in 0..7 {
            let mut changed = original.clone();
            let ResolvedTextUnit::Glyph(ref mut glyph) = changed[index] else {
                unreachable!()
            };
            match change {
                0 => glyph.simple_mkmk_cluster = None,
                1 => glyph.simple_mkmk_cluster = Some(99),
                2 => glyph.ligature_components = 2,
                3 => glyph.source_end += 1,
                4 => {
                    glyph.source_start += 1;
                    glyph.source_end += 1;
                }
                5 => glyph.glyph_id = 0, // unclassified
                _ => glyph.source_end = glyph.source_start,
            }
            assert!(
                !font.simple_mkmk_span(&changed, &emitted, 3),
                "index {index}, change {change}"
            );
        }
    }
    for index in 0..3 {
        let mut missing = emitted;
        missing[index] = None;
        assert!(!font.simple_mkmk_span(&original, &missing, 3));
        for boundary in [ResolvedTextUnit::Newline, ResolvedTextUnit::Tab] {
            let mut changed = original.clone();
            changed[index] = boundary;
            assert!(!font.simple_mkmk_span(&changed, &emitted, 3));
        }
    }
    let leading = units(&font, "\u{301}\u{301}\u{301}");
    assert!(!font.simple_mkmk_span(&leading, &[Some(0), Some(1), None], 2));
}

#[test]
fn simple_mkmk_metadata_preserves_graphemes_and_rejects_selectors_and_ligatures() {
    let font = font();
    let separate = units(&font, "A\u{301}A\u{301}");
    let clusters: Vec<_> = separate
        .iter()
        .map(|unit| match unit {
            ResolvedTextUnit::Glyph(glyph) => glyph.simple_mkmk_cluster,
            _ => None,
        })
        .collect();
    assert_eq!(clusters, [Some(0), Some(0), Some(1), Some(1)]);
    for text in ["A\u{fe00}\u{301}\u{301}", "AB\u{301}\u{301}"] {
        let shaped = units(&font, text);
        let emitted: Vec<_> = (0..shaped.len()).map(Some).collect();
        assert!(!font.simple_mkmk_span(&shaped, &emitted, shaped.len() - 1));
    }
}

#[test]
fn simple_mkmk_metadata_excludes_composition_restoration_and_tibetan_candidates() {
    for (bytes, text) in [
        (
            &include_bytes!("../../tests/fixtures/kana-composition.ttc")[..],
            "か\u{3099}",
        ),
        (
            &include_bytes!("../../tests/fixtures/hangul-composition.ttc")[..],
            "\u{1100}\u{1161}",
        ),
    ] {
        let font = Font::get_font_from_buffer(bytes).unwrap();
        let shaped = units(&font, text);
        assert_eq!(shaped.len(), 1);
        let ResolvedTextUnit::Glyph(glyph) = shaped[0] else {
            unreachable!()
        };
        assert!(glyph.simple_mkmk_cluster.is_none());
        let restored = font
            .restore_renderable_composition_sources(&[glyph])
            .unwrap();
        assert!(restored.iter().all(|g| g.simple_mkmk_cluster.is_none()));
        assert_eq!(restored.len(), 2);
    }
    let font = Font::get_font_from_buffer(include_bytes!(
        "../../tests/fixtures/tibetan-normalization.ttc"
    ))
    .unwrap();
    for text in ["\u{f43}", "\u{f40}\u{f75}"] {
        let shaped = units(&font, text);
        assert!(shaped.iter().all(|unit| match unit {
            ResolvedTextUnit::Glyph(glyph) => glyph.simple_mkmk_cluster.is_none(),
            _ => false,
        }));
    }
}

#[test]
fn simple_mkmk_provenance_survives_only_proven_single_paths() {
    let mut font =
        Font::get_font_from_buffer(include_bytes!("../../tests/fixtures/mkmk-predecessor.ttc"))
            .unwrap();
    for face in [16, 17, 26, 27, 28, 33, 34] {
        font.set_font(face).unwrap();
        let shaped = units(&font, "A\u{301}\u{302}\u{301}");
        let emitted = [Some(0), Some(1), Some(2), None];
        assert!(font.simple_mkmk_span(&shaped, &emitted, 3), "face {face}");
    }
    for face in [18, 19, 20, 24, 25, 29, 30, 31, 32] {
        font.set_font(face).unwrap();
        let text = if matches!(face, 20 | 25 | 32) {
            "AA\u{301}\u{302}\u{301}"
        } else {
            "A\u{301}\u{302}\u{301}"
        };
        let shaped = units(&font, text);
        let emitted: Vec<_> = (0..shaped.len()).map(Some).collect();
        assert!(
            !font.simple_mkmk_span(&shaped, &emitted, shaped.len() - 1),
            "face {face}"
        );
    }
}
