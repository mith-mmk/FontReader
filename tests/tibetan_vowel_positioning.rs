use fontcore::{FontFile, FontOptions, FontStretch, GlyphRun};

const FONT: &[u8] = include_bytes!("fixtures/tibetan-vowel-positioning.ttc");
const CONTROL_OFFSET: usize = 26;

fn shape(index: usize, text: &str, locale: Option<&str>, size: f32, stretch: f32) -> GlyphRun {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(index).unwrap();
    let mut options = FontOptions::new(&face)
        .with_font_size(size)
        .with_font_stretch(FontStretch(stretch));
    if let Some(locale) = locale {
        options = options.with_locale(locale);
    }
    face.engine().with_options(options).shape(text).unwrap()
}

#[cfg(feature = "layout")]
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

fn unchanged(index: usize, text: &str, locale: Option<&str>) {
    let run = shape(index, text, locale, 1000.0, 1.0);
    let baseline = shape(index + CONTROL_OFFSET, text, locale, 1000.0, 1.0);
    assert_eq!(
        run.glyphs.len(),
        baseline.glyphs.len(),
        "face {index}, {text:?}, {locale:?}"
    );
    for (actual, expected) in run.glyphs.iter().zip(&baseline.glyphs) {
        assert_eq!(
            (actual.x, actual.y),
            (expected.x, expected.y),
            "face {index}, {text:?}, {locale:?}"
        );
        assert_eq!(
            actual.glyph.metrics.advance_x,
            expected.glyph.metrics.advance_x
        );
        assert_eq!(
            actual.glyph.metrics.advance_y,
            expected.glyph.metrics.advance_y
        );
    }
}

