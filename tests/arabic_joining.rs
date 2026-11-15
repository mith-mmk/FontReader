//! Synthetic source-character joining through the public shaping pipeline.
#![cfg(feature = "layout")]
use fontcore::FontFile;

#[test]
fn joining_survives_ccmp_expansion_and_contraction() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/arabic-joining.ttf")).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.)
        .with_locale("ar")
        .with_right_to_left();
    for (text, widths) in [
        ("بب", vec![200., 0., 300., 0.]),
        ("بَب", vec![200., 0., 0., 300., 0.]),
        ("تَب", vec![250., 300., 0.]),
        ("بـب", vec![200., 0., 100., 300., 0.]),
        ("ب\u{200c}ب", vec![600., 0., 0., 600., 0.]),
        ("ب\u{200d}ب", vec![200., 0., 0., 300., 0.]),
    ] {
        let run = engine.shape(text).unwrap();
        assert_eq!(
            run.glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_x)
                .collect::<Vec<_>>(),
            widths,
            "{text}"
        );
    }
    // LTR remains outside the joining stage.
    let run = face
        .engine()
        .with_font_size(1000.)
        .with_locale("ar")
        .shape("بب")
        .unwrap();
    assert_eq!(
        run.glyphs
            .iter()
            .map(|g| g.glyph.metrics.advance_x)
            .collect::<Vec<_>>(),
        [600., 0., 600., 0.]
    );
}
