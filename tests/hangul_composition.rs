//! Original MIT synthetic fixtures; no installed fonts or Python required.
#[cfg(feature = "layout")]
use fontcore::FontVariant;
use fontcore::{FontFaceDescriptor, FontFamily, FontFile, GlyphRun, TextDirection};
const FONT: &[u8] = include_bytes!("fixtures/hangul-composition.ttc");

fn same(actual: &GlyphRun, expected: &GlyphRun) {
    assert_eq!(actual.glyphs.len(), expected.glyphs.len());
    for (a, b) in actual.glyphs.iter().zip(&expected.glyphs) {
        assert_eq!((a.x, a.y), (b.x, b.y));
        assert_eq!(format!("{:?}", a.glyph), format!("{:?}", b.glyph));
    }
}

#[test]
#[cfg(feature = "layout")]
fn canonical_forms_match_nfc_geometry_with_all_locales_scales_and_directions() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [0, 1, 6] {
        let face = file.face(index).unwrap();
        for locale in [None, Some("ko"), Some("kor"), Some("hang"), Some("en")] {
            for direction in [
                TextDirection::LeftToRight,
                TextDirection::RightToLeft,
                TextDirection::TopToBottom,
            ] {
                let options = face
                    .engine()
                    .options()
                    .with_font_size(37.0)
                    .with_text_direction(direction);
                let mut engine = face.engine().with_options(options);
                if let Some(locale) = locale {
                    engine = engine.with_locale(locale);
                }
                for (source, nfc) in [("가", "가"), ("각", "각"), ("각", "각"), ("힣", "힣")]
                {
                    same(&engine.shape(source).unwrap(), &engine.shape(nfc).unwrap());
                    same(
                        &engine
                            .shape(&format!("A{source}B\t{source}\n{source}{source}"))
                            .unwrap(),
                        &engine
                            .shape(&format!("A{nfc}B\t{nfc}\n{nfc}{nfc}"))
                            .unwrap(),
                    );
                    same(
                        &engine.shape_vertical_mixed_ltr(source).unwrap(),
                        &engine.shape_vertical_mixed_ltr(nfc).unwrap(),
                    );
                    same(
                        &engine.shape_sideways_ltr(source).unwrap(),
                        &engine.shape_sideways_ltr(nfc).unwrap(),
                    );
                }
                let engine = engine.with_font_variant(FontVariant::SmallCaps);
                same(&engine.shape("가").unwrap(), &engine.shape("가").unwrap());
            }
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn missing_composed_target_keeps_all_originals_not_an_lv_prefix() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [2, 3, 7] {
        let face = file.face(index).unwrap();
        let original = face.engine().shape("각").unwrap();
        assert_eq!(original.glyphs.len(), 3);
        same(
            &original,
            &file.face(2).unwrap().engine().shape("각").unwrap(),
        );
    }
    assert_eq!(
        file.face(3)
            .unwrap()
            .engine()
            .shape("각")
            .unwrap()
            .glyphs
            .len(),
        2
    );
    assert_eq!(
        file.face(4)
            .unwrap()
            .engine()
            .shape("각")
            .unwrap()
            .glyphs
            .len(),
        1
    );
}

#[test]
fn out_of_scope_old_jamo_marks_fillers_and_selectors_remain_nominal() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for text in [
        "ᄀ가",
        "가ᅡ",
        "각ᆨ",
        "가\u{d7cb}",
        "\u{a960}ᅡᆨ",
        "ᄀ\u{d7b0}ᆨ",
        "\u{115f}ᅡ",
        "ᄀ\u{1160}",
        "ㄱㅏ",
        "가\u{302e}",
        "가\u{fe00}",
        "가\u{e0100}",
        "가\u{200d}",
        "가\u{200c}",
        "가\u{301}",
        "\u{600}가",
    ] {
        same(
            &file.face(0).unwrap().engine().shape(text).unwrap(),
            &file.face(2).unwrap().engine().shape(text).unwrap(),
        );
    }
}

#[test]
fn family_keeps_each_original_grapheme_as_the_fallback_unit() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), file.face(1).unwrap());
    family.add_face(FontFaceDescriptor::new("Fallback"), file.face(8).unwrap());
    for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
        let options = family
            .options()
            .with_font_size(1000.0)
            .with_text_direction(direction);
        for text in ["가", "각", "각", "A각B"] {
            let expected_face = if cfg!(feature = "layout") { 1 } else { 8 };
            same(
                &family.text2glyph_run(text, options.clone()).unwrap(),
                &file
                    .face(expected_face)
                    .unwrap()
                    .engine()
                    .with_options(options.clone())
                    .shape(text)
                    .unwrap(),
            );
        }
        for text in ["가\u{301}", "가\u{d7cb}", "ᄀ가"] {
            same(
                &family.text2glyph_run(text, options.clone()).unwrap(),
                &file
                    .face(8)
                    .unwrap()
                    .engine()
                    .with_options(options.clone())
                    .shape(text)
                    .unwrap(),
            );
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn downstream_missing_output_restores_jamo_for_direct_and_grouped_family_runs() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (index, text, count) in [
        (9, "각", 3),
        (9, "각", 2),
        (10, "A각B", 5),
        (10, "힣A각B", 6),
    ] {
        let mut family = FontFamily::new("Primary");
        family.add_face(
            FontFaceDescriptor::new("Primary"),
            file.face(index).unwrap(),
        );
        family.add_face(FontFaceDescriptor::new("Fallback"), file.face(8).unwrap());
        let expected = file.face(index).unwrap().engine().shape(text).unwrap();
        assert_eq!(expected.glyphs.len(), count);
        same(
            &family.text2glyph_run(text, family.options()).unwrap(),
            &expected,
        );
        assert!(expected.glyphs.iter().all(|g| !g.glyph.layers.is_empty()));
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn no_layout_preserves_uncomposed_nominal_glyphs() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [0, 1, 2, 3, 4, 6, 9, 10] {
        let face = file.face(index).unwrap();
        for (source, count) in [("가", 2), ("각", 3), ("각", 2)] {
            assert_eq!(face.engine().shape(source).unwrap().glyphs.len(), count);
        }
    }
}
