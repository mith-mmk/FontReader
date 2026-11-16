use super::Font;
use crate::{FontFile, FontOptions};

#[test]
fn mapped_parser_keeps_original_offsets_across_discarded_controls() {
    let text = "\r\u{fe0f}a\u{308}\r\n\tß";
    let units = Font::parse_text_units_mapped(text);
    assert_eq!(
        units.iter().map(|(_, r)| r.clone()).collect::<Vec<_>>(),
        vec![4..7, 7..9, 9..10, 10..12]
    );
    for (_, range) in units {
        assert!(text.get(range).is_some());
    }
}

#[test]
fn mapped_shape_matches_legacy_and_retains_original_multiline_bytes() {
    let file = FontFile::from_buffer(include_bytes!("../../tests/fixtures/european-fallback.ttc"))
        .unwrap();
    let face = file.face(1).unwrap();
    let options = FontOptions::new(&face);
    let text = "\r\u{fe0f}a\u{308}\r\n\tß";
    let (mapped, ranges) = face.font().text2glyph_run_mapped(text, &options).unwrap();
    let legacy = face.font().text2glyph_run(text, &options).unwrap();
    assert_eq!(format!("{mapped:?}"), format!("{legacy:?}"));
    assert_eq!(mapped.glyphs.len(), ranges.len());
    assert_eq!(ranges.last(), Some(&(10..12)));
    assert!(ranges[..ranges.len() - 1]
        .iter()
        .all(|range| *range == (4..7)));
}

#[test]
fn mapped_ligature_owns_all_input_graphemes() {
    let file = FontFile::from_buffer(include_bytes!("../../tests/fixtures/european-fallback.ttc"))
        .unwrap();
    let face = file.face(1).unwrap();
    let (_, ranges) = face
        .font()
        .text2glyph_run_mapped("ßfiẞ", &FontOptions::new(&face))
        .unwrap();
    #[cfg(feature = "layout")]
    assert_eq!(ranges, vec![0..2, 2..4, 4..7]);
    #[cfg(not(feature = "layout"))]
    assert_eq!(ranges, vec![0..2, 2..3, 3..4, 4..7]);
}
