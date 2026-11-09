use fontcore::{FontFaceDescriptor, FontFamily, FontFile, GlyphRun, TextDirection};

const FONTS: &[u8] = include_bytes!("fixtures/unicode-grapheme-fallback.ttc");
const CLUSTERS: &[(&str, &str)] = &[
    ("Hangul Jamo", "\u{1100}\u{1161}\u{11a8}"),
    ("SpacingMark", "\u{0915}\u{093e}"),
    ("Prepend", "\u{0600}\u{0627}"),
    ("Indic conjunct", "\u{0915}\u{094d}\u{0937}"),
    ("emoji ZWJ", "\u{1f469}\u{1f3fd}\u{200d}\u{1f4bb}"),
    ("regional indicator pair", "\u{1f1ef}\u{1f1f5}"),
    ("combining mark", "A\u{0301}"),
];

fn family(fallback_index: usize) -> FontFamily {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let mut family = FontFamily::new("Primary");
    // Explicit family descriptors make candidate order independent of font names.
    family.add_face(FontFaceDescriptor::new("Primary"), file.face(0).unwrap());
    family.add_face(
        FontFaceDescriptor::new("Fallback"),
        file.face(fallback_index).unwrap(),
    );
    family
}

fn assert_run(actual: &GlyphRun, expected: &GlyphRun, label: &str) {
    assert_eq!(actual.glyphs.len(), expected.glyphs.len(), "{label}");
    for (a, b) in actual.glyphs.iter().zip(&expected.glyphs) {
        assert_eq!((a.x, a.y), (b.x, b.y), "{label}");
        assert_eq!(
            a.glyph.metrics.advance_x, b.glyph.metrics.advance_x,
            "{label}"
        );
        assert_eq!(
            a.glyph.metrics.advance_y, b.glyph.metrics.advance_y,
            "{label}"
        );
        assert_eq!(a.glyph.metrics.bounds, b.glyph.metrics.bounds, "{label}");
        assert_eq!(
            format!("{:?}", a.glyph.layers),
            format!("{:?}", b.glyph.layers),
            "{label}"
        );
    }
}

fn assert_whole_cluster_fallback(index: usize) {
    let (label, text) = CLUSTERS[index];
    let file = FontFile::from_buffer(FONTS).unwrap();
    let fallback = file.face(1).unwrap();
    let family = family(1);
    for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
        let options = family
            .options()
            .with_font_size(100.0)
            .with_text_direction(direction);
        let expected = fallback
            .engine()
            .with_options(options.clone())
            .shape(text)
            .unwrap();
        // This is scalar glyph resolution, not Indic or emoji shaping.
        assert_eq!(expected.glyphs.len(), text.chars().count(), "{label}");
        let actual = family.text2glyph_run(text, options.clone()).unwrap();
        assert_run(&actual, &expected, label);
    }
}

macro_rules! fallback_case {
    ($name:ident, $index:expr) => {
        #[test]
        fn $name() {
            assert_whole_cluster_fallback($index);
        }
    };
}
fallback_case!(fallback_keeps_hangul_jamo_together, 0);
fallback_case!(fallback_keeps_spacing_mark_together, 1);
fallback_case!(fallback_keeps_prepend_together, 2);
fallback_case!(fallback_keeps_indic_conjunct_together, 3);
fallback_case!(fallback_keeps_emoji_zwj_together, 4);
fallback_case!(fallback_keeps_regional_indicators_together, 5);
fallback_case!(fallback_keeps_combining_mark_together, 6);

#[test]
fn incomplete_faces_do_not_split_a_cluster_to_fill_missing_glyphs() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let primary = file.face(0).unwrap();
    let family = family(2);
    let options = family.options().with_font_size(100.0);
    for &(label, text) in CLUSTERS {
        // Neither face covers the whole cluster. Existing policy uses the first
        // candidate for the entire cluster, retaining its .notdef glyphs.
        let expected = primary
            .engine()
            .with_options(options.clone())
            .shape(text)
            .unwrap();
        let missing = primary
            .engine()
            .with_font_size(100.0)
            .shape("\u{ffff}")
            .unwrap();
        assert!(
            expected
                .glyphs
                .iter()
                .any(|g| format!("{:?}", g.glyph.layers)
                    == format!("{:?}", missing.glyphs[0].glyph.layers)),
            "{label}"
        );
        assert_run(
            &family.text2glyph_run(text, options.clone()).unwrap(),
            &expected,
            label,
        );
    }
}

