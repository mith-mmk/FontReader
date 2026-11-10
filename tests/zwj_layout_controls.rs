use fontcore::{FontFaceDescriptor, FontFamily, FontFile, GlyphBounds, GlyphRun, TextDirection};

const FONT: &[u8] = include_bytes!("fixtures/vertical-no-metrics.ttf");

fn positions(run: &GlyphRun) -> Vec<(f32, f32)> {
    run.glyphs.iter().map(|g| (g.x, g.y)).collect()
}

#[test]
fn zwj_controls_keep_horizontal_line_and_tab_positions() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.current_face().unwrap();
    let engine = face.engine().with_font_size(100.0).with_line_height(150.0);
    // The fixture has no ZWJ mapping: its existing empty .notdef advances 100.
    // Preserve that behavior while ensuring controls never become glyphs.
    for (control, last) in [
        ("\n", (0.0, 150.0)),
        ("\r\n", (0.0, 150.0)),
        ("\t", (760.0, 0.0)),
        ("\r", (160.0, 0.0)),
    ] {
        let text = format!("A\u{200d}{control}B");
        let run = engine.shape(&text).unwrap();
        assert_eq!(positions(&run), [(0.0, 0.0), (60.0, 0.0), last], "{text:?}");
        // Existing engine measurement sums glyph advances, not tabs/line maxima.
        assert_eq!(engine.measure(&text).unwrap(), 260.0);
        let layout = engine.measure_layout(&text).unwrap();
        assert_eq!(layout.block_extent, last.1 as f64);
        assert_eq!(
            layout.line_bounds,
            Some(GlyphBounds {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 160.0_f32.max(last.0 + 100.0),
                max_y: last.1
            })
        );
        assert_svg_bounds(
            &engine.render_svg(&text).unwrap(),
            layout.ink_bounds.unwrap(),
        );
    }
}

#[test]
fn zwj_controls_keep_vertical_column_and_tab_positions() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.current_face().unwrap();
    let engine = face
        .engine()
        .with_font_size(100.0)
        .with_line_height(150.0)
        .with_vertical_flow();
    for (control, last) in [
        ("\n", (-150.0, 0.0)),
        ("\r\n", (-150.0, 0.0)),
        ("\t", (0.0, 800.0)),
        ("\r", (0.0, 200.0)),
    ] {
        let text = format!("A\u{200d}{control}B");
        let run = engine.shape(&text).unwrap();
        assert_eq!(
            positions(&run),
            [(0.0, 0.0), (0.0, 100.0), last],
            "{text:?}"
        );
        assert_eq!(engine.measure(&text).unwrap(), 300.0);
        let layout = engine.measure_layout(&text).unwrap();
        assert_eq!(layout.block_extent, -last.0 as f64);
        assert_eq!(
            layout.line_bounds,
            Some(GlyphBounds {
                min_x: last.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 200.0_f32.max(last.1 + 100.0)
            })
        );
        assert_svg_bounds(
            &engine.render_svg(&text).unwrap(),
            layout.ink_bounds.unwrap(),
        );
    }
}

fn assert_svg_bounds(svg: &str, ink: GlyphBounds) {
    let view_box = format!(
        "viewBox=\"{} {} {} {}\"",
        ink.min_x - 4.0,
        ink.min_y - 4.0,
        ink.max_x - ink.min_x + 8.0,
        ink.max_y - ink.min_y + 8.0
    );
    assert!(svg.contains(&view_box), "expected {view_box}: {svg}");
}

