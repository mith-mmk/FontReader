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

fn shifted(mut run: GlyphRun, up: f32) -> GlyphRun {
    for glyph in &mut run.glyphs {
        glyph.y -= up;
    }
    run
}

#[test]
fn rule_adjustments_validate_inputs_and_keep_defaults() {
    let face = FontFile::from_buffer(PAIRS).unwrap().face(0).unwrap();
    let base = rule('A', 'Z', vec![&face]);
    assert_eq!(base.size_multiplier(), 1.0);
    assert_eq!(base.baseline_shift(), 0.0);
    for value in [0.0, -0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            base.clone().with_size_multiplier(value).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            base.clone().with_baseline_shift(value).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }
    for value in [-f32::MAX, -3.0, -0.0, 0.0, 3.0, f32::MAX] {
        assert_eq!(
            base.clone()
                .with_baseline_shift(value)
                .unwrap()
                .baseline_shift(),
            value
        );
    }
    assert_eq!(
        base.clone()
            .with_size_multiplier(f32::MIN_POSITIVE)
            .unwrap()
            .size_multiplier(),
        f32::MIN_POSITIVE
    );
    let adjusted = base
        .with_size_multiplier(1.25)
        .unwrap()
        .with_baseline_shift(-2.0)
        .unwrap();
    assert_eq!(adjusted.size_multiplier(), 1.25);
    assert_eq!(adjusted.baseline_shift(), -2.0);
}

#[test]
fn rule_size_scales_geometry_gpos_and_measure_before_shaping() {
    let face = FontFile::from_buffer(PAIRS).unwrap().face(0).unwrap();
    for scale in [0.5, 1.25, 2.0] {
        let mut composite = CompositeFont::new(vec![&face])
            .unwrap()
            .with_font_size(80.0)
            .with_font_stretch(FontStretch(1.3));
        composite.add_rule(
            rule('A', 'Z', vec![&face])
                .with_size_multiplier(scale)
                .unwrap(),
        );
        let engine = face.engine().with_options(
            fontcore::FontOptions::new(&face)
                .with_font_size(80.0 * scale)
                .with_font_stretch(FontStretch(1.3)),
        );
        let expected = engine.shape("ABA").unwrap();
        same(&composite.shape_ltr("ABA").unwrap(), &expected);
        assert_eq!(
            composite.measure_ltr("ABA").unwrap(),
            expected
                .glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_x)
                .sum::<f32>() as f64
        );
        assert_eq!(
            composite.render_svg_ltr("ABA").unwrap(),
            engine.render_svg("ABA").unwrap()
        );
    }
}

#[test]
fn baseline_is_absolute_positive_up_and_never_changes_following_cursor() {
    let file = FontFile::from_buffer(PAIRS).unwrap();
    let face = file.face(0).unwrap();
    for shift in [-9.0, 7.0] {
        let mut composite = CompositeFont::new(vec![&face])
            .unwrap()
            .with_font_size(100.0);
        composite.add_rule(
            rule('A', 'B', vec![&face])
                .with_size_multiplier(1.5)
                .unwrap()
                .with_baseline_shift(shift)
                .unwrap(),
        );
        let expected = concatenate(vec![
            shifted(
                face.engine().with_font_size(150.0).shape("AB").unwrap(),
                shift,
            ),
            face.engine().with_font_size(100.0).shape(",").unwrap(),
        ]);
        same(&composite.shape_ltr("AB,").unwrap(), &expected);
        let last = composite.shape_ltr("AB,").unwrap().glyphs.pop().unwrap();
        let base = face.engine().with_font_size(150.0).shape("AB").unwrap();
        assert_eq!(
            last.y,
            base.glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_y)
                .sum::<f32>()
        );
        assert_eq!(
            composite.measure_ltr("AB,").unwrap(),
            expected
                .glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_x)
                .sum::<f32>() as f64
        );
        let svg = composite.render_svg_ltr("AB,").unwrap();
        assert!(svg.contains("viewBox="));
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
    }
}