#[test]
fn engine_keeps_scalar_resolution_uvs_and_missing_zwj_behavior() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let full = file.face(1).unwrap();
    let engine = full.engine().with_font_size(100.0);
    for &(label, text) in CLUSTERS {
        let run = engine.shape(text).unwrap();
        assert_eq!(run.glyphs.len(), text.chars().count(), "{label}");
        assert!(
            run.glyphs.iter().all(|g| g.glyph.metrics.advance_x == 80.0),
            "{label}"
        );
    }
    for text in ["A\u{fe00}", "\u{4e00}\u{e0100}"] {
        let run = engine.shape(text).unwrap();
        assert_eq!(run.glyphs.len(), 1);
        assert_eq!(run.glyphs[0].glyph.metrics.advance_x, 90.0);
    }
    assert!(engine.shape("\u{fe00}\u{e0100}").unwrap().glyphs.is_empty());
    let primary = file.face(0).unwrap();
    let run = primary
        .engine()
        .with_font_size(100.0)
        .shape("A\u{200d}")
        .unwrap();
    assert_eq!(run.glyphs.len(), 2);
    assert_eq!(run.glyphs[1].glyph.metrics.advance_x, 40.0);
    let missing = primary
        .engine()
        .with_font_size(100.0)
        .shape("\u{ffff}")
        .unwrap();
    assert_eq!(
        format!("{:?}", run.glyphs[1].glyph.layers),
        format!("{:?}", missing.glyphs[0].glyph.layers)
    );
}

#[test]
fn ordinary_latin_cjk_and_layout_controls_keep_their_boundaries() {
    let family = family(1);
    let options = family
        .options()
        .with_font_size(100.0)
        .with_line_height(150.0);
    let run = family
        .text2glyph_run("A\u{4e00}B\r\nA\tB\rA", options)
        .unwrap();
    assert_eq!(run.glyphs.len(), 6);
    let positions: Vec<_> = run.glyphs.iter().map(|g| (g.x, g.y)).collect();
    assert_eq!(
        positions,
        [
            (0.0, 0.0),
            (40.0, 0.0),
            (80.0, 0.0),
            (0.0, 150.0),
            (640.0, 150.0),
            (720.0, 150.0)
        ]
    );
    let advances: Vec<_> = run
        .glyphs
        .iter()
        .map(|g| g.glyph.metrics.advance_x)
        .collect();
    assert_eq!(advances, [40.0, 40.0, 80.0, 40.0, 80.0, 40.0]);
}

#[test]
fn family_fallback_preserves_nondefault_variation_mappings() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let full = file.face(1).unwrap();
    let mut family = FontFamily::new("Suffix only");
    family.add_face(
        FontFaceDescriptor::new("Suffix only"),
        file.face(2).unwrap(),
    );
    family.add_face(FontFaceDescriptor::new("Full fallback"), full.clone());
    let options = family.options().with_font_size(100.0);
    for text in ["A\u{fe00}", "\u{4e00}\u{e0100}"] {
        let expected = full
            .engine()
            .with_options(options.clone())
            .shape(text)
            .unwrap();
        let actual = family.text2glyph_run(text, options.clone()).unwrap();
        assert_eq!(actual.glyphs.len(), 1);
        assert_eq!(actual.glyphs[0].glyph.metrics.advance_x, 90.0);
        assert_run(&actual, &expected, text);
    }
}

#[test]
fn leading_orphan_selectors_do_not_drop_a_following_mark() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let full = file.face(1).unwrap();
    let engine = full.engine().with_font_size(100.0);
    let expected = engine.shape("\u{0301}").unwrap();
    assert_eq!(expected.glyphs.len(), 1);
    for text in [
        "\u{fe00}\u{0301}",
        "\u{e0100}\u{0301}",
        "\u{fe00}\u{e0100}\u{0301}",
    ] {
        assert_run(&engine.shape(text).unwrap(), &expected, text);
    }
}
