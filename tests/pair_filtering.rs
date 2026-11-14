#![cfg(feature = "layout")]
use fontcore::{FontFile, FontOptions, FontStretch, ShapingPolicy};

const FONT: &[u8] = include_bytes!("fixtures/pair-filtering.ttc");
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

#[test]
fn pair_lookup_selects_its_own_neighbors_and_applies_both_records() {
    let file = FontFile::from_buffer(FONT).unwrap();
    // Face, input, first and second affected indices. None means no match.
    for (face_index, text, pair) in [
        (1, "A\u{301}B", None),
        (1, "AB", Some((0, 1))),
        (2, "A\u{301}\u{300}B", Some((0, 3))),
        (3, "\u{301}AB\u{300}", Some((0, 3))),
        (3, "AB", None),
        (4, "A\u{e000}B", Some((0, 2))),
        (4, "ACB", None),
        (5, "A\u{300}B", Some((0, 2))),
        (5, "A\u{301}B", Some((0, 1))),
        (6, "A\u{300}B", Some((0, 2))),
        (6, "A\u{301}B", Some((0, 1))),
        (7, "A\u{300}B", Some((0, 2))),
        (7, "A\u{301}B", Some((0, 1))),
        (8, "A\u{301}B", Some((0, 2))),
        (9, "A\u{301}B", None),
        (9, "AB", Some((0, 1))),
        (10, "A\u{301}B", Some((0, 2))),
        (11, "A\u{301}B", Some((0, 2))),
        (2, "A\nB", None),
        (2, "A\tB", None),
    ] {
        let face = file.face(face_index).unwrap();
        let baseline = file.face(face_index + 15).unwrap();
        for policy in [
            ShapingPolicy::LeftToRight,
            ShapingPolicy::RightToLeft,
            ShapingPolicy::TopToBottom,
        ] {
            for (size, stretch) in [(1000.0, 1.0), (125.0, 2.0)] {
                let scale = size / 1000.0;
                let scale_x = scale * stretch;
                let engine = face
                    .engine()
                    .with_options(
                        FontOptions::new(&face)
                            .with_font_size(size)
                            .with_font_stretch(FontStretch(stretch)),
                    )
                    .with_shaping_policy(policy);
                let base_engine = baseline
                    .engine()
                    .with_options(
                        FontOptions::new(&baseline)
                            .with_font_size(size)
                            .with_font_stretch(FontStretch(stretch)),
                    )
                    .with_shaping_policy(policy);
                let run = engine.shape(text).unwrap();
                let base = base_engine.shape(text).unwrap();
                assert_eq!(run.glyphs.len(), base.glyphs.len());
                assert_eq!(
                    run.glyphs.len(),
                    text.chars().filter(|ch| !['\n', '\t'].contains(ch)).count()
                );
                let mut delta = (0.0, 0.0);
                let mut total = 0.0;
                for (i, (glyph, plain)) in run.glyphs.iter().zip(&base.glyphs).enumerate() {
                    let (xp, yp, xa, ya) = match pair {
                        Some((first, _)) if i == first => (10.0, 20.0, -30.0, 40.0),
                        Some((_, second)) if i == second => (-50.0, -60.0, 70.0, -80.0),
                        _ => (0.0, 0.0, 0.0, 0.0),
                    };
                    close(
                        glyph.glyph.metrics.advance_x,
                        plain.glyph.metrics.advance_x + xa * scale_x,
                    );
                    close(
                        glyph.glyph.metrics.advance_y,
                        plain.glyph.metrics.advance_y + ya * scale,
                    );
                    // Existing GDEF attachment-class fallback stays separate
                    // from PairPos filtering and uses the base glyph's origin.
                    let attached = i == 1
                        && (([5, 8].contains(&face_index) && text.contains('\u{301}'))
                            || (face_index == 7 && text.contains('\u{300}')));
                    let origin_delta = if attached {
                        (
                            run.glyphs[0].x - base.glyphs[0].x,
                            run.glyphs[0].y - base.glyphs[0].y,
                        )
                    } else {
                        delta
                    };
                    close(
                        glyph.x,
                        plain.x + origin_delta.0 + xp * scale_x
                            - if !attached && policy == ShapingPolicy::RightToLeft {
                                xa * scale_x
                            } else {
                                0.0
                            },
                    );
                    close(glyph.y, plain.y + origin_delta.1 - yp * scale);
                    delta.0 += if policy == ShapingPolicy::RightToLeft {
                        -xa * scale_x
                    } else {
                        xa * scale_x
                    };
                    delta.1 += ya * scale;
                    total += if policy == ShapingPolicy::TopToBottom {
                        ya * scale
                    } else {
                        xa * scale_x
                    };
                }
                close(
                    engine.measure(text).unwrap() as f32,
                    base_engine.measure(text).unwrap() as f32 + total,
                );
                assert_eq!(
                    engine.measure_layout(text).unwrap().inline_advance,
                    engine.measure(text).unwrap()
                );
                assert!(engine.render_svg(text).unwrap().contains("viewBox="));
            }
        }
    }
}

