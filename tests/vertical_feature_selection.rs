use fontcore::FontFile;

const FONTS: &[u8] = include_bytes!("fixtures/vertical-feature-selection.ttc");
const BASE: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");

fn check(index: usize, replacement: &str) {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(index).unwrap();
    let engine = face.engine().with_font_size(1000.0).with_vertical_flow();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.current_face().unwrap();
    let expected = base_face
        .engine()
        .with_font_size(1000.0)
        .with_vertical_flow();
    let run = engine.shape("AA").unwrap();
    let reference = expected.shape(replacement).unwrap();
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds, reference.glyphs[0].glyph.metrics.bounds,
        "face {index}"
    );
    assert_eq!(
        run.glyphs[0].glyph.metrics.advance_y, reference.glyphs[0].glyph.metrics.advance_y,
        "face {index}"
    );
    assert_eq!(
        run.glyphs[1].y, reference.glyphs[0].glyph.metrics.advance_y,
        "face {index}"
    );
    assert_eq!(
        engine.measure("A").unwrap(),
        expected.measure(replacement).unwrap(),
        "face {index}"
    );
    assert_eq!(
        engine.render_svg("A").unwrap(),
        expected.render_svg(replacement).unwrap(),
        "face {index}"
    );
}

#[cfg(feature = "layout")]
#[test]
fn vertical_feature_precedence_is_independent_of_record_order() {
    for index in [0, 1, 5, 9, 10] {
        check(index, "\u{3001}");
    }
}

#[cfg(feature = "layout")]
#[test]
fn vertical_feature_fallback_requires_an_available_lookup_set() {
    for index in [2, 6, 7, 8] {
        check(index, "B");
    }
}

#[cfg(feature = "layout")]
#[test]
fn vertical_never_uses_vrtr_or_per_glyph_vert_fallback() {
    for index in [3, 4, 11] {
        check(index, "A");
    }
}

#[cfg(not(feature = "layout"))]
#[test]
fn vertical_features_are_inactive_without_layout() {
    for index in 0..12 {
        check(index, "A");
    }
}

#[test]
fn horizontal_features_remain_inactive() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.current_face().unwrap();
    let expected = base_face.engine().with_font_size(1000.0);
    for index in 0..12 {
        let face = file.face(index).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        assert_eq!(
            engine.render_svg("A").unwrap(),
            expected.render_svg("A").unwrap()
        );
        assert_eq!(engine.measure("A").unwrap(), 600.0);
    }
}
