//! Explicit range selection uses only committed, synthetic MIT fixtures.
use fontcore::{
    CompositeFont, CompositeFontRule, FontFace, FontFile, FontStretch, GlyphRun, UnicodeRange,
};
use std::io::ErrorKind;

const GRAPHEMES: &[u8] = include_bytes!("fixtures/unicode-grapheme-fallback.ttc");
const PAIRS: &[u8] = include_bytes!("fixtures/pair-coordinates.ttc");
const SIDEWAYS: &[u8] = include_bytes!("fixtures/sideways-ltr.ttc");

fn rule<'a>(start: char, end: char, faces: Vec<&'a FontFace>) -> CompositeFontRule<'a> {
    CompositeFontRule::new(vec![UnicodeRange::new(start, end).unwrap()], faces).unwrap()
}

fn same(actual: &GlyphRun, expected: &GlyphRun) {
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
}

fn concatenate(runs: Vec<GlyphRun>) -> GlyphRun {
    let mut output = GlyphRun::default();
    let mut cursor = (0.0, 0.0);
    for mut run in runs {
        let mut delta = (0.0, 0.0);
        for g in &mut run.glyphs {
            g.x += cursor.0;
            g.y += cursor.1;
            delta.0 += g.glyph.metrics.advance_x;
            delta.1 += g.glyph.metrics.advance_y;
        }
        cursor.0 += delta.0;
        cursor.1 += delta.1;
        output.glyphs.extend(run.glyphs);
    }
    output
}

#[test]
fn explicit_selection_wins_even_when_default_covers_everything() {
    let file = FontFile::from_buffer(GRAPHEMES).unwrap();
    let full = file.face(1).unwrap();
    let primary = file.face(0).unwrap();
    let mut composite = CompositeFont::new(vec![&full]).unwrap();
    composite.add_rule(rule('A', 'A', vec![&primary]));
    same(
        &composite.shape_ltr("A一B").unwrap(),
        &concatenate(vec![
            primary.shape("A").unwrap(),
            full.shape("一B").unwrap(),
        ]),
    );
    assert_eq!(composite.rules().len(), 1);
    assert!(std::ptr::eq(composite.default_faces()[0], &full));
}

#[test]
fn inclusive_ranges_overlap_priority_and_no_escape() {
    let file = FontFile::from_buffer(GRAPHEMES).unwrap();
    let full = file.face(1).unwrap();
    let primary = file.face(0).unwrap();
    let mut composite = CompositeFont::new(vec![&full]).unwrap();
    composite.add_rule(rule('A', 'B', vec![&primary]));
    composite.add_rule(rule('A', 'Z', vec![&full]));
    // B is missing in the owning rule. Do not escape to another rule/default.
    same(
        &composite.shape_ltr("AB").unwrap(),
        &primary.shape("AB").unwrap(),
    );
    for ch in ['@', 'C', '\u{e000}', '\u{100000}', '\u{10ffff}'] {
        same(
            &composite.shape_ltr(&ch.to_string()).unwrap(),
            &full.shape(&ch.to_string()).unwrap(),
        );
    }
    let r = UnicodeRange::new('\u{e000}', '\u{100000}').unwrap();
    assert_eq!((r.start(), r.end()), ('\u{e000}', '\u{100000}'));
    for ch in ['\u{e000}', '\u{1f469}', '\u{100000}'] {
        assert!(r.contains(ch));
    }
    assert!(!r.contains('\u{100001}'));
    let extra = CompositeFontRule::new(
        vec![r, UnicodeRange::new('A', 'B').unwrap()],
        vec![&primary],
    )
    .unwrap();
    assert_eq!(extra.ranges().len(), 2);
    assert!(std::ptr::eq(extra.faces()[0], &primary));
}

#[test]
fn whole_clusters_use_rule_local_fallback_and_first_scalar() {
    let file = FontFile::from_buffer(GRAPHEMES).unwrap();
    let primary = file.face(0).unwrap();
    let full = file.face(1).unwrap();
    let suffix = file.face(2).unwrap();
    for text in [
        "A\u{301}",
        "\u{1f469}\u{1f3fd}\u{200d}\u{1f4bb}",
        "\u{600}\u{627}",
        "\u{1100}\u{1161}\u{11a8}",
        "\u{915}\u{94d}\u{937}",
        "\u{1f1ef}\u{1f1f5}",
    ] {
        let first = text.chars().next().unwrap();
        let mut composite = CompositeFont::new(vec![&suffix]).unwrap();
        composite.add_rule(rule(first, first, vec![&primary, &full]));
        same(
            &composite.shape_ltr(text).unwrap(),
            &full.shape(text).unwrap(),
        );
        let mut missing = CompositeFont::new(vec![&full]).unwrap();
        missing.add_rule(rule(first, first, vec![&primary, &suffix]));
        same(
            &missing.shape_ltr(text).unwrap(),
            &primary.shape(text).unwrap(),
        );
    }
}