#[test]
fn adjustment_identity_controls_cross_rule_kerning_and_ligatures() {
    let file = FontFile::from_buffer(PAIRS).unwrap();
    let face = file.face(0).unwrap();
    for (b_size, b_shift) in [(1.5, 4.0), (1.0, 4.0), (1.5, -4.0)] {
        let mut composite = CompositeFont::new(vec![&face])
            .unwrap()
            .with_font_size(100.0);
        composite.add_rule(
            rule('A', 'A', vec![&face])
                .with_size_multiplier(1.5)
                .unwrap()
                .with_baseline_shift(4.0)
                .unwrap(),
        );
        composite.add_rule(
            rule('B', 'B', vec![&face])
                .with_size_multiplier(b_size)
                .unwrap()
                .with_baseline_shift(b_shift)
                .unwrap(),
        );
        let expected = if b_size == 1.5 && b_shift == 4.0 {
            shifted(
                face.engine().with_font_size(150.0).shape("AB").unwrap(),
                4.0,
            )
        } else {
            concatenate(vec![
                shifted(face.engine().with_font_size(150.0).shape("A").unwrap(), 4.0),
                shifted(
                    face.engine()
                        .with_font_size(100.0 * b_size)
                        .shape("B")
                        .unwrap(),
                    b_shift,
                ),
            ])
        };
        same(&composite.shape_ltr("AB").unwrap(), &expected);
    }
    let face = FontFile::from_buffer(SIDEWAYS).unwrap().face(3).unwrap();
    for shift in [2.0, 3.0] {
        let mut composite = CompositeFont::new(vec![&face]).unwrap();
        composite.add_rule(
            rule('A', 'A', vec![&face])
                .with_baseline_shift(2.0)
                .unwrap(),
        );
        composite.add_rule(
            rule('B', 'B', vec![&face])
                .with_baseline_shift(shift)
                .unwrap(),
        );
        let expected = if shift == 2.0 {
            shifted(face.shape("AB").unwrap(), 2.0)
        } else {
            concatenate(vec![
                shifted(face.shape("A").unwrap(), 2.0),
                shifted(face.shape("B").unwrap(), shift),
            ])
        };
        same(&composite.shape_ltr("AB").unwrap(), &expected);
    }
}

#[test]
fn adjusted_rule_keeps_entire_egc_and_fallback_policy() {
    let file = FontFile::from_buffer(GRAPHEMES).unwrap();
    let primary = file.face(0).unwrap();
    let full = file.face(1).unwrap();
    let suffix = file.face(2).unwrap();
    for text in [
        "A\u{301}",
        "\u{1f469}\u{1f3fd}\u{200d}\u{1f4bb}",
        "\u{600}\u{627}",
        "\u{915}\u{94d}\u{937}",
        "一\u{e0100}",
    ] {
        let ch = text.chars().next().unwrap();
        for (faces, expected) in [(vec![&full], &full), (vec![&primary, &suffix], &primary)] {
            let mut composite = CompositeFont::new(vec![&full])
                .unwrap()
                .with_font_size(40.0);
            composite.add_rule(
                rule(ch, ch, faces)
                    .with_size_multiplier(1.25)
                    .unwrap()
                    .with_baseline_shift(3.0)
                    .unwrap(),
            );
            composite.add_rule(rule(ch, ch, vec![&full]));
            same(
                &composite.shape_ltr(text).unwrap(),
                &shifted(
                    expected.engine().with_font_size(50.0).shape(text).unwrap(),
                    3.0,
                ),
            );
        }
    }
}

