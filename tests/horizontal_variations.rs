use fontcore::{FontFile, FontOptions, FontStretch};
const COLLECTION: &[u8] = include_bytes!("fixtures/horizontal-variations.ttc");
fn check(index: usize, text: &str, base: f32, delta: f32) {
    let file = FontFile::from_buffer(COLLECTION).unwrap();
    let face = file.face(index).unwrap();
    for (weight, scalar) in [(400.0, 0.0), (650.0, 0.5), (900.0, 1.0)] {
        let engine = face
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", weight);
        let run = engine.shape(text).unwrap();
        let advance = base + delta * scalar;
        assert_eq!(
            run.glyphs[0].glyph.metrics.advance_x, advance,
            "face {index}, weight {weight}, {text}"
        );
        assert_eq!(run.glyphs[1].x, advance);
        assert_eq!(engine.measure(text).unwrap(), f64::from(advance * 2.0));
    }
}
#[test]
fn gvar_only() {
    check(0, "AA", 600.0, 200.0);
}
#[test]
fn matching_hvar_gvar() {
    check(1, "AA", 600.0, 200.0);
}
#[test]
fn missing_lsb_mapping() {
    check(2, "AA", 600.0, 200.0);
}
#[test]
fn explicit_advance_mapping() {
    check(3, "AA", 600.0, 200.0);
}
// Conflicting tables test a deterministic precedence policy, not conforming fonts.
#[test]
fn conflicting_hvar_diagnostic() {
    check(4, "AA", 600.0, 300.0);
}
#[test]
fn mapped_zero_diagnostic() {
    check(5, "AA", 600.0, 0.0);
}
#[test]
fn phantom_only() {
    check(6, "AA", 600.0, 200.0);
}
#[test]
fn hvar_without_applicable_tuple() {
    check(7, "AA", 600.0, 200.0);
}
#[test]
fn negative_delta() {
    check(8, "AA", 600.0, -200.0);
}
#[test]
fn composite_matching_hvar_gvar() {
    check(1, "BB", 700.0, 200.0);
}

fn bearing(index: usize, base: f32, delta: f32) {
    let file = FontFile::from_buffer(COLLECTION).unwrap();
    for (weight, scalar) in [(400.0, 0.0), (650.0, 0.5), (900.0, 1.0)] {
        let run = file
            .face(index)
            .unwrap()
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", weight)
            .shape("A")
            .unwrap();
        assert_eq!(
            run.glyphs[0].glyph.metrics.bearing_x,
            base + delta * scalar,
            "face {index}, weight {weight}"
        );
    }
}
#[test]
fn matching_lsb() {
    bearing(1, 50.0, 40.0);
}
#[test]
fn absent_lsb_uses_gvar() {
    bearing(2, 50.0, 40.0);
}
#[test]
fn mapped_zero_lsb_diagnostic() {
    bearing(5, 50.0, 0.0);
}
// Out-of-range diagnostic metrics exercise saturation, not valid font authoring.
#[test]
fn advances_clamp_without_wrap() {
    let file = FontFile::from_buffer(COLLECTION).unwrap();
    for (index, expected) in [(9, 0.0), (10, 65535.0), (11, 0.0), (12, 65535.0)] {
        let run = file
            .face(index)
            .unwrap()
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", 900.0)
            .shape("A")
            .unwrap();
        assert_eq!(
            run.glyphs[0].glyph.metrics.advance_x, expected,
            "face {index}"
        );
    }
}

#[test]
fn signed_fractional_deltas() {
    let file = FontFile::from_buffer(COLLECTION).unwrap();
    for index in [13, 14] {
        let face = file.face(index).unwrap();
        for (weight, advance, bearing) in [
            (400.0, 600.0, 50.0),
            (525.0, 550.0, 40.0),
            (900.0, 399.0, 9.0),
        ] {
            let run = face
                .engine()
                .with_font_size(1000.0)
                .with_variation("wght", weight)
                .shape("AA")
                .unwrap();
            assert_eq!(run.glyphs[0].glyph.metrics.advance_x, advance);
            assert_eq!(run.glyphs[0].glyph.metrics.bearing_x, bearing);
            assert_eq!(run.glyphs[1].x, advance);
        }
    }
}

#[test]
fn scaled_stretched_shape_measure_and_svg_agree() {
    let file = FontFile::from_buffer(COLLECTION).unwrap();
    // Consistent tables with and without optional LSB mapping, on selected faces.
    for index in 0..4 {
        let face = file.face(index).unwrap();
        let options = FontOptions::new(&face)
            .with_font_size(100.0)
            .with_font_stretch(FontStretch(1.5));
        let engine = face
            .engine()
            .with_options(options)
            .with_variation("wght", 900.0);
        let run = engine.shape("AA").unwrap();
        assert!((run.glyphs[0].glyph.metrics.advance_x - 120.0).abs() < 0.001);
        assert!((run.glyphs[0].glyph.metrics.bearing_x - 13.5).abs() < 0.001);
        assert!((run.glyphs[1].x - 120.0).abs() < 0.001);
        assert!((engine.measure("AA").unwrap() - 240.0).abs() < 0.001);
        let measured = engine.measure_layout("AA").unwrap();
        assert!((measured.inline_advance - 240.0).abs() < 0.001);
        let bounds = measured.ink_bounds.unwrap();
        assert!((bounds.min_x - 13.5).abs() < 0.001);
        assert!((bounds.max_x - 193.5).abs() < 0.001);
        assert_eq!(bounds.min_y, -70.0);
        assert_eq!(bounds.max_y, 0.0);
        let svg = engine.render_svg("AA").unwrap();
        let view_box = svg
            .split("viewBox=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let values: Vec<f32> = view_box
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        // SVG adds four user units around the same absolute ink bounds.
        for (actual, expected) in values.iter().zip([
            bounds.min_x - 4.0,
            bounds.min_y - 4.0,
            bounds.max_x - bounds.min_x + 8.0,
            bounds.max_y - bounds.min_y + 8.0,
        ]) {
            assert!((actual - expected).abs() < 0.001, "{svg}");
        }
    }
}

#[test]
fn standalone_first_face_applies_each_delta_once() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/horizontal-variations.ttf")).unwrap();
    let face = file.face(0).unwrap();
    for (weight, scalar) in [(400.0, 0.0), (650.0, 0.5), (900.0, 1.0)] {
        let engine = face
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", weight);
        let run = engine.shape("AA").unwrap();
        assert_eq!(
            run.glyphs[0].glyph.metrics.advance_x,
            600.0 + 200.0 * scalar
        );
        assert_eq!(run.glyphs[0].glyph.metrics.bearing_x, 50.0 + 40.0 * scalar);
        assert_eq!(run.glyphs[1].x, 600.0 + 200.0 * scalar);
        assert_eq!(
            engine.measure("AA").unwrap(),
            f64::from(1200.0 + 400.0 * scalar)
        );
    }
}
