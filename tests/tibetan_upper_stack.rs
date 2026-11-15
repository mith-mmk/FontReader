//! Original MIT synthetic fonts, no installed font or Python runtime dependency.
use fontcore::{FontFile, GlyphRun};
const FONT: &[u8] = include_bytes!("fixtures/tibetan-upper-stack.ttc");
const CONTROL: usize = 24;
fn shape(face: usize, text: &str, locale: Option<&str>, size: f32) -> GlyphRun {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(face).unwrap();
    let mut engine = face.engine().with_font_size(size);
    if let Some(locale) = locale {
        engine = engine.with_locale(locale);
    }
    engine.shape(text).unwrap()
}
fn same(a: &GlyphRun, b: &GlyphRun) {
    assert_eq!(format!("{:?}", a), format!("{:?}", b));
}
#[cfg(feature = "layout")]
#[test]
fn upper_stacks_require_complete_base_and_use_scaled_authored_anchors() {
    for face in [0, 1, 2, 21] {
        for locale in [
            None,
            Some("bo"),
            Some("tibt"),
            Some("dz"),
            Some("en"),
            Some("bo-Tibt-CN"),
        ] {
            for upper in [
                '\u{f72}', '\u{f7a}', '\u{f7b}', '\u{f7c}', '\u{f7d}', '\u{f80}',
            ] {
                for subs in ["\u{f90}", "\u{f90}\u{f90}"] {
                    for size in [500.0, 1000.0] {
                        let text = format!("\u{f40}{subs}{upper}");
                        let run = shape(face, &text, locale, size);
                        assert_eq!(run.glyphs.len(), 2, "{face} {text:?}");
                        let (base, mark) = (&run.glyphs[0], &run.glyphs[1]);
                        let expected = if face == 2 && subs.chars().count() == 1 {
                            (450.0, 100.0)
                        } else {
                            (250.0, -100.0)
                        };
                        assert_eq!(
                            (mark.x - base.x, mark.y - base.y),
                            (expected.0 * size / 1000.0, expected.1 * size / 1000.0)
                        );
                        assert_eq!(base.glyph.metrics.advance_x, 600.0 * size / 1000.0);
                        assert_eq!(mark.glyph.metrics.advance_x, 0.0);
                        let repeated = shape(face, &text.repeat(3), locale, size);
                        assert_eq!(repeated.glyphs.len(), 6);
                    }
                }
            }
        }
    }
}
#[test]
fn absent_proofs_and_bad_candidates_restore_complete_legacy_run() {
    for face in (3..CONTROL).filter(|&face| face != 21) {
        for locale in [None, Some("bo"), Some("tibt")] {
            for text in ["ཀྐི", "ཀྐིཀ", "ཀྐཀྐི"] {
                if face == 17 && text != "ཀྐིཀ" {
                    continue;
                }
                same(
                    &shape(face, text, locale, 1000.0),
                    &shape(face + CONTROL, text, locale, 1000.0),
                );
            }
        }
    }
}
#[test]
fn unsupported_vowels_and_run_barriers_do_not_get_partial_stack_support() {
    for text in [
        "ཀྐཱི",
        "ཀྐཱི",
        "ཀྐཱྀ",
        "ཀྐིུ",
        "ཀྐིུ",
        "ཀྐིི",
        "ཀྐིཀཱི",
        "ཀྐི་",
        "ཀྐི།",
        "Aཀྐི",
        "ཀྐིA",
        "ཀྐི ཀྐི",
        "ཀྐི\u{200d}",
        "ཀྐི\u{200c}",
        "ཀྐི\u{fe00}",
        "ཀྐི\u{e0100}",
        "ཀྐི\u{301}",
        "ཀྐི\u{34f}",
    ] {
        same(
            &shape(0, text, None, 1000.0),
            &shape(CONTROL, text, None, 1000.0),
        );
    }
}
#[cfg(feature = "layout")]
#[test]
fn canonical_consonants_remain_wholly_owned_by_one_base() {
    for (a, b) in [("གྷི", "གྷི"), ("ཀྒྷི", "ཀྒྷི")] {
        let a = shape(0, a, None, 1000.0);
        let b = shape(0, b, None, 1000.0);
        assert_eq!(a.glyphs.len(), 2);
        same(&a, &b);
    }
}
#[cfg(not(feature = "layout"))]
#[test]
fn no_layout_has_no_upper_stack_activation() {
    for face in 0..CONTROL {
        same(
            &shape(face, "ཀྐི", None, 1000.0),
            &shape(face + CONTROL, "ཀྐི", None, 1000.0),
        );
    }
}

#[cfg(feature = "layout")]
#[test]
fn explicit_vertical_and_rotated_routes_keep_certified_upper_attachment() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(1000.0);
    let vertical = engine.clone().with_vertical_flow().shape("ཀྐི").unwrap();
    assert_eq!(vertical.glyphs.len(), 2);
    assert_eq!(
        (
            vertical.glyphs[1].x - vertical.glyphs[0].x,
            vertical.glyphs[1].y - vertical.glyphs[0].y
        ),
        (250.0, -100.0)
    );
    for run in [
        engine.shape_vertical_mixed_ltr("ཀྐི").unwrap(),
        engine.shape_sideways_ltr("ཀྐི").unwrap(),
    ] {
        assert_eq!(run.glyphs.len(), 2);
        assert_eq!(
            (
                run.glyphs[1].x - run.glyphs[0].x,
                run.glyphs[1].y - run.glyphs[0].y
            ),
            (100.0, 250.0)
        );
    }
}
