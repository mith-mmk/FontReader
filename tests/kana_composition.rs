//! Original MIT synthetic fixtures, independent of installed fonts or Python.
use fontcore::{FontFaceDescriptor, FontFamily, FontFile, GlyphRun, TextDirection};
const FONT: &[u8] = include_bytes!("fixtures/kana-composition.ttc");
#[cfg(feature = "layout")]
const PAIRS: &[(&str, &str)] = &[
    ("が", "が"),
    ("ぎ", "ぎ"),
    ("ぐ", "ぐ"),
    ("げ", "げ"),
    ("ご", "ご"),
    ("ざ", "ざ"),
    ("じ", "じ"),
    ("ず", "ず"),
    ("ぜ", "ぜ"),
    ("ぞ", "ぞ"),
    ("だ", "だ"),
    ("ぢ", "ぢ"),
    ("づ", "づ"),
    ("で", "で"),
    ("ど", "ど"),
    ("ば", "ば"),
    ("ぱ", "ぱ"),
    ("び", "び"),
    ("ぴ", "ぴ"),
    ("ぶ", "ぶ"),
    ("ぷ", "ぷ"),
    ("べ", "べ"),
    ("ぺ", "ぺ"),
    ("ぼ", "ぼ"),
    ("ぽ", "ぽ"),
    ("ゔ", "ゔ"),
    ("ゞ", "ゞ"),
    ("ガ", "ガ"),
    ("ギ", "ギ"),
    ("グ", "グ"),
    ("ゲ", "ゲ"),
    ("ゴ", "ゴ"),
    ("ザ", "ザ"),
    ("ジ", "ジ"),
    ("ズ", "ズ"),
    ("ゼ", "ゼ"),
    ("ゾ", "ゾ"),
    ("ダ", "ダ"),
    ("ヂ", "ヂ"),
    ("ヅ", "ヅ"),
    ("デ", "デ"),
    ("ド", "ド"),
    ("バ", "バ"),
    ("パ", "パ"),
    ("ビ", "ビ"),
    ("ピ", "ピ"),
    ("ブ", "ブ"),
    ("プ", "プ"),
    ("ベ", "ベ"),
    ("ペ", "ペ"),
    ("ボ", "ボ"),
    ("ポ", "ポ"),
    ("ヴ", "ヴ"),
    ("ヷ", "ヷ"),
    ("ヸ", "ヸ"),
    ("ヹ", "ヹ"),
    ("ヺ", "ヺ"),
    ("ヾ", "ヾ"),
];

fn same(actual: &GlyphRun, expected: &GlyphRun) {
    assert_eq!(actual.glyphs.len(), expected.glyphs.len());
    for (a, b) in actual.glyphs.iter().zip(&expected.glyphs) {
        assert_eq!((a.x, a.y), (b.x, b.y));
        assert_eq!(format!("{:?}", a.glyph), format!("{:?}", b.glyph));
    }
}

#[test]
#[cfg(feature = "layout")]
fn all_58_pairs_match_nfc_geometry_across_locales_and_orientations() {
    assert_eq!(PAIRS.len(), 58);
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [0, 1, 4] {
        let face = file.face(index).unwrap();
        for locale in [
            None,
            Some("ja"),
            Some("jan"),
            Some("kana"),
            Some("ain"),
            Some("en"),
        ] {
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
                for &(source, nfc) in PAIRS {
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
            }
        }
    }
}

