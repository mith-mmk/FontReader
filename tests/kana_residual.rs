//! Original MIT fixtures; no installed fonts required.
use fontcore::{FontFile, FontOptions};
const FONT: &[u8] = include_bytes!("fixtures/kana-residual.ttc");

#[test]
fn residual_policy_preserves_authored_and_nominal_boundaries() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for vertical in [false, true] {
        for index in 0..22 {
            let face = file.face(index).unwrap();
            let mut options = FontOptions::new(&face).with_font_size(1000.0);
            if vertical {
                options = options.with_vertical_flow();
            }
            let engine = face
                .engine()
                .with_options(options.clone())
                .with_locale("ja");
            let run = engine.shape("あ\u{3099}B").unwrap();
            if cfg!(feature = "layout") && [8, 21].contains(&index) {
                assert_eq!(run.glyphs.len(), 2);
                continue;
            }
            let mark = &run.glyphs[1];
            let advance = if vertical {
                mark.glyph.metrics.advance_y
            } else {
                mark.glyph.metrics.advance_x
            };
            let expected =
                if cfg!(feature = "layout") && [0, 1, 3, 9, 10, 14, 16, 17, 18].contains(&index) {
                    0.0
                } else {
                    1000.0
                };
            assert_eq!(advance, expected, "face {index} vertical {vertical}");
            if cfg!(feature = "layout") && [0, 1, 9, 16].contains(&index) {
                // Zero width does not imply attachment at the preceding origin.
                assert_eq!(
                    (mark.x, mark.y),
                    if vertical {
                        (0.0, 1000.0)
                    } else {
                        (1000.0, 0.0)
                    }
                );
                assert_eq!(
                    mark.glyph.metrics.bounds,
                    engine.shape("\u{3099}").unwrap().glyphs[0]
                        .glyph
                        .metrics
                        .bounds
                );
            }
        }
    }
}

#[test]
fn exact_original_kana_grapheme_only_and_trailing_cursor() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    for vertical in [false, true] {
        let mut options = FontOptions::new(&face).with_font_size(1000.0);
        if vertical {
            options = options.with_vertical_flow();
        }
        let engine = face.engine().with_options(options.clone());
        for base in ["あ", "ア", "ㇷ"] {
            for mark in ['\u{3099}', '\u{309a}'] {
                let run = engine.shape(&format!("{base}{mark}B")).unwrap();
                let expected = if cfg!(feature = "layout") {
                    1000.0
                } else {
                    2000.0
                };
                assert_eq!(
                    if vertical {
                        run.glyphs[2].y
                    } else {
                        run.glyphs[2].x
                    },
                    expected
                );
            }
        }
        for text in [
            "\u{3099}",
            "\u{309a}",
            "A\u{3099}",
            "あ\u{301}",
            "あ゛",
            "あ゜",
            "あ\u{3099}\u{309a}",
            "あ\u{3099}\u{fe0f}",
            "あ\n\u{3099}",
            "あ\t\u{3099}",
        ] {
            let run = engine.shape(text).unwrap();
            for glyph in run.glyphs.iter() {
                assert_eq!(
                    if vertical {
                        glyph.glyph.metrics.advance_y
                    } else {
                        glyph.glyph.metrics.advance_x
                    },
                    if text.starts_with('A')
                        && cfg!(feature = "layout")
                        && !vertical
                        && glyph.x == 0.0
                    {
                        1010.0
                    } else {
                        1000.0
                    },
                    "{text:?}"
                );
            }
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn composition_rollback_stays_outside_residual_fallback() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(16).unwrap();
    let run = face
        .engine()
        .with_options(FontOptions::new(&face).with_font_size(1000.0))
        .shape("か\u{3099}B")
        .unwrap();
    assert_eq!(run.glyphs.len(), 3);
    assert_eq!(run.glyphs[1].glyph.metrics.advance_x, 1000.0);
}

#[test]
#[cfg(feature = "layout")]
fn alternate_run_modes_and_authored_placement() {
    use fontcore::TextDirection;
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [0, 3] {
        let face = file.face(index).unwrap();
        let options = FontOptions::new(&face).with_font_size(1000.0);
        let engine = face.engine().with_options(options.clone());
        for run in [
            engine.shape("あ\u{3099}B").unwrap(),
            engine.shape_sideways_ltr("あ\u{3099}B").unwrap(),
            engine.shape_vertical_mixed_ltr("あ\u{3099}B").unwrap(),
        ] {
            if index == 0 {
                assert_eq!(run.glyphs[1].glyph.metrics.advance_x, 0.0);
                assert_eq!(run.glyphs[1].glyph.metrics.advance_y, 0.0);
            }
        }
        let rtl = face
            .engine()
            .with_options(
                options
                    .clone()
                    .with_text_direction(TextDirection::RightToLeft),
            )
            .shape("あ\u{3099}B")
            .unwrap();
        assert_eq!(rtl.glyphs[1].glyph.metrics.advance_x, 0.0);
        if index == 3 {
            let run = engine.shape("あ\u{3099}B").unwrap();
            assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (1025.0, -35.0));
            let run = face
                .engine()
                .with_options(options.with_vertical_flow())
                .shape("あ\u{3099}B")
                .unwrap();
            assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (50.0, 930.0));
        }
    }
}
