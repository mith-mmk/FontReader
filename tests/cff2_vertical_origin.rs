#![cfg(feature = "cff2")]
use fontcore::{FontFile, FontOptions, FontStretch, GlyphLayer};
const FONT: &[u8] = include_bytes!("fixtures/cff2-vertical-origin.ttc");

fn offset(index: usize, text: &str, weight: f32) -> (f32, f32) {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(index).unwrap();
    let run = face
        .engine()
        .with_font_size(1000.0)
        .with_vertical_flow()
        .with_variation("wght", weight)
        .shape(text)
        .unwrap();
    let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
        panic!()
    };
    (path.offset_x, path.offset_y)
}

#[test]
fn cff2_vorg_and_explicit_signed_origin_deltas() {
    for (weight, scalar) in [(400.0, 0.0), (650.0, 0.5), (900.0, 1.0)] {
        // Origin deltas must not be combined with the independent TSB deltas.
        assert_eq!(offset(0, "A", weight), (-300.0, 900.0 + 80.0 * scalar));
        assert_eq!(offset(0, "B", weight), (-500.0, -100.0 - 60.0 * scalar));
        for index in [2, 3] {
            // Absent and explicitly mapped zero origin deltas do not use the
            // advance-height map (which has nonzero deltas for these glyphs).
            assert_eq!(offset(index, "A", weight), (-300.0, 900.0));
            assert_eq!(offset(index, "B", weight), (-500.0, -100.0));
        }
    }
}

#[test]
fn cff2_no_vorg_uses_varied_curve_extrema_and_metrics() {
    for (weight, scalar) in [(400.0, 0.0), (650.0, 0.5), (900.0, 1.0)] {
        for index in [1, 5] {
            // Face 5 deliberately has an origin map without VORG. That map
            // cannot replace or be added to the yMax + TSB reconstruction.
            let (a, b) = if cfg!(feature = "layout") {
                (800.0 + 140.0 * scalar, 650.0 + 140.0 * scalar)
            } else {
                (850.0 + 50.0 * scalar, 800.0 + 60.0 * scalar)
            };
            assert_eq!(offset(index, "A", weight), (-300.0, a));
            assert_eq!(offset(index, "B", weight), (-500.0, b));
        }
        assert_eq!(offset(4, "A", weight), (-300.0, 850.0 + 50.0 * scalar));
        assert_eq!(offset(4, "B", weight), (-500.0, 800.0 + 60.0 * scalar));
    }
}

#[test]
fn cff2_origin_scales_and_horizontal_is_unchanged() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    for stretch in [0.0, 1.5] {
        let options = FontOptions::new(&face)
            .with_font_size(200.0)
            .with_font_stretch(FontStretch(stretch))
            .with_vertical_flow()
            .with_variation("wght", 900.0);
        let run = face.engine().with_options(options).shape("A ").unwrap();
        let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
            panic!()
        };
        assert_eq!((path.offset_x, path.offset_y), (-60.0 * stretch, 196.0));
        assert_eq!(run.glyphs[1].glyph.metrics.bounds, None);
        let bounds = run.glyphs[0].glyph.metrics.bounds.unwrap();
        assert!(bounds.min_y.is_finite() && bounds.max_y.is_finite());
        if stretch == 0.0 {
            assert_eq!((bounds.min_x, bounds.max_x), (0.0, 0.0));
        }
    }
    for weight in [400.0, 650.0, 900.0] {
        let run = face
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", weight)
            .shape("A")
            .unwrap();
        let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
            panic!()
        };
        assert_eq!((path.offset_x, path.offset_y), (0.0, 0.0));
        assert_eq!(run.glyphs[0].glyph.metrics.advance_x, 600.0);
    }
}

#[test]
fn cff2_woff_shape_measure_and_svg_agree() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/cff2-vertical-origin.woff")).unwrap();
    let face = file.current_face().unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_vertical_flow()
        .with_variation("wght", 900.0);
    let run = engine.shape("A").unwrap();
    let bounds = run.glyphs[0].glyph.metrics.bounds.unwrap();
    assert_eq!((bounds.min_y, bounds.max_y), (180.0, 980.0));
    assert_eq!(engine.measure_layout("A").unwrap().ink_bounds, Some(bounds));
    let svg = engine.render_svg("A").unwrap();
    assert!(svg.contains("viewBox=\"-254 176 408 808\""), "{svg}");
}

#[cfg(feature = "layout")]
#[test]
fn cff2_marks_inherit_varied_base_origin_and_advance() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    for (weight, scalar) in [(400.0, 0.0), (650.0, 0.5), (900.0, 1.0)] {
        let run = face
            .engine()
            .with_font_size(1000.0)
            .with_vertical_flow()
            .with_variation("wght", weight)
            .shape("A\u{301}\u{301}B")
            .unwrap();
        for index in 0..3 {
            let GlyphLayer::Path(path) = &run.glyphs[index].glyph.layers[0] else {
                panic!()
            };
            assert_eq!(
                (path.offset_x, path.offset_y),
                (-300.0, 900.0 + 80.0 * scalar)
            );
        }
        assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (200.0, 0.0));
        assert_eq!((run.glyphs[2].x, run.glyphs[2].y), (200.0, -100.0));
        assert_eq!(
            run.glyphs[0].glyph.metrics.advance_y,
            1200.0 + 200.0 * scalar
        );
        assert_eq!(run.glyphs[3].y, 1200.0 + 200.0 * scalar);
        assert_eq!(
            offset(0, "\u{3001}", weight),
            (-500.0, -100.0 - 60.0 * scalar)
        );
    }
}
