//! Original MIT synthetic regression fonts; no Python or installed font needed.
use fontcore::{FontFaceDescriptor, FontFamily, FontFile, GlyphRun, TextDirection};
const FONT: &[u8] = include_bytes!("fixtures/bounded-hangul.ttc");
const OLD: &str = "\u{1100}\u{1161}\u{11eb}";
fn same(a: GlyphRun, b: GlyphRun) {
    assert_eq!(a.glyphs.len(), b.glyphs.len());
    for (a, b) in a.glyphs.iter().zip(b.glyphs.iter()) {
        assert_eq!(
            (
                a.x,
                a.y,
                a.glyph.metrics.advance_x,
                a.glyph.metrics.advance_y
            ),
            (
                b.x,
                b.y,
                b.glyph.metrics.advance_x,
                b.glyph.metrics.advance_y
            )
        );
        assert_eq!(format!("{:?}", a.glyph), format!("{:?}", b.glyph));
    }
}
#[test]
#[cfg(feature = "layout")]
fn strict_context_shared_masks_and_extension_match_explicit_outlines() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [1, 2, 7, 15] {
        for locale in [None, Some("ko"), Some("kor"), Some("hang"), Some("en")] {
            let face = file.face(index).unwrap();
            let mut engine = face.engine().with_font_size(1000.0);
            if let Some(locale) = locale {
                engine = engine.with_locale(locale);
            }
            for text in [OLD, "\u{a960}\u{d7b0}\u{d7cb}"] {
                same(
                    engine.shape(text).unwrap(),
                    file.face(0)
                        .unwrap()
                        .engine()
                        .with_font_size(1000.0)
                        .shape("\u{e000}\u{e001}\u{e002}")
                        .unwrap(),
                );
            }
            same(
                engine.shape(&OLD.repeat(2)).unwrap(),
                file.face(0)
                    .unwrap()
                    .engine()
                    .with_font_size(1000.0)
                    .shape(&"\u{e000}\u{e001}\u{e002}".repeat(2))
                    .unwrap(),
            );
        }
    }
}
#[test]
fn unsupported_wrong_category_or_missing_output_preserves_complete_legacy_run() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [3, 4, 5, 8, 9, 10, 11, 12, 14, 17, 18] {
        same(
            file.face(index).unwrap().engine().shape(OLD).unwrap(),
            file.face(0).unwrap().engine().shape(OLD).unwrap(),
        );
    }
}
#[test]
fn exclusions_and_orientations_keep_legacy_output() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for text in [
        "\u{119e}",
        "\u{1100}\u{1161}\u{1161}",
        "\u{115f}\u{1161}",
        "\u{1100}\u{1160}",
        "\u{ac00}\u{11eb}",
        "\u{1100}\u{1161}\u{11eb}\u{302e}\u{302e}",
        "\u{1100}\u{1161}\u{11eb}\u{fe00}",
        "\u{1100}\u{1161}\u{11eb}\u{200d}",
        "A\u{1100}\u{1161}\u{11eb}",
    ] {
        same(
            file.face(1).unwrap().engine().shape(text).unwrap(),
            file.face(0).unwrap().engine().shape(text).unwrap(),
        );
    }
    for direction in [TextDirection::RightToLeft, TextDirection::TopToBottom] {
        same(
            file.face(1)
                .unwrap()
                .engine()
                .with_options(
                    file.face(0)
                        .unwrap()
                        .engine()
                        .options()
                        .with_text_direction(direction),
                )
                .shape(OLD)
                .unwrap(),
            file.face(0)
                .unwrap()
                .engine()
                .with_options(
                    file.face(0)
                        .unwrap()
                        .engine()
                        .options()
                        .with_text_direction(direction),
                )
                .shape(OLD)
                .unwrap(),
        );
    }
    same(
        file.face(1)
            .unwrap()
            .engine()
            .shape_sideways_ltr(OLD)
            .unwrap(),
        file.face(0)
            .unwrap()
            .engine()
            .shape_sideways_ltr(OLD)
            .unwrap(),
    );
    same(
        file.face(1)
            .unwrap()
            .engine()
            .shape_vertical_mixed_ltr(OLD)
            .unwrap(),
        file.face(0)
            .unwrap()
            .engine()
            .shape_vertical_mixed_ltr(OLD)
            .unwrap(),
    );
}
#[test]
fn family_grouping_and_ccmp_contraction_keep_legacy_coverage() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [1, 6] {
        let mut family = FontFamily::new("Primary");
        family.add_face(
            FontFaceDescriptor::new("Primary"),
            file.face(index).unwrap(),
        );
        family.add_face(FontFaceDescriptor::new("Fallback"), file.face(0).unwrap());
        for text in [OLD, "A\u{1100}\u{1161}\u{11eb}"] {
            same(
                family.text2glyph_run(text, family.options()).unwrap(),
                file.face(index).unwrap().engine().shape(text).unwrap(),
            );
        }
    }
    #[cfg(feature = "layout")]
    same(
        file.face(6).unwrap().engine().shape(OLD).unwrap(),
        file.face(0).unwrap().engine().shape("\u{e003}").unwrap(),
    );
}
#[test]
#[cfg(not(feature = "layout"))]
fn no_layout_leaves_jamo_nominal() {
    let file = FontFile::from_buffer(FONT).unwrap();
    same(
        file.face(1).unwrap().engine().shape(OLD).unwrap(),
        file.face(0).unwrap().engine().shape(OLD).unwrap(),
    );
}

#[test]
#[cfg(feature = "layout")]
fn requested_language_is_selected_without_merging_default_or_other_languages() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(16).unwrap();
    same(
        face.engine().shape(OLD).unwrap(),
        file.face(0)
            .unwrap()
            .engine()
            .shape("\u{e000}\u{e001}\u{e002}")
            .unwrap(),
    );
    same(
        face.engine().with_locale("kor").shape(OLD).unwrap(),
        file.face(0)
            .unwrap()
            .engine()
            .shape("\u{e003}\u{1161}\u{11eb}")
            .unwrap(),
    );
}
#[test]
#[cfg(feature = "layout")]
fn family_rejects_candidate_only_coverage_before_grouping() {
    let file = FontFile::from_buffer(FONT).unwrap();
    same(
        file.face(13).unwrap().engine().shape(OLD).unwrap(),
        file.face(0)
            .unwrap()
            .engine()
            .shape("\u{e000}\u{e001}\u{e002}")
            .unwrap(),
    );
    let mut family = FontFamily::new("Unsafe");
    family.add_face(FontFaceDescriptor::new("Unsafe"), file.face(13).unwrap());
    family.add_face(FontFaceDescriptor::new("Safe"), file.face(0).unwrap());
    for text in [OLD, "A\u{1100}\u{1161}\u{11eb}"] {
        same(
            family.text2glyph_run(text, family.options()).unwrap(),
            file.face(0).unwrap().engine().shape(text).unwrap(),
        );
    }
}
