#![cfg(feature = "layout")]

use fontcore::{FontFile, GlyphFlow};

#[test]
fn vertical_punctuation_uses_format1_substitutions() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-gsub-format1.ttf")).unwrap();
    let face = file.current_face().unwrap();
    let vertical = face.engine().with_font_size(100.0).with_vertical_flow();
    // comma (gid 3) -> B (gid 2), bracket (gid 4) -> comma (gid 3).
    // The covered glyphs have coverage indices 0 and 1, not glyph IDs.
    let run = vertical.shape("\u{3001}\u{300c}A").unwrap();
    assert_eq!(run.glyphs.len(), 3);
    assert_eq!(run.glyphs[0].glyph.metrics.advance_y, 80.0);
    assert_eq!(run.glyphs[1].glyph.metrics.advance_y, 100.0);
    assert_eq!(run.glyphs[2].glyph.metrics.advance_y, 120.0);
    assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (0.0, 80.0));
    assert_eq!((run.glyphs[2].x, run.glyphs[2].y), (0.0, 180.0));
    assert!(run
        .glyphs
        .iter()
        .all(|g| g.glyph.font.unwrap().flow == GlyphFlow::Vertical));
    assert_eq!(vertical.measure("\u{3001}\u{300c}A").unwrap(), 300.0);
    assert!(vertical
        .render_svg("\u{3001}\u{300c}")
        .unwrap()
        .contains("<svg"));
    let horizontal = face.engine().with_font_size(100.0);
    let original = FontFile::from_buffer(include_bytes!("fixtures/vertical-metrics.ttf")).unwrap();
    let original_face = original.current_face().unwrap();
    assert_eq!(
        horizontal.render_svg("\u{3001}\u{300c}").unwrap(),
        original_face
            .engine()
            .with_font_size(100.0)
            .render_svg("\u{3001}\u{300c}")
            .unwrap()
    );
}