#[test]
fn variation_sequences_keep_the_face_order_and_selected_uvs() {
    let file = FontFile::from_buffer(GRAPHEMES).unwrap();
    let primary = file.face(0).unwrap();
    let full = file.face(1).unwrap();
    for text in ["A\u{fe00}", "一\u{e0100}"] {
        let first = text.chars().next().unwrap();
        let mut ordered = CompositeFont::new(vec![&full]).unwrap();
        ordered.add_rule(rule(first, first, vec![&primary, &full]));
        // Existing support accepts the base glyph without a UVS mapping.
        same(
            &ordered.shape_ltr(text).unwrap(),
            &primary.shape(text).unwrap(),
        );
        let mut explicit = CompositeFont::new(vec![&primary]).unwrap();
        explicit.add_rule(rule(first, first, vec![&full]));
        same(
            &explicit.shape_ltr(text).unwrap(),
            &full.shape(text).unwrap(),
        );
    }
}

#[test]
fn default_list_fallback_and_orphan_selector_policy() {
    let file = FontFile::from_buffer(GRAPHEMES).unwrap();
    let primary = file.face(0).unwrap();
    let full = file.face(1).unwrap();
    let composite = CompositeFont::new(vec![&primary, &full]).unwrap();
    same(
        &composite.shape_ltr("A\u{301}").unwrap(),
        &full.shape("A\u{301}").unwrap(),
    );
    let mut explicit = CompositeFont::new(vec![&primary]).unwrap();
    explicit.add_rule(rule('\u{301}', '\u{301}', vec![&full]));
    same(
        &explicit.shape_ltr("\u{fe00}\u{301}").unwrap(),
        &full.shape("\u{301}").unwrap(),
    );
    assert!(explicit
        .shape_ltr("\u{fe00}\u{e0100}")
        .unwrap()
        .glyphs
        .is_empty());
}

#[test]
fn same_face_spans_preserve_pairs_and_cross_face_spans_carry_both_advances() {
    let file = FontFile::from_buffer(PAIRS).unwrap();
    let face = file.face(0).unwrap();
    let other = file.face(1).unwrap();
    let mut same_face = CompositeFont::new(vec![&face])
        .unwrap()
        .with_font_size(100.0);
    same_face.add_rule(rule('A', 'A', vec![&face]));
    same(
        &same_face.shape_ltr("ABA").unwrap(),
        &face.engine().with_font_size(100.0).shape("ABA").unwrap(),
    );
    let mut switched = CompositeFont::new(vec![&face])
        .unwrap()
        .with_font_size(100.0);
    switched.add_rule(rule('B', 'B', vec![&other]));
    same(
        &switched.shape_ltr("AAB").unwrap(),
        &concatenate(vec![
            face.engine().with_font_size(100.0).shape("AA").unwrap(),
            other.engine().with_font_size(100.0).shape("B").unwrap(),
        ]),
    );
    // AB's nonzero y advance must carry into another face, not just its x advance.
    let mut cross_axis = CompositeFont::new(vec![&face])
        .unwrap()
        .with_font_size(100.0);
    cross_axis.add_rule(rule(',', ',', vec![&other]));
    same(
        &cross_axis.shape_ltr("AB,").unwrap(),
        &concatenate(vec![
            face.engine().with_font_size(100.0).shape("AB").unwrap(),
            other.engine().with_font_size(100.0).shape(",").unwrap(),
        ]),
    );
}

#[test]
fn ligatures_cross_rule_boundaries_but_not_face_boundaries() {
    let file = FontFile::from_buffer(SIDEWAYS).unwrap();
    let face = file.face(3).unwrap();
    let other = face.clone();
    let mut composite = CompositeFont::new(vec![&face]).unwrap();
    composite.add_rule(rule('A', 'A', vec![&face]));
    same(
        &composite.shape_ltr("AB").unwrap(),
        &face.shape("AB").unwrap(),
    );
    if cfg!(feature = "layout") {
        assert_eq!(composite.shape_ltr("AB").unwrap().glyphs.len(), 1);
    }
    composite.add_rule(rule('B', 'B', vec![&other]));
    same(
        &composite.shape_ltr("AB").unwrap(),
        &concatenate(vec![face.shape("A").unwrap(), other.shape("B").unwrap()]),
    );
}

