//! MIT synthetic font: unclassified marks with font-authored vert SinglePos.
use fontcore::{FontFaceDescriptor, FontFamily, FontFile, FontOptions, FontStretch};
const FONT: &[u8] = include_bytes!("fixtures/vertical-singlepos.ttf");
const CONTROL: &[u8] = include_bytes!("fixtures/vertical-singlepos-control.ttf");

#[test]
fn vertical_singlepos_positions_unclassified_kana_marks() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.current_face().unwrap();
    for (size, stretch) in [(1000.0, 1.0), (500.0, 2.0), (200.0, 0.5)] {
        let sx = size / 1000.0 * stretch;
        let sy = size / 1000.0;
        let options = FontOptions::new(&face)
            .with_font_size(size)
            .with_font_stretch(FontStretch(stretch))
            .with_vertical_flow();
        let engine = face.engine().with_options(options);
        for text in ["ㇷ\u{3099}ㇷ", "ㇷ\u{309a}ㇷ"] {
            let run = engine.shape(text).unwrap();
            assert_eq!(run.glyphs.len(), 3);
            let mark = &run.glyphs[1];
            let enabled = cfg!(feature = "layout");
            assert_eq!(mark.x, if enabled { 500.0 * sx } else { 0.0 });
            assert_eq!(mark.y, if enabled { 0.0 } else { 1000.0 * sy });
            assert_eq!(mark.glyph.metrics.advance_y, 0.0);
            assert_eq!(run.glyphs[2].y, 1000.0 * sy);
            assert!((engine.measure(text).unwrap() - f64::from(2000.0 * sy)).abs() < 0.001);
            assert!(engine.render_svg(text).unwrap().contains("<svg"));
            let mixed = engine.shape_vertical_mixed_ltr(text).unwrap();
            assert_eq!(format!("{:?}", mixed.glyphs), format!("{:?}", run.glyphs));
        }
    }
}

#[test]
fn horizontal_and_sideways_ignore_vertical_singlepos() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let control = FontFile::from_buffer(CONTROL).unwrap();
    let face = file.current_face().unwrap();
    let control_face = control.current_face().unwrap();
    for text in ["ㇷ\u{3099}ㇷ", "B", "ABC"] {
        let engine = face.engine();
        let baseline = control_face.engine();
        assert_eq!(
            format!("{:?}", engine.shape(text).unwrap()),
            format!("{:?}", baseline.shape(text).unwrap())
        );
        assert_eq!(
            format!("{:?}", engine.shape_sideways_ltr(text).unwrap()),
            format!("{:?}", baseline.shape_sideways_ltr(text).unwrap())
        );
    }
}

#[test]
fn vertical_signed_values_and_public_family_paths() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.current_face().unwrap();
    let mut family = FontFamily::new("Test");
    family.add_face(
        FontFaceDescriptor::new("Test"),
        file.current_face().unwrap(),
    );
    let options = family.options().with_font_size(1000.0).with_vertical_flow();
    let engine = face.engine().with_options(options.clone());
    for text in ["ㇷ\u{3099}ㇷ", "BB"] {
        let run = engine.shape(text).unwrap();
        let expected = if cfg!(feature = "layout") && text == "BB" {
            2100.0
        } else {
            2000.0
        };
        assert_eq!(engine.measure(text).unwrap(), expected);
        assert_eq!(
            engine.measure_layout(text).unwrap().inline_advance,
            expected
        );
        assert_eq!(
            family.measure_with_options(text, options.clone()).unwrap(),
            expected
        );
        assert_eq!(
            format!(
                "{:?}",
                family.text2glyph_run(text, options.clone()).unwrap()
            ),
            format!("{:?}", run)
        );
        assert_eq!(
            format!("{:?}", face.text2glyph_run(text, options.clone()).unwrap()),
            format!("{:?}", run)
        );
        assert_eq!(
            face.measure_with_options(text, options.clone()).unwrap(),
            expected
        );
        assert_eq!(
            family
                .text2svg_with_options(text, "px", options.clone())
                .unwrap(),
            engine.render_svg(text).unwrap()
        );
        let mut ink = run.glyphs[0].glyph.metrics.bounds.unwrap();
        ink.min_x += run.glyphs[0].x;
        ink.max_x += run.glyphs[0].x;
        ink.min_y += run.glyphs[0].y;
        ink.max_y += run.glyphs[0].y;
        for glyph in &run.glyphs[1..] {
            let bounds = glyph.glyph.metrics.bounds.unwrap();
            ink.min_x = ink.min_x.min(glyph.x + bounds.min_x);
            ink.max_x = ink.max_x.max(glyph.x + bounds.max_x);
            ink.min_y = ink.min_y.min(glyph.y + bounds.min_y);
            ink.max_y = ink.max_y.max(glyph.y + bounds.max_y);
        }
        assert_eq!(engine.measure_layout(text).unwrap().ink_bounds, Some(ink));
        if text == "BB" && cfg!(feature = "layout") {
            assert_eq!((run.glyphs[0].x, run.glyphs[0].y), (-20.0, 30.0));
            assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (20.0, 1080.0));
        }
    }
    let text = "ㇷ\u{3099}ㇷ";
    assert_eq!(
        engine.measure_vertical_mixed_ltr(text).unwrap(),
        engine.measure(text).unwrap()
    );
    assert_eq!(
        engine
            .measure_layout_vertical_mixed_ltr(text)
            .unwrap()
            .inline_advance,
        engine.measure(text).unwrap()
    );
    assert!(engine
        .render_svg_vertical_mixed_ltr(text)
        .unwrap()
        .contains("<svg"));
}