#[test]
fn exclusions_and_ainu_named_sequences_preserve_nominal_geometry() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for text in [
        "セ゚",
        "ツ゚",
        "ト゚",
        "ㇷ゚",
        "パ゙",
        "パ\u{301}",
        "パ\u{34f}",
        "パ\u{fe00}",
        "パ\u{e0100}",
        "パ\u{200d}",
        "パ\u{200c}",
        "\u{600}パ",
        "ﾊﾞ",
        "ﾊﾟ",
        "ハ゛",
        "A\u{301}",
        "゚ハ",
    ] {
        same(
            &file.face(0).unwrap().engine().shape(text).unwrap(),
            &file.face(2).unwrap().engine().shape(text).unwrap(),
        );
    }
    for scalar in '\u{31f0}'..='\u{31ff}' {
        let text = scalar.to_string();
        same(
            &file.face(0).unwrap().engine().shape(&text).unwrap(),
            &file.face(2).unwrap().engine().shape(&text).unwrap(),
        );
    }
    for text in ["セ゚", "ツ゚", "ト゚", "ㇷ゚"] {
        let run = file.face(0).unwrap().engine().shape(text).unwrap();
        assert_eq!(run.glyphs.len(), 2);
        assert!(run.glyphs[1].x > 0.0);
    }
}

#[test]
#[cfg(feature = "layout")]
fn missing_target_preserves_both_originals() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [2, 3] {
        for &(source, _) in PAIRS {
            let actual = file.face(index).unwrap().engine().shape(source).unwrap();
            assert_eq!(actual.glyphs.len(), 2);
            same(
                &actual,
                &file.face(2).unwrap().engine().shape(source).unwrap(),
            );
        }
    }
}

#[test]
fn fallback_is_whole_grapheme_and_composed_only_faces_are_supported() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for primary in [1, 8] {
        let mut family = FontFamily::new("Primary");
        family.add_face(
            FontFaceDescriptor::new("Primary"),
            file.face(primary).unwrap(),
        );
        family.add_face(FontFaceDescriptor::new("Fallback"), file.face(7).unwrap());
        for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
            let options = family
                .options()
                .with_font_size(1000.0)
                .with_text_direction(direction);
            for text in ["パ", "AパB", "パ\u{301}", "ㇷ゚"] {
                let expected_face = if cfg!(feature = "layout")
                    && primary == 1
                    && !text.contains('\u{301}')
                    && text != "ㇷ゚"
                {
                    1
                } else {
                    7
                };
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
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn invalid_gsub_restores_mixed_kana_hangul_without_recursive_retry_loss() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (index, text, count) in [
        (5, "パ", 2),
        (6, "AパB", 4),
        (6, "각パ", 5),
        (6, "パ각", 5),
        (9, "힣AパB", 5),
        (9, "힣각パ", 6),
    ] {
        let expected = file.face(index).unwrap().engine().shape(text).unwrap();
        assert_eq!(expected.glyphs.len(), count, "{index}: {text}");
        assert!(expected.glyphs.iter().all(|g| !g.glyph.layers.is_empty()));
        let mut family = FontFamily::new("Primary");
        family.add_face(
            FontFaceDescriptor::new("Primary"),
            file.face(index).unwrap(),
        );
        family.add_face(FontFaceDescriptor::new("Fallback"), file.face(7).unwrap());
        same(
            &family.text2glyph_run(text, family.options()).unwrap(),
            &expected,
        );
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn disabled_layout_keeps_original_scalars_and_advances() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in 0..10 {
        for text in ["パ", "が", "ゞ", "ヾ"] {
            assert_eq!(
                file.face(index)
                    .unwrap()
                    .engine()
                    .shape(text)
                    .unwrap()
                    .glyphs
                    .len(),
                2
            );
        }
    }
}

#[test]
#[cfg(feature = "raw")]
fn raw_single_scalar_apis_keep_independent_nominal_kana() {
    let font = fontcore::fontreader::Font::get_font_from_buffer(FONT).unwrap();
    for (ch, id) in [('ハ', 97), ('\u{309a}', 63), ('パ', 99)] {
        assert_eq!(
            format!("{:?}", font.get_glyph(ch)),
            format!("{:?}", font.get_glyph_from_id(id))
        );
        assert_eq!(
            format!("{:?}", font.get_glyph_with_uvs(ch, '\0')),
            format!("{:?}", font.get_glyph_from_id(id))
        );
    }
}