#[test]
#[cfg(feature = "layout")]
fn simple_vowels_use_tibetan_anchors_for_all_locales_and_scale_once() {
    for locale in [None, Some("bo"), Some("en")] {
        for consonant in ['\u{f40}', '\u{f47}', '\u{f49}', '\u{f6c}'] {
            for vowel in [
                '\u{f71}', '\u{f72}', '\u{f74}', '\u{f7a}', '\u{f7b}', '\u{f7c}', '\u{f7d}',
                '\u{f80}',
            ] {
                let bottom = matches!(vowel, '\u{f71}' | '\u{f74}');
                for (size, stretch) in [(1000.0, 1.0), (125.0, 2.0), (200.0, 0.5)] {
                    let run = shape(0, &format!("{consonant}{vowel}"), locale, size, stretch);
                    assert_eq!(run.glyphs.len(), 2);
                    close(
                        run.glyphs[1].x - run.glyphs[0].x,
                        if bottom { 200.0 } else { 250.0 } * size / 1000.0 * stretch,
                    );
                    close(
                        run.glyphs[1].y - run.glyphs[0].y,
                        if bottom { 100.0 } else { -100.0 } * size / 1000.0,
                    );
                    close(run.glyphs[1].glyph.metrics.advance_x, 0.0);
                    close(run.glyphs[1].glyph.metrics.advance_y, 0.0);
                }
            }
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn ordinary_mark_and_mark_to_mark_keep_priority() {
    for locale in [None, Some("bo"), Some("en")] {
        let run = shape(1, "\u{f40}\u{f72}", locale, 1000.0, 1.0);
        close(run.glyphs[1].x - run.glyphs[0].x, 450.0);
        close(run.glyphs[1].y - run.glyphs[0].y, 100.0);
        let run = shape(2, "\u{f40}\u{f72}\u{f72}", locale, 1000.0, 1.0);
        assert_eq!(run.glyphs.len(), 3);
        close(run.glyphs[1].x - run.glyphs[0].x, 250.0);
        close(run.glyphs[2].x - run.glyphs[1].x, 80.0);
        close(run.glyphs[2].y - run.glyphs[1].y, -200.0);
    }
}

#[test]
#[cfg(feature = "layout")]
fn dflt_does_not_supply_fallback_and_existing_language_selection_is_preserved() {
    for locale in [None, Some("bo"), Some("en")] {
        unchanged(3, "\u{f40}\u{f72}", locale);
        let run = shape(4, "\u{f40}\u{f72}", locale, 1000.0, 1.0);
        close(run.glyphs[1].x - run.glyphs[0].x, 250.0);
        close(run.glyphs[1].y - run.glyphs[0].y, -100.0);
    }
    // JAN is deliberately in tibt: script stays tibt but LangSys follows locale.
    let run = shape(4, "\u{f40}\u{f72}", Some("ja"), 1000.0, 1.0);
    close(run.glyphs[1].x - run.glyphs[0].x, 650.0);
    close(run.glyphs[1].y - run.glyphs[0].y, -300.0);
}

#[test]
#[cfg(feature = "layout")]
fn lookup_flags_filter_sets_missing_gdef_and_extension_are_respected() {
    for locale in [None, Some("bo"), Some("en")] {
        for index in [5, 6, 9, 11, 12, 16, 17, 18, 20, 21, 22, 24] {
            unchanged(index, "\u{f40}\u{f72}", locale);
            if index != 17 {
                unchanged(index, "\u{f40}\u{f74}", locale);
            }
        }
        for index in [7, 8, 10, 13, 14, 23, 25] {
            for (vowel, dx, dy) in [('\u{f72}', 250.0, -100.0), ('\u{f74}', 200.0, 100.0)] {
                let run = shape(index, &format!("\u{f40}{vowel}"), locale, 1000.0, 1.0);
                close(run.glyphs[1].x - run.glyphs[0].x, dx);
                close(run.glyphs[1].y - run.glyphs[0].y, dy);
            }
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn cmap_aliases_invalid_sequences_and_ligatures_do_not_enable_fallback() {
    for locale in [None, Some("bo"), Some("en")] {
        for text in [
            "A\u{301}",
            "A\u{323}",
            "A\u{f72}",
            "\u{f40}\u{301}",
            "\u{f48}\u{f72}",
            "\u{f6d}\u{f72}",
            "\u{f72}",
            "\u{f72}\u{f40}",
            "\u{f40}\u{f73}",
            "\u{f40}\u{f75}",
            "\u{f40}\u{f76}",
            "\u{f40}\u{f77}",
            "\u{f40}\u{f78}",
            "\u{f40}\u{f79}",
            "\u{f40}\u{f81}",
            "\u{f40}\u{f90}\u{f72}",
            "\u{f40} \u{f72}",
            "\u{f40}\n\u{f72}",
            "\u{f40}\t\u{f72}",
        ] {
            unchanged(0, text, locale);
        }
        unchanged(15, "\u{f40}\u{f40}\u{f72}", locale);
        unchanged(19, "\u{f40}\u{f72}\u{f74}", locale);
        assert_eq!(
            shape(19, "\u{f40}\u{f72}\u{f74}", locale, 1000.0, 1.0)
                .glyphs
                .len(),
            2
        );
        assert_eq!(
            shape(15, "\u{f40}\u{f40}\u{f72}", locale, 1000.0, 1.0)
                .glyphs
                .len(),
            2
        );
        // The first simple vowel may attach; a second vowel must not reuse abvm.
        let run = shape(0, "\u{f40}\u{f72}\u{f72}", locale, 1000.0, 1.0);
        let baseline = shape(CONTROL_OFFSET, "\u{f40}\u{f72}\u{f72}", locale, 1000.0, 1.0);
        close(run.glyphs[2].x, baseline.glyphs[2].x);
        close(run.glyphs[2].y, baseline.glyphs[2].y);
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn no_layout_ignores_all_tibetan_positioning_tables() {
    for index in 0..CONTROL_OFFSET {
        for locale in [None, Some("bo"), Some("en")] {
            for text in [
                "\u{f40}\u{f72}",
                "\u{f40}\u{f74}",
                "\u{f40}\u{f72}\u{f72}",
                "A\u{301}",
            ] {
                unchanged(index, text, locale);
            }
        }
    }
}
