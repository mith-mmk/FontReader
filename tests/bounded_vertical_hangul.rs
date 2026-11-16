//! Original MIT fixtures; authored GPOS, not inferred zero vertical advance.
use fontcore::{FontFile, GlyphRun, ShapingPolicy};
const FONT: &[u8] = include_bytes!("fixtures/bounded-vertical-hangul.ttc");
const OLD: &str = "\u{1100}\u{1161}\u{11eb}";
fn run(index: usize, text: &str, locale: Option<&str>) -> GlyphRun {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(index).unwrap();
    let mut engine = face
        .engine()
        .with_font_size(1000.0)
        .with_shaping_policy(ShapingPolicy::TopToBottom);
    if let Some(locale) = locale {
        engine = engine.with_locale(locale);
    }
    engine.shape(text).unwrap()
}
fn same(a: GlyphRun, b: GlyphRun) {
    assert_eq!(format!("{:?}", a), format!("{:?}", b));
}
#[test]
#[cfg(feature = "layout")]
fn orientation_selection_order_and_mask_union() {
    for (face, text) in [
        (1, "\u{e003}\u{e001}\u{e002}"),
        (2, "\u{e003}\u{e001}\u{e002}"),
        (3, "\u{e000}\u{e001}\u{e002}"),
        (4, "A\u{1161}\u{11eb}"),
        (12, "\u{e003}\u{e001}\u{e002}"),
    ] {
        same(run(face, OLD, None), run(0, text, None));
    }
}
#[test]
fn unsupported_or_foreign_orientation_preserves_legacy() {
    same(run(5, OLD, None), run(6, OLD, None));
    same(run(7, OLD, Some("kor")), run(0, OLD, None));
    for face in [8, 9, 11, 14, 16] {
        same(run(face, OLD, None), run(0, OLD, None));
    }
}
#[test]
#[cfg(feature = "layout")]
fn authored_vertical_advance_is_applied_once_not_inferred_from_hmtx() {
    let positioned = run(2, OLD, None);
    let no_gpos = run(10, OLD, None);
    assert_eq!(
        positioned
            .glyphs
            .iter()
            .map(|g| g.glyph.metrics.advance_y)
            .collect::<Vec<_>>(),
        vec![1000.0, 0.0, 0.0]
    );
    assert_eq!(
        no_gpos
            .glyphs
            .iter()
            .map(|g| g.glyph.metrics.advance_y)
            .collect::<Vec<_>>(),
        vec![1000.0, 1000.0, 1000.0]
    );
    same(positioned, run(0, "\u{e003}\u{e001}\u{e002}", None));
}
#[test]
fn exclusions_and_no_layout_keep_original_units() {
    for text in [
        "A\u{1100}\u{1161}\u{11eb}",
        "\u{1100}\u{1161}\u{11eb}\u{302e}\u{302e}",
        "\u{119e}",
    ] {
        same(run(1, text, None), run(0, text, None));
    }
    #[cfg(not(feature = "layout"))]
    same(run(1, OLD, None), run(0, OLD, None));
}

#[test]
#[cfg(feature = "layout")]
fn vertical_family_requires_candidate_and_legacy_coverage() {
    use fontcore::{FontFaceDescriptor, FontFamily, TextDirection};
    let file = FontFile::from_buffer(FONT).unwrap();
    same(run(13, OLD, None), run(0, "\u{e003}\u{e001}\u{e002}", None));
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), file.face(13).unwrap());
    family.add_face(FontFaceDescriptor::new("Fallback"), file.face(0).unwrap());
    let options = family
        .options()
        .with_text_direction(TextDirection::TopToBottom)
        .with_font_size(1000.0);
    for text in [OLD, "A\u{1100}\u{1161}\u{11eb}"] {
        same(
            family.text2glyph_run(text, options.clone()).unwrap(),
            run(0, text, None),
        );
    }
}

#[test]
#[cfg(feature = "layout")]
fn horizontal_only_shared_lookup_is_not_admitted_even_when_jamo_references_it() {
    same(run(15, OLD, None), run(0, "\u{e003}\u{1161}\u{11eb}", None));
}
