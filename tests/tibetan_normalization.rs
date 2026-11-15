//! Original MIT font fixture; no installed fonts, network or Python needed.
use fontcore::{FontFaceDescriptor, FontFamily, FontFile, GlyphRun, TextDirection};
const FONT: &[u8] = include_bytes!("fixtures/tibetan-normalization.ttc");
const CONTROL: usize = 8;

fn shape(index: usize, text: &str, locale: Option<&str>) -> GlyphRun {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(index).unwrap();
    let mut engine = face.engine().with_font_size(1000.0);
    if let Some(locale) = locale {
        engine = engine.with_locale(locale);
    }
    engine.shape(text).unwrap()
}
fn same_geometry(a: &GlyphRun, b: &GlyphRun) {
    assert_eq!(a.glyphs.len(), b.glyphs.len());
    for (a, b) in a.glyphs.iter().zip(&b.glyphs) {
        assert_eq!((a.x, a.y), (b.x, b.y));
        assert_eq!(format!("{:?}", a.glyph), format!("{:?}", b.glyph));
    }
}

#[test]
#[cfg(feature = "layout")]
fn canonical_forms_and_reordered_lower_vowels_keep_geometry() {
    for locale in [None, Some("bo"), Some("dz"), Some("tibt"), Some("en")] {
        for (compound, decomposed) in [
            ("\u{f43}", "\u{f42}\u{fb7}"),
            ("\u{f4d}", "\u{f4c}\u{fb7}"),
            ("\u{f52}", "\u{f51}\u{fb7}"),
            ("\u{f57}", "\u{f56}\u{fb7}"),
            ("\u{f5c}", "\u{f5b}\u{fb7}"),
            ("\u{f69}", "\u{f40}\u{fb5}"),
            ("\u{f40}\u{f93}", "\u{f40}\u{f92}\u{fb7}"),
            ("\u{f40}\u{f9d}", "\u{f40}\u{f9c}\u{fb7}"),
            ("\u{f40}\u{fa2}", "\u{f40}\u{fa1}\u{fb7}"),
            ("\u{f40}\u{fa7}", "\u{f40}\u{fa6}\u{fb7}"),
            ("\u{f40}\u{fac}", "\u{f40}\u{fab}\u{fb7}"),
            ("\u{f40}\u{fb9}", "\u{f40}\u{f90}\u{fb5}"),
            ("\u{f40}\u{f75}", "\u{f40}\u{f71}\u{f74}"),
            ("\u{f40}\u{f74}\u{f71}", "\u{f40}\u{f71}\u{f74}"),
        ] {
            let actual = shape(0, compound, locale);
            assert_eq!(actual.glyphs.len(), 1, "{compound:?} {locale:?}");
            same_geometry(&actual, &shape(0, decomposed, locale));
            let repeated = format!("{compound}{compound}{compound}");
            let repeated = shape(0, &repeated, locale);
            assert_eq!(repeated.glyphs.len(), 3);
        }
    }
}

#[test]
fn unsupported_forms_selectors_and_mixed_runs_are_never_partially_normalized() {
    for locale in [None, Some("bo"), Some("tibt")] {
        for text in [
            "\u{f40}\u{f73}",
            "\u{f40}\u{f81}",
            "\u{f40}\u{f76}",
            "\u{f40}\u{f78}",
            "\u{f40}\u{f77}",
            "\u{f40}\u{f79}",
            "\u{f40}\u{f71}\u{f72}",
            "\u{f40}\u{f72}\u{f71}",
            "\u{f43}\u{fe00}",
            "\u{f43}\u{e0100}",
            "\u{f40}\u{f75}\u{fe00}",
            "\u{f75}",
            "\u{f93}",
            "A\u{f43}",
            "\u{f43}A",
            "\u{f43} \u{f43}",
            "\u{f43}\u{f0b}",
            "\u{f43}\u{f0d}",
            "\u{f43}\u{200d}",
            "\u{f43}\u{200c}",
            "\u{f43}\u{34f}",
            "\u{f43}\u{301}",
            "\u{f43}\u{f40}\u{f73}",
            "\u{f40}\u{f74}\u{f90}\u{f71}",
            "\u{f40}\u{f74}\u{f74}",
        ] {
            same_geometry(&shape(0, text, locale), &shape(CONTROL, text, locale));
        }
    }
}

#[test]
fn missing_components_and_failed_candidates_restore_available_originals() {
    for locale in [None, Some("bo"), Some("tibt")] {
        for (face, text) in [
            (1, "\u{f40}\u{f75}"),
            (3, "\u{f43}"),
            (3, "\u{f40}\u{f75}"),
            (3, "\u{f40}\u{f71}\u{f74}"),
            (3, "\u{f40}\u{f74}\u{f71}"),
            (4, "\u{f43}"),
            (4, "\u{f40}\u{f75}"),
            (5, "\u{f43}"),
            (5, "\u{f40}\u{f75}"),
            (7, "\u{f43}"),
            (7, "\u{f40}\u{f75}"),
        ] {
            same_geometry(
                &shape(face, text, locale),
                &shape(face + CONTROL, text, locale),
            );
        }
    }
}

#[test]
fn family_probes_original_graphemes_before_grouping_or_normalizing() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for primary in [2, 6] {
        let mut family = FontFamily::new("Primary");
        family.add_face(
            FontFaceDescriptor::new("Primary"),
            file.face(primary).unwrap(),
        );
        family.add_face(FontFaceDescriptor::new("Fallback"), file.face(0).unwrap());
        for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
            let options = family
                .options()
                .with_font_size(1000.0)
                .with_text_direction(direction)
                .with_locale("tibt");
            let cluster = if primary == 2 {
                "\u{f43}"
            } else {
                "\u{f40}\u{f75}"
            };
            for text in [
                cluster.to_string(),
                format!("{cluster}A"),
                format!("{cluster} {cluster}"),
            ] {
                let actual = family.text2glyph_run(&text, options.clone()).unwrap();
                let expected = file
                    .face(0)
                    .unwrap()
                    .engine()
                    .with_options(options.clone())
                    .shape(cluster)
                    .unwrap();
                // The neighboring A/space may use the primary, but the complete
                // original Tibetan grapheme must use the fallback face.
                let first = &actual.glyphs[0];
                let expected = &expected.glyphs[0];
                assert_eq!(
                    format!("{:?}", first.glyph),
                    format!("{:?}", expected.glyph),
                    "{text:?}"
                );
            }
        }
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn no_layout_does_not_normalize_or_reorder() {
    for face in 0..CONTROL {
        for text in ["\u{f43}", "\u{f40}\u{f75}", "\u{f40}\u{f74}\u{f71}"] {
            same_geometry(&shape(face, text, None), &shape(face + CONTROL, text, None));
        }
    }
}

#[test]
fn newly_admitted_lower_vowels_preserve_family_primary_on_failed_substitution() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), file.face(3).unwrap());
    family.add_face(FontFaceDescriptor::new("Fallback"), file.face(0).unwrap());
    let options = family.options().with_font_size(1000.0).with_locale("tibt");
    for text in [
        "\u{f40}\u{f75}",
        "\u{f40}\u{f71}\u{f74}",
        "\u{f40}\u{f74}\u{f71}",
    ] {
        let actual = family.text2glyph_run(text, options.clone()).unwrap();
        same_geometry(&actual, &shape(3 + CONTROL, text, Some("tibt")));
    }
}
