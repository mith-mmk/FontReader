//! Font selection and pen advances, not editor caret/hit-testing APIs.
use fontcore::{CompositeFont, FontFaceDescriptor, FontFamily, FontFile};
const FONT: &[u8] = include_bytes!("fixtures/european-fallback.ttc");
const UMLAUTS: &[(&str, &str)] = &[
    ("ä", "a\u{308}"),
    ("ö", "o\u{308}"),
    ("ü", "u\u{308}"),
    ("Ä", "A\u{308}"),
    ("Ö", "O\u{308}"),
    ("Ü", "U\u{308}"),
];

fn family(fallback: usize) -> FontFamily {
    let file = FontFile::from_buffer(FONT).unwrap();
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), file.face(0).unwrap());
    family.add_face(
        FontFaceDescriptor::new("Fallback"),
        file.face(fallback).unwrap(),
    );
    family
}

#[test]
fn family_selects_whole_european_clusters() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let full = file.face(1).unwrap();
    let family = family(1);
    for text in UMLAUTS.iter().flat_map(|&(a, b)| [a, b]).chain(["ß", "ẞ"]) {
        let options = family.options().with_font_size(1000.0);
        let expected = full
            .engine()
            .with_options(options.clone())
            .shape(text)
            .unwrap();
        let actual = family.text2glyph_run(text, options.clone()).unwrap();
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"), "{text:?}");
        assert_eq!(family.measure_with_options(text, options).unwrap(), 800.0);
    }
}

#[test]
fn incomplete_faces_never_split_umlaut_base_and_mark() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let primary = file.face(0).unwrap();
    let family = family(2); // Only the mark exists in this fallback.
    for &(_, text) in UMLAUTS {
        let options = family.options().with_font_size(1000.0);
        let actual = family.text2glyph_run(text, options.clone()).unwrap();
        let expected = primary.engine().with_options(options).shape(text).unwrap();
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    }
}

#[test]
fn composite_fallback_preserves_following_span_pen_position() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let primary = file.face(0).unwrap();
    let full = file.face(1).unwrap();
    let composite = CompositeFont::new(vec![&primary, &full])
        .unwrap()
        .with_font_size(1000.0);
    for text in ["ä", "a\u{308}", "ß", "ẞ"] {
        let run = composite.shape_ltr(&format!("B{text}B")).unwrap();
        assert_eq!(run.glyphs.first().unwrap().x, 0.0);
        assert_eq!(run.glyphs.last().unwrap().x, 1400.0);
        assert_eq!(
            run.glyphs
                .iter()
                .map(|g| g.glyph.metrics.advance_x)
                .sum::<f32>(),
            2000.0
        );
    }
}

#[test]
fn umlaut_normalization_preserves_pen_advance_not_scalar_count() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let full = file.face(1).unwrap();
    let engine = full.engine().with_font_size(1000.0);
    for &(nfc, nfd) in UMLAUTS {
        for text in [nfc, nfd] {
            let run = engine.shape(&format!("{text}B{text}B")).unwrap();
            assert_eq!(run.glyphs.last().unwrap().x, 2400.0);
            assert_eq!(engine.measure(&format!("{text}B{text}B")).unwrap(), 3200.0);
        }
    }
}

#[test]
fn ligature_pen_position_uses_shaped_advance_not_character_count() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let full = file.face(1).unwrap();
    let engine = full.engine().with_font_size(1000.0);
    let run = engine.shape("fiBfiB").unwrap();
    let count = if cfg!(feature = "layout") { 4 } else { 6 };
    assert_eq!(run.glyphs.len(), count);
    for (index, glyph) in run.glyphs.iter().enumerate() {
        assert_eq!(glyph.x, index as f32 * 800.0);
    }
    assert_eq!(engine.measure("fiBfiB").unwrap(), count as f64 * 800.0);
}
