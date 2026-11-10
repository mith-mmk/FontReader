#![cfg(feature = "cff")]
use fontcore::{FontFile, FontOptions, FontStretch, GlyphLayer};
const FONT: &[u8] = include_bytes!("fixtures/cff-vertical-origin.ttc");
fn offset(face: usize, text: &str) -> (f32, f32) {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(face).unwrap();
    let run = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape(text)
        .unwrap();
    let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
        panic!()
    };
    (path.offset_x, path.offset_y)
}
#[test]
fn cff_vorg_default_signed_and_selected_face() {
    assert_eq!(offset(0, "A"), (-30.0, 90.0));
    assert_eq!(offset(0, "B"), (-50.0, -10.0));
    assert_eq!(
        offset(1, "B"),
        (-50.0, if cfg!(feature = "layout") { 65.0 } else { 80.0 })
    );
    assert_eq!(offset(2, "B"), (-50.0, 80.0));
}
#[test]
fn cff_origins_scale_and_preserve_horizontal_and_empty_glyphs() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let options = FontOptions::new(&face)
        .with_font_size(200.0)
        .with_font_stretch(FontStretch(1.5))
        .with_vertical_flow();
    let run = face.engine().with_options(options).shape("A ").unwrap();
    let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
        panic!()
    };
    assert_eq!((path.offset_x, path.offset_y), (-90.0, 180.0));
    assert_eq!(run.glyphs[1].glyph.metrics.bounds, None);
    let run = face.engine().with_font_size(100.0).shape("A").unwrap();
    let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
        panic!()
    };
    assert_eq!((path.offset_x, path.offset_y), (0.0, 0.0));
    let options = FontOptions::new(&face)
        .with_font_size(100.0)
        .with_font_stretch(FontStretch(0.0))
        .with_vertical_flow();
    let run = face.engine().with_options(options).shape("A").unwrap();
    let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
        panic!()
    };
    assert_eq!((path.offset_x, path.offset_y), (0.0, 90.0));
    let bounds = run.glyphs[0].glyph.metrics.bounds.unwrap();
    assert_eq!((bounds.min_x, bounds.max_x), (0.0, 0.0));
    assert!(bounds.min_y.is_finite() && bounds.max_y.is_finite());
}
#[test]
fn cff_woff_measure_and_svg() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/cff-vertical-origin.woff")).unwrap();
    let face = file.current_face().unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = engine.shape("A").unwrap();
    assert_eq!(run.glyphs[0].glyph.metrics.bounds.unwrap().min_y, 20.0);
    assert_eq!(
        engine.measure_layout("A").unwrap().ink_bounds,
        run.glyphs[0].glyph.metrics.bounds
    );
    assert!(engine
        .render_svg("A")
        .unwrap()
        .contains("viewBox=\"-29 16 48 78\""));
}
#[cfg(feature = "layout")]
#[test]
fn cff_marks_inherit_base_origin() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let run = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("A\u{301}\u{301}")
        .unwrap();
    let GlyphLayer::Path(path) = &run.glyphs[1].glyph.layers[0] else {
        panic!()
    };
    assert_eq!((path.offset_x, path.offset_y), (-30.0, 90.0));
    assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (20.0, 0.0));
    let GlyphLayer::Path(path) = &run.glyphs[2].glyph.layers[0] else {
        panic!()
    };
    assert_eq!((path.offset_x, path.offset_y), (-30.0, 90.0));
    assert_eq!((run.glyphs[2].x, run.glyphs[2].y), (20.0, -10.0));
}

#[test]
fn malformed_optional_vorg_falls_back_and_does_not_reject_truetype() {
    assert_eq!(
        offset(3, "A"),
        (-30.0, if cfg!(feature = "layout") { 90.0 } else { 85.0 })
    );
    assert_eq!(
        offset(4, "A"),
        (-30.0, if cfg!(feature = "layout") { 80.0 } else { 85.0 })
    );
}
#[cfg(feature = "layout")]
#[test]
fn cff_substitution_uses_final_id_and_keeps_vertical_advance() {
    assert_eq!(offset(0, "\u{3001}"), (-50.0, -10.0));
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let run = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("AB")
        .unwrap();
    assert_eq!(run.glyphs[0].glyph.metrics.advance_y, 120.0);
    assert_eq!(run.glyphs[1].glyph.metrics.advance_y, 80.0);
    assert_eq!(run.glyphs[1].y, 120.0);
}
