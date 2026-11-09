use fontcore::{FontFile, FontOptions, FontStretch, GlyphBounds, ShapingPolicy};

const FONT: &[u8] = include_bytes!("fixtures/pair-coordinates.ttc");
const BASE: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

fn extend(bounds: &mut Option<GlyphBounds>, x: f32, y: f32) {
    if let Some(b) = bounds {
        b.min_x = b.min_x.min(x);
        b.min_y = b.min_y.min(y);
        b.max_x = b.max_x.max(x);
        b.max_y = b.max_y.max(y);
    } else {
        *bounds = Some(GlyphBounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        });
    }
}

fn check_bounds(actual: Option<GlyphBounds>, expected: Option<GlyphBounds>) {
    let (a, e) = (actual.unwrap(), expected.unwrap());
    for (actual, expected) in [
        (a.min_x, e.min_x),
        (a.min_y, e.min_y),
        (a.max_x, e.max_x),
        (a.max_y, e.max_y),
    ] {
        close(actual, expected);
    }
}

#[test]
fn pair_placement_and_directional_advances_are_converted_once() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let base_file = FontFile::from_buffer(BASE).unwrap();
    let base_face = base_file.current_face().unwrap();
    for face_index in 0..2 {
        let face = file.face(face_index).unwrap();
        for policy in [
            ShapingPolicy::LeftToRight,
            ShapingPolicy::RightToLeft,
            ShapingPolicy::TopToBottom,
        ] {
            let vertical = policy == ShapingPolicy::TopToBottom;
            let rtl = policy == ShapingPolicy::RightToLeft;
            for (size, stretch) in [(100.0, 1.0), (50.0, 2.0), (200.0, 0.5)] {
                let sx = size / 1000.0 * stretch;
                let sy = size / 1000.0;
                let options = FontOptions::new(&face)
                    .with_font_size(size)
                    .with_font_stretch(FontStretch(stretch));
                let engine = face
                    .engine()
                    .with_options(options)
                    .with_shaping_policy(policy);
                let base_options = FontOptions::new(&base_face)
                    .with_font_size(size)
                    .with_font_stretch(FontStretch(stretch));
                let base_engine = base_face
                    .engine()
                    .with_options(base_options)
                    .with_shaping_policy(policy);
                for (text, values) in [
                    (
                        "AB",
                        [(20.0, 100.0, -40.0, 120.0), (-30.0, -200.0, 60.0, -80.0)],
                    ),
                    (
                        "BA",
                        [(-20.0, -100.0, 40.0, -120.0), (30.0, 200.0, -60.0, 80.0)],
                    ),
                    ("AA", [(10.0, 0.0, 20.0, 0.0), (-10.0, 0.0, -20.0, 0.0)]),
                ] {
                    let run = engine.shape(text).unwrap();
                    let baseline = base_engine.shape(text).unwrap();
                    let mut cursor = (0.0, 0.0);
                    let mut ink = None;
                    let mut lines = None;
                    let mut inline = 0.0;
                    for ((glyph, base), values) in
                        run.glyphs.iter().zip(&baseline.glyphs).zip(values)
                    {
                        let (xp, yp, xa, ya) = if cfg!(feature = "layout") {
                            values
                        } else {
                            (0.0, 0.0, 0.0, 0.0)
                        };
                        let ax = base.glyph.metrics.advance_x + xa * sx;
                        // YAdvance changes the directional advance height, unlike
                        // Cartesian YPlacement: positive vkrn increases TTB advance.
                        let ay = base.glyph.metrics.advance_y + ya * sy;
                        let x = cursor.0 + xp * sx - if rtl { ax } else { 0.0 };
                        let y = cursor.1 - yp * sy;
                        close(glyph.x, x);
                        close(glyph.y, y);
                        close(glyph.glyph.metrics.advance_x, ax);
                        close(glyph.glyph.metrics.advance_y, ay);
                        assert_eq!(glyph.glyph.metrics.bounds, base.glyph.metrics.bounds);
                        let b = base.glyph.metrics.bounds.unwrap();
                        extend(&mut ink, x + b.min_x, y + b.min_y);
                        extend(&mut ink, x + b.max_x, y + b.max_y);
                        extend(&mut lines, x, y);
                        extend(
                            &mut lines,
                            x + if vertical { 0.0 } else { ax },
                            y + if vertical { ay } else { 0.0 },
                        );
                        cursor.0 += if rtl { -ax } else { ax };
                        cursor.1 += ay;
                        inline += if vertical { ay } else { ax };
                    }
                    let measured = engine.measure_layout(text).unwrap();
                    close(measured.inline_advance as f32, inline);
                    close(engine.measure(text).unwrap() as f32, inline);
                    check_bounds(measured.ink_bounds, ink);
                    check_bounds(measured.line_bounds, lines);
                    let line = lines.unwrap();
                    close(
                        measured.block_extent as f32,
                        if vertical {
                            line.max_x - line.min_x
                        } else {
                            line.max_y - line.min_y
                        },
                    );
                }
            }
        }
    }
}

#[cfg(feature = "layout")]
#[test]
fn signed_pair_placement_reaches_svg_and_sideways_rotation() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for face_index in 0..2 {
        let face = file.face(face_index).unwrap();
        let engine = face.engine().with_font_size(100.0);
        let horizontal = engine.shape("AB").unwrap();
        close(horizontal.glyphs[0].y, -10.0);
        close(horizontal.glyphs[1].y, 32.0);
        let sideways = engine.shape_sideways_ltr("AB").unwrap();
        for (rotated, glyph) in sideways.glyphs.iter().zip(&horizontal.glyphs) {
            close(rotated.x, -glyph.y);
            close(rotated.y, glyph.x);
            close(
                rotated.glyph.metrics.advance_x,
                -glyph.glyph.metrics.advance_y,
            );
            close(
                rotated.glyph.metrics.advance_y,
                glyph.glyph.metrics.advance_x,
            );
        }
        let h = engine.measure_layout("AB").unwrap();
        let s = engine.measure_layout_sideways_ltr("AB").unwrap();
        assert_eq!(h.inline_advance, 162.0);
        assert_eq!(s.inline_advance, 162.0);
        assert_eq!(engine.measure_sideways_ltr("AB").unwrap(), 162.0);
        check_bounds(
            s.line_bounds,
            Some(GlyphBounds {
                min_x: -32.0,
                min_y: 2.0,
                max_x: 10.0,
                max_y: 159.0,
            }),
        );
        assert_eq!(s.block_extent, 42.0);
        let b = h.ink_bounds.unwrap();
        check_bounds(
            s.ink_bounds,
            Some(GlyphBounds {
                min_x: -b.max_y,
                min_y: b.min_x,
                max_x: -b.min_y,
                max_y: b.max_x,
            }),
        );
        // Absolute ink is (7,-80)..(133,42); SVG adds four units on each side.
        assert!(engine
            .render_svg("AB")
            .unwrap()
            .contains("viewBox=\"3 -84 134 130\""));
        assert!(engine
            .render_svg_sideways_ltr("AB")
            .unwrap()
            .contains("viewBox=\"-46 3 130 134\""));
        assert!(face
            .engine()
            .with_font_size(100.0)
            .with_shaping_policy(ShapingPolicy::RightToLeft)
            .render_svg("AB")
            .unwrap()
            .contains("viewBox=\"-159 -84 154 130\""));
        let vertical = engine.with_vertical_flow();
        // Vertical origins differ from horizontal ones, not from the GPOS sign.
        assert!(vertical
            .render_svg("AB")
            .unwrap()
            .contains("viewBox=\"-51 -4 78 225\""));
    }
}
