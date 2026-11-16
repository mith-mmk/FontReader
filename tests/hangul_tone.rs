use fontcore::{FontFile, GlyphRun};
const FONT: &[u8] = include_bytes!("fixtures/hangul-tone.ttc");
const OLD: &str = "\u{1100}\u{1161}\u{11eb}";
fn run(index: usize, text: &str) -> GlyphRun {
    let f = FontFile::from_buffer(FONT).unwrap();
    f.face(index)
        .unwrap()
        .engine()
        .with_font_size(1000.0)
        .shape(text)
        .unwrap()
}
fn same(a: GlyphRun, b: GlyphRun) {
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}
#[test]
#[cfg(feature = "layout")]
fn old_spacing_tone_precedes_syllable_and_zero_width_stays_after() {
    for tone in ['\u{302e}', '\u{302f}'] {
        same(
            run(1, &format!("{OLD}{tone}")),
            run(0, "\u{e004}\u{e000}\u{e001}\u{e002}"),
        );
        same(
            run(2, &format!("{OLD}{tone}")),
            run(2, "\u{e000}\u{e001}\u{e002}\u{e004}"),
        );
        same(
            run(3, &format!("{OLD}{tone}")),
            run(0, "\u{e003}\u{e000}\u{e001}\u{e002}"),
        );
    }
}
#[test]
fn unrelated_mark_repeated_tone_modern_and_orphan_remain_legacy() {
    for text in ["〮", "가〮", "가〮", "가ᇫ〮〮", "가ᇫ〮A"] {
        same(run(1, text), run(0, text));
    }
}
#[test]
#[cfg(not(feature = "layout"))]
fn tone_without_layout_retains_nominal_order() {
    for tone in ['\u{302e}', '\u{302f}'] {
        same(
            run(1, &format!("{OLD}{tone}")),
            run(0, &format!("{OLD}{tone}")),
        );
    }
}
#[test]
#[cfg(feature = "layout")]
fn vertical_orientation_also_substitutes_tone_and_retains_authored_positioning() {
    use fontcore::ShapingPolicy;
    let f = FontFile::from_buffer(FONT).unwrap();
    let actual = f
        .face(4)
        .unwrap()
        .engine()
        .with_font_size(1000.0)
        .with_shaping_policy(ShapingPolicy::TopToBottom)
        .shape(&format!("{OLD}〮"))
        .unwrap();
    let expected = f
        .face(0)
        .unwrap()
        .engine()
        .with_font_size(1000.0)
        .with_shaping_policy(ShapingPolicy::TopToBottom)
        .shape("\u{e003}\u{e000}\u{e001}\u{e002}")
        .unwrap();
    same(actual, expected);
}
#[test]
#[cfg(feature = "layout")]
fn variable_face_keeps_toned_run_legacy() {
    let mut expected = run(0, &format!("{OLD}〮"));
    // The static control now draws relative to hmtx's metric origin. Variable
    // phantom/HVAR origins are deliberately outside that correction, so retain
    // the known raw-origin geometry of this fixture while comparing every
    // other glyph field, advance and ordering with the legacy fallback control.
    assert_eq!(expected.glyphs.len(), 4);
    for (glyph, xmin) in expected.glyphs.iter_mut().zip([10.0, 20.0, 30.0, 100.0]) {
        for layer in &mut glyph.glyph.layers {
            let fontcore::GlyphLayer::Path(path) = layer else {
                panic!("expected simple path")
            };
            assert_eq!(path.offset_x, -xmin);
            path.offset_x = 0.0;
        }
        let bounds = glyph.glyph.metrics.bounds.as_mut().unwrap();
        assert_eq!(bounds.min_x, 0.0);
        bounds.min_x += xmin;
        bounds.max_x += xmin;
    }
    same(run(5, &format!("{OLD}〮")), expected);
}
#[test]
#[cfg(feature = "layout")]
fn family_toned_cluster_matches_direct_face() {
    use fontcore::{FontFaceDescriptor, FontFamily};
    let f = FontFile::from_buffer(FONT).unwrap();
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), f.face(1).unwrap());
    let text = format!("{OLD}〮{OLD}〯");
    same(
        family
            .text2glyph_run(&text, family.options().with_font_size(1000.0))
            .unwrap(),
        run(1, &text),
    );
}
#[test]
#[cfg(feature = "layout")]
fn family_requires_legacy_coverage_for_toned_cluster() {
    use fontcore::{FontFaceDescriptor, FontFamily};
    let f = FontFile::from_buffer(FONT).unwrap();
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), f.face(6).unwrap());
    family.add_face(FontFaceDescriptor::new("Fallback"), f.face(0).unwrap());
    let text = format!("{OLD}〮");
    same(
        family
            .text2glyph_run(&text, family.options().with_font_size(1000.0))
            .unwrap(),
        run(0, &text),
    );
}