#[test]
fn adjustment_extremes_reject_nonfinite_output_without_partial_results() {
    let face = FontFile::from_buffer(PAIRS).unwrap().face(0).unwrap();
    for (size, multiplier, shift) in [
        (2.0, f32::MAX, 0.0),
        (f32::MIN_POSITIVE, f32::MIN_POSITIVE, 0.0),
        (f32::MAX, 2.0, 0.0),
        (f32::MAX, 1.0, 1.0),
    ] {
        let mut composite = CompositeFont::new(vec![&face])
            .unwrap()
            .with_font_size(size);
        composite.add_rule(
            rule('A', 'B', vec![&face])
                .with_size_multiplier(multiplier)
                .unwrap()
                .with_baseline_shift(shift)
                .unwrap(),
        );
        // Unused rules do not introduce failures for empty text.
        assert!(composite.shape_ltr("").unwrap().glyphs.is_empty());
        assert_eq!(
            composite.shape_ltr("AB").unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            composite.measure_ltr("AB").unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            composite.render_svg_ltr("AB").unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }
    let mut opposite = CompositeFont::new(vec![&face]).unwrap();
    opposite.add_rule(
        rule('A', 'A', vec![&face])
            .with_baseline_shift(f32::MAX)
            .unwrap(),
    );
    opposite.add_rule(
        rule('B', 'B', vec![&face])
            .with_baseline_shift(-f32::MAX)
            .unwrap(),
    );
    assert_eq!(
        opposite.shape_ltr("AB").unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn adjusted_color_and_variable_faces_keep_normal_engine_output() {
    let color = FontFile::from_buffer(SIDEWAYS).unwrap().face(8).unwrap();
    let variable = FontFile::from_buffer(include_bytes!("fixtures/horizontal-variations.ttc"))
        .unwrap()
        .face(0)
        .unwrap();
    for face in [&color, &variable] {
        let mut composite = CompositeFont::new(vec![face])
            .unwrap()
            .with_font_size(50.0)
            .with_variation("wght", 650.0);
        composite.add_rule(
            rule('A', 'A', vec![face])
                .with_size_multiplier(1.5)
                .unwrap()
                .with_baseline_shift(4.0)
                .unwrap(),
        );
        same(
            &composite.shape_ltr("A").unwrap(),
            &shifted(
                face.engine()
                    .with_font_size(75.0)
                    .with_variation("wght", 650.0)
                    .shape("A")
                    .unwrap(),
                4.0,
            ),
        );
    }
}

#[test]
fn adjusted_svg_viewbox_tracks_baseline_without_changing_size() {
    let face = FontFile::from_buffer(PAIRS).unwrap().face(0).unwrap();
    let normal = face.engine().with_font_size(75.0).render_svg("AB").unwrap();
    let mut composite = CompositeFont::new(vec![&face])
        .unwrap()
        .with_font_size(50.0);
    composite.add_rule(
        rule('A', 'B', vec![&face])
            .with_size_multiplier(1.5)
            .unwrap()
            .with_baseline_shift(4.0)
            .unwrap(),
    );
    let shifted = composite.render_svg_ltr("AB").unwrap();
    fn viewbox(svg: &str) -> Vec<f32> {
        svg.split("viewBox=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect()
    }
    let before = viewbox(&normal);
    let after = viewbox(&shifted);
    assert_eq!(
        after,
        vec![before[0], before[1] - 4.0, before[2], before[3]]
    );
    assert_ne!(normal, shifted);
}

#[cfg(feature = "svg-fonts")]
#[test]
fn adjusted_svg_keeps_clips_and_user_space_gradients_with_the_path() {
    use fontcore::{GlyphLayer, GlyphPaint};
    let face = FontFile::from_buffer(include_bytes!("fixtures/composite-adjustments-svg.ttf"))
        .unwrap()
        .face(0)
        .unwrap();
    let mut composite = CompositeFont::new(vec![&face])
        .unwrap()
        .with_font_size(50.0);
    composite.add_rule(
        rule('A', 'A', vec![&face])
            .with_size_multiplier(2.0)
            .unwrap()
            .with_baseline_shift(20.0)
            .unwrap(),
    );
    let run = composite.shape_ltr("BA\u{fe0f}A\u{fe0f}").unwrap();
    let svg = composite.render_svg_ltr("BA\u{fe0f}A\u{fe0f}").unwrap();
    let mut translated_layers = 0;
    for glyph in &run.glyphs[1..] {
        for layer in &glyph.glyph.layers {
            let GlyphLayer::Path(path) = layer else {
                panic!("expected pathified SVG");
            };
            assert!(matches!(
                path.paint,
                GlyphPaint::LinearGradient(_) | GlyphPaint::RadialGradient(_)
            ));
            // Geometry, clip and paint must share a translated local coordinate
            // system, including both the previous span advance and baseline.
            let fontcore::Command::MoveTo(x, y) = path.commands[0] else {
                panic!("fixture path must start with MoveTo");
            };
            assert!(svg.contains(&format!(
                "<g transform=\"translate({} {})\"><path d=\"M{} {} ",
                glyph.x + path.offset_x,
                glyph.y + path.offset_y,
                x,
                y,
            )));
            translated_layers += 1;
        }
    }
    assert_eq!(translated_layers, 4);
    assert_eq!(svg.matches("<g transform=\"translate(").count(), 4);
    assert_eq!(svg.matches("clip-path=\"url(").count(), 2);
    assert!(svg.contains("<linearGradient") && svg.contains("<radialGradient"));
    assert!(svg.contains("gradientUnits=\"userSpaceOnUse\""));
}