#[test]
fn repeated_joiners_and_controls_do_not_drop_following_text() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.current_face().unwrap();
    for vertical in [false, true] {
        let mut engine = face.engine().with_font_size(100.0).with_line_height(150.0);
        if vertical {
            engine = engine.with_vertical_flow();
        }
        let text = "\u{200d}\nA\u{200d}\u{200d}\u{200d}\n\nB\u{200d}\t\tA\u{200d}\r\nB\u{200d}";
        let expected = if vertical {
            vec![
                (0.0, 0.0),
                (-150.0, 0.0),
                (-150.0, 100.0),
                (-150.0, 200.0),
                (-150.0, 300.0),
                (-450.0, 0.0),
                (-450.0, 100.0),
                (-450.0, 1400.0),
                (-450.0, 1500.0),
                (-600.0, 0.0),
                (-600.0, 100.0),
            ]
        } else {
            vec![
                (0.0, 0.0),
                (0.0, 150.0),
                (60.0, 150.0),
                (160.0, 150.0),
                (260.0, 150.0),
                (0.0, 450.0),
                (100.0, 450.0),
                (1400.0, 450.0),
                (1460.0, 450.0),
                (0.0, 600.0),
                (100.0, 600.0),
            ]
        };
        assert_eq!(positions(&engine.shape(text).unwrap()), expected);
        assert_eq!(engine.shape("\u{200d}").unwrap().glyphs.len(), 1);
        assert_eq!(engine.shape("A\u{200d}").unwrap().glyphs.len(), 2);
    }
}

#[test]
fn controls_after_zwj_preserve_family_fallback_and_measurement() {
    let primary = FontFile::from_buffer(FONT).unwrap().current_face().unwrap();
    // Face 6 maps f to A, has vmtx, and has no vertical substitution for f.
    // The primary has no f, so the suffix must really use the fallback face.
    let fallback = FontFile::from_buffer(include_bytes!("fixtures/sideways-ltr.ttc"))
        .unwrap()
        .face(6)
        .unwrap();
    let mut family = FontFamily::new("ZWJ controls");
    family.add_face(FontFaceDescriptor::new("ZWJ controls"), primary);
    family.add_face(FontFaceDescriptor::new("Fallback"), fallback.clone());
    for vertical in [false, true] {
        let direction = if vertical {
            TextDirection::TopToBottom
        } else {
            TextDirection::LeftToRight
        };
        let options = family
            .options()
            .with_font_size(100.0)
            .with_line_height(150.0)
            .with_text_direction(direction);
        let fallback_run = fallback
            .engine()
            .with_options(options.clone())
            .shape("f")
            .unwrap();
        let last_advance = if vertical {
            if cfg!(feature = "layout") {
                120.0
            } else {
                100.0
            }
        } else {
            60.0
        };
        for control in ["\n", "\t", "\r", "\r\n"] {
            let text = format!("A\u{200d}{control}f");
            let run = family.text2glyph_run(&text, options.clone()).unwrap();
            let prefix_advance = if vertical { 200.0 } else { 160.0 };
            let last = if control.contains('\n') {
                if vertical {
                    (-150.0, 0.0)
                } else {
                    (0.0, 150.0)
                }
            } else {
                let inline = prefix_advance + if control == "\t" { 600.0 } else { 0.0 };
                if vertical {
                    (0.0, inline)
                } else {
                    (inline, 0.0)
                }
            };
            let second = if vertical { (0.0, 100.0) } else { (60.0, 0.0) };
            assert_eq!(positions(&run), [(0.0, 0.0), second, last], "{text:?}");
            assert_eq!(
                format!("{:?}", run.glyphs[2].glyph.metrics),
                format!("{:?}", fallback_run.glyphs[0].glyph.metrics)
            );
            assert_eq!(
                format!("{:?}", run.glyphs[2].glyph.layers),
                format!("{:?}", fallback_run.glyphs[0].glyph.layers)
            );
            // Family measurement retains its existing maximum-line extent contract.
            let expected = if control.contains('\n') {
                prefix_advance.max(last_advance)
            } else {
                (if vertical { last.1 } else { last.0 }) + last_advance
            };
            assert_eq!(
                family.measure_with_options(&text, options.clone()).unwrap(),
                expected as f64
            );
            assert!(family
                .text2svg_with_options(&text, "px", options.clone())
                .unwrap()
                .contains("<svg"));
        }
    }
}
