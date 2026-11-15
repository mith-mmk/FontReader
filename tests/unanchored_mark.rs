use fontcore::FontFile;
#[cfg(feature = "layout")]
use fontcore::{FontOptions, TextDirection};
const FONT: &[u8] = include_bytes!("fixtures/unanchored-mark.ttc");

#[test]
#[cfg(feature = "layout")]
fn gdef_metadata_does_not_invent_gpos_attachment_origins() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for mode in 0..10 {
        let face_base = (mode / 5) * 36 + (mode % 5) * 6;
        for advance in 0..2 {
            for evidence in 1..3 {
                for direction in [
                    TextDirection::LeftToRight,
                    TextDirection::RightToLeft,
                    TextDirection::TopToBottom,
                ] {
                    for orientation in 0..3 {
                        let expected_face = file.face(face_base).unwrap();
                        let geometry_face = file.face(face_base + advance * 3).unwrap();
                        let geometry_engine = geometry_face.engine().with_options(
                            FontOptions::new(&geometry_face)
                                .with_font_size(1000.0)
                                .with_text_direction(direction),
                        );
                        let actual_face = file.face(face_base + advance * 3 + evidence).unwrap();
                        let expected_engine = expected_face.engine().with_options(
                            FontOptions::new(&expected_face)
                                .with_font_size(1000.0)
                                .with_text_direction(direction),
                        );
                        let actual_engine = actual_face.engine().with_options(
                            FontOptions::new(&actual_face)
                                .with_font_size(1000.0)
                                .with_text_direction(direction),
                        );
                        for text in [
                            "A\u{301}",
                            "A\u{301}B",
                            "A\u{301}\u{301}",
                            "\u{301}",
                            "A\u{301}\u{10ffff}",
                            "A\u{3001}\u{301}B",
                        ] {
                            let shape = |engine: &fontcore::FontEngine| match orientation {
                                1 => engine.shape_vertical_mixed_ltr(text),
                                2 => engine.shape_sideways_ltr(text),
                                _ => engine.shape(text),
                            };
                            let expected = shape(&expected_engine).unwrap();
                            let actual = shape(&actual_engine).unwrap();
                            let geometry = shape(&geometry_engine).unwrap();
                            assert_eq!(actual.glyphs.len(), geometry.glyphs.len());
                            for (a, e) in actual.glyphs.iter().zip(&geometry.glyphs) {
                                assert_eq!(
                                    format!("{:?}", a.glyph.layers),
                                    format!("{:?}", e.glyph.layers)
                                );
                                assert_eq!(
                                    format!("{:?}", a.glyph.metrics.bounds),
                                    format!("{:?}", e.glyph.metrics.bounds)
                                );
                            }
                            assert_eq!(actual.glyphs.len(), expected.glyphs.len());
                            // A leading mark has no eligible previous spacing glyph;
                            // its historical positive nominal advance is retained.
                            if text.starts_with('\u{301}') && advance != 0 {
                                if orientation == 0 && direction == TextDirection::LeftToRight {
                                    let authored = if mode % 5 == 3 { 31.0 } else { 0.0 };
                                    assert_eq!(
                                        actual.glyphs[0].glyph.metrics.advance_x,
                                        150.0 + authored
                                    );
                                }
                                continue;
                            }
                            for (a, e) in actual.glyphs.iter().zip(&expected.glyphs) {
                                if advance == 0 {
                                    assert_eq!(
                                        format!("{:?}", a.glyph.layers),
                                        format!("{:?}", e.glyph.layers)
                                    );
                                    assert_eq!(
                                        format!("{:?}", a.glyph.metrics.bounds),
                                        format!("{:?}", e.glyph.metrics.bounds)
                                    );
                                }
                                assert_eq!((a.x, a.y, a.glyph.metrics.advance_x, a.glyph.metrics.advance_y),
                                           (e.x, e.y, e.glyph.metrics.advance_x, e.glyph.metrics.advance_y),
                                           "mode {mode} advance {advance} evidence {evidence} direction {direction:?} orientation {orientation} text {text:?}");
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn no_gpos_and_no_layout_keep_historical_policy() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for advance in 0..2 {
        for evidence in 0..3 {
            let face = file.face(30 + advance * 3 + evidence).unwrap();
            let run = face
                .engine()
                .with_font_size(1000.0)
                .shape("A\u{301}B")
                .unwrap();
            let fallback = cfg!(feature = "layout") && evidence != 0;
            assert_eq!(run.glyphs.len(), 3);
            assert_eq!(run.glyphs[1].x, if fallback { 0.0 } else { 600.0 });
            assert_eq!(
                run.glyphs[1].glyph.metrics.advance_x,
                if fallback {
                    0.0
                } else {
                    advance as f32 * 150.0
                }
            );
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn authored_single_and_pair_adjustments_survive_nominal_zeroing() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (mode, x, y) in [(3, 617.0, -23.0), (4, 606.0, -10.0)] {
        for evidence in 1..3 {
            let face = file.face(mode * 6 + 3 + evidence).unwrap();
            let run = face
                .engine()
                .with_font_size(1000.0)
                .shape("A\u{301}B")
                .unwrap();
            let mark = &run.glyphs[1];
            assert_eq!((mark.x, mark.y), (x, y));
            assert_eq!(
                (mark.glyph.metrics.advance_x, mark.glyph.metrics.advance_y),
                (31.0, 43.0)
            );
        }
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn all_gpos_metadata_variants_are_inert_without_layout() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for vertical in 0..2 {
        for advance in 0..2 {
            let baseline = file.face(vertical * 36 + advance * 3).unwrap();
            for mode in 0..6 {
                for evidence in 0..3 {
                    let face = file
                        .face(vertical * 36 + mode * 6 + advance * 3 + evidence)
                        .unwrap();
                    for text in ["A\u{301}B", "A、\u{301}B", "\u{301}"] {
                        assert_eq!(
                            format!("{:?}", face.engine().shape(text).unwrap()),
                            format!("{:?}", baseline.engine().shape(text).unwrap())
                        );
                    }
                }
            }
        }
    }
}