#[test]
fn validation_empty_measure_and_svg_consistency() {
    let file = FontFile::from_buffer(GRAPHEMES).unwrap();
    let face = file.face(1).unwrap();
    assert!(matches!(CompositeFont::new(vec![]), Err(e) if e.kind()==ErrorKind::InvalidInput));
    assert_eq!(
        UnicodeRange::new('z', 'a').unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        CompositeFontRule::new(vec![], vec![&face])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        CompositeFontRule::new(vec![UnicodeRange::new('a', 'z').unwrap()], vec![])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
    let composite = CompositeFont::new(vec![&face])
        .unwrap()
        .with_font_size(37.0)
        .with_font_stretch(FontStretch(1.3));
    assert!(composite.shape_ltr("").unwrap().glyphs.is_empty());
    assert_eq!(composite.measure_ltr("").unwrap(), 0.0);
    let run = composite.shape_ltr("AB一").unwrap();
    assert_eq!(
        composite.measure_ltr("AB一").unwrap(),
        run.glyphs
            .iter()
            .map(|g| g.glyph.metrics.advance_x)
            .sum::<f32>() as f64
    );
    assert_eq!(
        composite.render_svg_ltr("AB一").unwrap(),
        face.engine()
            .with_options(
                fontcore::FontOptions::new(&face)
                    .with_font_size(37.0)
                    .with_font_stretch(FontStretch(1.3))
            )
            .render_svg("AB一")
            .unwrap()
    );
    for c in [
        '\t', '\r', '\n', '\u{b}', '\u{c}', '\u{85}', '\u{2028}', '\u{2029}',
    ] {
        let text = format!("A{c}B");
        assert_eq!(
            composite.shape_ltr(&text).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            composite.measure_ltr(&text).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            composite.render_svg_ltr(&text).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }
    for size in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        assert!(composite
            .clone()
            .with_font_size(size)
            .shape_ltr("")
            .is_err());
    }
    for stretch in [-1.0, f32::INFINITY, f32::NAN] {
        assert!(composite
            .clone()
            .with_font_stretch(FontStretch(stretch))
            .shape_ltr("")
            .is_err());
    }
}

#[test]
fn explicit_order_overrides_color_preference_and_previous_face_continuity() {
    let file = FontFile::from_buffer(SIDEWAYS).unwrap();
    let outline = file.face(5).unwrap();
    let color = file.face(8).unwrap();
    let composite = CompositeFont::new(vec![&outline, &color]).unwrap();
    same(
        &composite.shape_ltr("A").unwrap(),
        &outline.shape("A").unwrap(),
    );
    let mut explicit = CompositeFont::new(vec![&color]).unwrap();
    explicit.add_rule(rule('A', 'A', vec![&outline]));
    same(
        &explicit.shape_ltr("BA\u{301}").unwrap(),
        &concatenate(vec![
            color.shape("B").unwrap(),
            outline.shape("A\u{301}").unwrap(),
        ]),
    );
}

#[test]
fn builders_forward_variant_locale_and_variations_to_selected_faces() {
    let file = FontFile::from_buffer(SIDEWAYS).unwrap();
    let default = file.face(5).unwrap();
    for index in [2, 4] {
        let selected = file.face(index).unwrap();
        let mut composite = CompositeFont::new(vec![&default])
            .unwrap()
            .with_locale("ja-JP")
            .with_font_variant(fontcore::FontVariant::SmallCaps);
        composite.add_rule(rule('A', 'Z', vec![&selected]));
        same(
            &composite.shape_ltr("AB").unwrap(),
            &selected
                .engine()
                .with_locale("ja-JP")
                .with_font_variant(fontcore::FontVariant::SmallCaps)
                .shape("AB")
                .unwrap(),
        );
    }
    let variable = FontFile::from_buffer(include_bytes!("fixtures/horizontal-variations.ttc"))
        .unwrap()
        .face(0)
        .unwrap();
    let mut composite = CompositeFont::new(vec![&default])
        .unwrap()
        .with_font_size(100.0)
        .with_variation("wght", 650.0);
    composite.add_rule(rule('A', 'A', vec![&variable]));
    same(
        &composite.shape_ltr("AB").unwrap(),
        &concatenate(vec![
            variable
                .engine()
                .with_font_size(100.0)
                .with_variation("wght", 650.0)
                .shape("A")
                .unwrap(),
            default
                .engine()
                .with_font_size(100.0)
                .with_variation("wght", 650.0)
                .shape("B")
                .unwrap(),
        ]),
    );
    let variations = [fontcore::FontVariationSetting::new("wght", 900.0).unwrap()];
    let composite = composite.with_variations(&variations);
    same(
        &composite.shape_ltr("A").unwrap(),
        &variable
            .engine()
            .with_font_size(100.0)
            .with_variations(&variations)
            .shape("A")
            .unwrap(),
    );
}

#[test]
fn mixed_face_svg_uses_the_composed_run_and_requested_unit() {
    let file = FontFile::from_buffer(GRAPHEMES).unwrap();
    let primary = file.face(0).unwrap();
    let full = file.face(1).unwrap();
    let mut composite = CompositeFont::new(vec![&full])
        .unwrap()
        .with_font_size(24.0)
        .with_svg_unit("pt");
    composite.add_rule(rule('A', 'A', vec![&primary]));
    let mut family = fontcore::FontFamily::new("Primary");
    family.add_face(
        fontcore::FontFaceDescriptor::new("Primary"),
        primary.clone(),
    );
    family.add_face(fontcore::FontFaceDescriptor::new("Fallback"), full.clone());
    let options = family.options().with_font_size(24.0);
    // This fixture's coverage produces the same two spans through fallback.
    same(
        &composite.shape_ltr("AB").unwrap(),
        &family.text2glyph_run("AB", options.clone()).unwrap(),
    );
    assert_eq!(
        composite.render_svg_ltr("AB").unwrap(),
        family.text2svg_with_options("AB", "pt", options).unwrap()
    );
}
