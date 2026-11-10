use fontcore::{FontFile, FontOptions, FontStretch};

const FONT: &[u8] = include_bytes!("fixtures/mark-coordinates.ttf");

#[test]
#[cfg(feature = "layout")]
fn mark_anchors_follow_screen_coordinates_and_scale_once() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.current_face().unwrap();
    for vertical in [false, true] {
        for (size, stretch) in [(100.0, 1.0), (50.0, 2.0), (200.0, 0.5)] {
            let sx = size / 1000.0 * stretch;
            let sy = size / 1000.0;
            let mut options = FontOptions::new(&face)
                .with_font_size(size)
                .with_font_stretch(FontStretch(stretch));
            if vertical {
                options = options.with_vertical_flow();
            }
            let engine = face.engine().with_options(options);
            // A: equal anchor heights; B: mark anchor is 200 units above base.
            // AB ligature: the last component's anchor, not component zero.
            for (text, dx, dy) in [
                ("A\u{301}", 200.0, 0.0),
                ("B\u{301}", 200.0, 200.0),
                ("AB\u{301}", 700.0, 200.0),
            ] {
                let run = engine.shape(text).unwrap();
                assert_eq!(run.glyphs.len(), 2, "{text}");
                let base = &run.glyphs[0];
                let mark = &run.glyphs[1];
                assert!((mark.x - base.x - dx * sx).abs() < 0.001);
                assert!((mark.y - base.y - dy * sy).abs() < 0.001);
                assert_eq!(mark.glyph.metrics.advance_x, 0.0);
                assert_eq!(mark.glyph.metrics.advance_y, 0.0);
                // Rectangular outlines put the mark's bottom on the base top.
                // Compare absolute outline coordinates, not only run origins.
                let bb = base.glyph.metrics.bounds.unwrap();
                let mb = mark.glyph.metrics.bounds.unwrap();
                assert!((mark.y + mb.max_y - base.y - bb.min_y).abs() < 0.001);
                if text.starts_with("AB") {
                    assert!((mark.x + mb.min_x - base.x - bb.max_x).abs() < 0.001);
                }
            }
            let run = engine.shape("A\u{301}\u{301}").unwrap();
            assert_eq!(run.glyphs.len(), 3);
            let first = &run.glyphs[1];
            let second = &run.glyphs[2];
            assert!((second.x - first.x).abs() < 0.001);
            assert!((second.y - first.y + 100.0 * sy).abs() < 0.001);
            let b1 = first.glyph.metrics.bounds.unwrap();
            let b2 = second.glyph.metrics.bounds.unwrap();
            assert!((second.y + b2.max_y - first.y - b1.min_y).abs() < 0.001);
            assert_eq!(second.glyph.metrics.advance_x, 0.0);
            assert_eq!(second.glyph.metrics.advance_y, 0.0);
        }
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn mark_tables_leave_no_layout_behavior_unchanged() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.current_face().unwrap();
    for vertical in [false, true] {
        let mut options = FontOptions::new(&face)
            .with_font_size(50.0)
            .with_font_stretch(FontStretch(2.0));
        if vertical {
            options = options.with_vertical_flow();
        }
        let run = face
            .engine()
            .with_options(options)
            .shape("A\u{301}\u{301}")
            .unwrap();
        assert_eq!(run.glyphs.len(), 3);
        let expected = if vertical { (0.0, 50.0) } else { (60.0, 0.0) };
        for mark in &run.glyphs[1..] {
            assert_eq!((mark.x, mark.y), expected);
            assert_eq!(mark.glyph.metrics.advance_x, 0.0);
            assert_eq!(mark.glyph.metrics.advance_y, 0.0);
        }
    }
}