#[test]
fn independent_lookups_choose_different_neighbors() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(12).unwrap();
    let run = face
        .engine()
        .with_font_size(1000.0)
        .shape("A\u{301}B")
        .unwrap();
    close(run.glyphs[0].glyph.metrics.advance_x, 473.0);
    close(run.glyphs[1].glyph.metrics.advance_x, 7.0);
    close(run.glyphs[2].glyph.metrics.advance_x, 570.0);
    close(run.glyphs[0].x, 11.0);
    close(run.glyphs[0].y, -22.0);
    close(run.glyphs[1].x, 478.0);
    close(run.glyphs[2].x, 430.0);
}

#[test]
fn second_value_format_controls_next_pair_even_when_record_is_zero() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (face_index, expected) in [(13, 1490.0), (14, 1470.0)] {
        let face = file.face(face_index).unwrap();
        assert_eq!(
            face.engine().with_font_size(1000.0).measure("ABC").unwrap(),
            expected
        );
    }
}

#[test]
fn single_and_pair_values_accumulate_on_attached_marks() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(30).unwrap();
    let engine = face.engine().with_font_size(1000.0);
    let run = engine.shape("A\u{301}").unwrap();
    assert_eq!(run.glyphs.len(), 2);
    close(run.glyphs[0].x, 11.0);
    close(run.glyphs[0].y, -22.0);
    close(run.glyphs[0].glyph.metrics.advance_x, 473.0);
    close(run.glyphs[0].glyph.metrics.advance_y, 44.0);
    close(run.glyphs[1].x, 266.0);
    close(run.glyphs[1].y, -468.0);
    close(run.glyphs[1].glyph.metrics.advance_x, 77.0);
    close(run.glyphs[1].glyph.metrics.advance_y, -72.0);
    assert_eq!(engine.measure("A\u{301}").unwrap(), 550.0);
    // Pair placements and attachment survive the SVG bounds/origin pipeline.
    assert!(engine
        .render_svg("A\u{301}")
        .unwrap()
        .contains("viewBox=\"57 -872 563 854\""));
}

#[test]
fn pair_filtering_preserves_locale_script_selection() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(31).unwrap();
    for (locale, advance) in [("default", 990.0), ("ar", 910.0)] {
        let engine = face.engine().with_options(
            FontOptions::new(&face)
                .with_font_size(1000.0)
                .with_locale(locale),
        );
        assert_eq!(engine.measure("A\u{301}B").unwrap(), advance);
    }
}

#[test]
fn extension_wrapped_class_pairs_filter_marks() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(32).unwrap();
    let run = face
        .engine()
        .with_font_size(1000.0)
        .shape("A\u{301}B")
        .unwrap();
    assert_eq!(run.glyphs.len(), 3);
    close(run.glyphs[0].glyph.metrics.advance_x, 470.0);
    close(run.glyphs[1].glyph.metrics.advance_x, 0.0);
    close(run.glyphs[2].glyph.metrics.advance_x, 570.0);
}
