use fontcore::{FontFile, GlyphBounds, GlyphLayer, GlyphPaint};
#[cfg(feature = "layout")]
use fontcore::{FontOptions, FontStretch};

const FONT: &[u8] = include_bytes!("fixtures/colr-vertical-origin.ttc");

fn bounds(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> GlyphBounds {
    GlyphBounds {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

#[test]
fn colr_composition_keeps_paints_and_uses_one_base_origin() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = engine.shape("\u{3001}\u{3001}").unwrap();
    let (x, y) = if cfg!(feature = "layout") {
        (-30.0, 80.0)
    } else {
        (0.0, 0.0)
    };
    for glyph in &run.glyphs {
        assert_eq!(glyph.glyph.layers.len(), 2);
        for (layer, paint) in glyph.glyph.layers.iter().zip([0xffff0000, 0xff0000ff]) {
            let GlyphLayer::Path(path) = layer else {
                panic!("expected COLR path")
            };
            assert_eq!(path.paint, GlyphPaint::Solid(paint));
            assert_eq!((path.offset_x, path.offset_y), (x, y));
        }
        assert_eq!(
            glyph.glyph.metrics.bounds,
            Some(bounds(5.0 + x, -70.0 + y, 80.0 + x, 10.0 + y))
        );
    }
    let advance = if cfg!(feature = "layout") {
        120.0
    } else {
        100.0
    };
    assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (0.0, advance));
    assert_eq!(
        engine.measure("\u{3001}\u{3001}").unwrap(),
        f64::from(advance) * 2.0
    );
    assert_eq!(
        engine
            .measure_layout("\u{3001}\u{3001}")
            .unwrap()
            .ink_bounds,
        Some(bounds(5.0 + x, -70.0 + y, 80.0 + x, 10.0 + y + advance))
    );
    let svg = engine.render_svg("\u{3001}").unwrap();
    assert!(svg.contains("#ff0000") && svg.contains("#0000ff"), "{svg}");
    if cfg!(feature = "layout") {
        assert!(svg.contains("viewBox=\"-29 6 83 88\""), "{svg}");
    }
    let horizontal = face
        .engine()
        .with_font_size(100.0)
        .shape("\u{3001}")
        .unwrap();
    assert_eq!(
        horizontal.glyphs[0].glyph.metrics.bounds,
        Some(bounds(5.0, -70.0, 80.0, 10.0))
    );
    for layer in &horizontal.glyphs[0].glyph.layers {
        let GlyphLayer::Path(path) = layer else {
            panic!("expected path")
        };
        assert_eq!((path.offset_x, path.offset_y), (0.0, 0.0));
    }
}

#[cfg(feature = "layout")]
#[test]
fn colr_color_marks_inherit_base_origin_through_mark_chains() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [0, 6] {
        let face = file.face(index).unwrap();
        let run = face
            .engine()
            .with_font_size(100.0)
            .with_vertical_flow()
            .shape("\u{3001}\u{0301}\u{0301}B")
            .unwrap();
        for i in [1, 2] {
            let GlyphLayer::Path(path) = &run.glyphs[i].glyph.layers[0] else {
                panic!("expected path")
            };
            assert_eq!(path.paint, GlyphPaint::CurrentColor);
            assert_eq!((path.offset_x, path.offset_y), (-30.0, 80.0));
            assert_eq!(run.glyphs[i].glyph.metrics.advance_y, 0.0);
        }
        assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (20.0, 0.0));
        assert_eq!((run.glyphs[2].x, run.glyphs[2].y), (20.0, -10.0));
        assert_eq!(run.glyphs[3].y, 120.0);
    }
}

#[cfg(feature = "layout")]
#[test]
fn colr_negative_bearings_scale_and_stretch_once() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [1, 7] {
        let face = file.face(index).unwrap();
        for stretch in [0.0, 1.5] {
            let options = FontOptions::new(&face)
                .with_font_size(200.0)
                .with_font_stretch(FontStretch(stretch))
                .with_vertical_flow();
            let engine = face.engine().with_options(options);
            let run = engine.shape("\u{3001}").unwrap();
            let actual = run.glyphs[0].glyph.metrics.bounds.unwrap();
            let expected = bounds(-50.0 * stretch, -60.0, 100.0 * stretch, 100.0);
            for (a, b) in [
                (actual.min_x, expected.min_x),
                (actual.min_y, expected.min_y),
                (actual.max_x, expected.max_x),
                (actual.max_y, expected.max_y),
            ] {
                assert!((a - b).abs() < 0.0001, "{a} != {b}");
            }
            for layer in &run.glyphs[0].glyph.layers {
                let GlyphLayer::Path(path) = layer else {
                    panic!("expected path")
                };
                assert!((path.offset_x + 60.0 * stretch).abs() < 0.0001);
                assert_eq!(path.offset_y, 80.0);
            }
            assert_eq!(engine.measure("\u{3001}").unwrap(), 240.0);
        }
    }
}

#[test]
fn colr_missing_metrics_and_variable_origins_remain_unchanged() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [2, 5] {
        let face = file.face(index).unwrap();
        let run = face
            .engine()
            .with_font_size(100.0)
            .with_vertical_flow()
            .with_variation("wght", 900.0)
            .shape("\u{3001}")
            .unwrap();
        assert_eq!(run.glyphs[0].glyph.layers.len(), 2);
        assert_eq!(
            run.glyphs[0].glyph.metrics.bounds,
            Some(bounds(5.0, -70.0, 80.0, 10.0))
        );
        for layer in &run.glyphs[0].glyph.layers {
            let GlyphLayer::Path(path) = layer else {
                panic!("expected path")
            };
            assert_eq!((path.offset_x, path.offset_y), (0.0, 0.0));
        }
    }
}

#[test]
fn colr_metadata_does_not_translate_unrenderable_color_fallback() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [3, 4] {
        let face = file.face(index).unwrap();
        let run = face
            .engine()
            .with_font_size(100.0)
            .with_vertical_flow()
            .shape("\u{3001}")
            .unwrap();
        assert_eq!(run.glyphs[0].glyph.layers.len(), 1);
        let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
            panic!("expected fallback path")
        };
        assert_eq!(path.paint, GlyphPaint::CurrentColor);
        assert_eq!((path.offset_x, path.offset_y), (0.0, 0.0));
        assert_eq!(
            run.glyphs[0].glyph.metrics.bounds,
            Some(bounds(5.0, -70.0, 45.0, 0.0))
        );
    }
}
