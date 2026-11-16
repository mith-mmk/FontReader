//! Original MIT synthetic fixtures; no installed fonts required.
use fontcore::{FontFile, GlyphRun};
const FONT: &[u8] = include_bytes!("fixtures/latin-composition.ttc");
const PAIRS: &[(&str, &str)] = &[
    ("a\u{303}\u{301}", "ã\u{301}"),
    ("a\u{325}\u{303}", "ḁ\u{303}"),
    ("a\u{306}\u{301}", "ắ"),
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
fn composed_prefix_and_residual_mark_match_nfc_including_cursor() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [0, 2] {
        let face = file.face(index).unwrap();
        let engine = face.engine();
        for (source, nfc) in PAIRS {
            same(&engine.shape(source).unwrap(), &engine.shape(nfc).unwrap());
            same(
                &engine.shape(&format!("{source}B{source}B")).unwrap(),
                &engine.shape(&format!("{nfc}B{nfc}B")).unwrap(),
            );
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn longest_renderable_prefix_keeps_uncomposed_tail() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(4).unwrap();
    let engine = face.engine();
    same(
        &engine.shape("a\u{306}\u{301}B").unwrap(),
        &engine.shape("ă\u{301}B").unwrap(),
    );
}

#[test]
fn missing_composed_glyph_keeps_original_scalars() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(1).unwrap();
    for (source, _) in PAIRS {
        let run = face.engine().shape(&format!("{source}B")).unwrap();
        assert_eq!(run.glyphs.len(), 4, "{source:?}");
    }
}

#[test]
#[cfg(feature = "layout")]
fn failed_gsub_restores_exact_originals_once_and_preserves_tail() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let failing = file.face(3).unwrap();
    let nominal = file.face(1).unwrap();
    for (source, _) in PAIRS {
        let text = format!("{source}B{source}B");
        same(
            &failing.engine().shape(&text).unwrap(),
            &nominal.engine().shape(&text).unwrap(),
        );
    }
}

#[test]
fn outside_bounded_scope_remains_uncomposed() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    for source in ["a\u{303}\u{325}", "a\u{303}\u{301}\u{301}"] {
        assert_eq!(
            face.engine().shape(source).unwrap().glyphs.len(),
            source.chars().count()
        );
    }
}

#[test]
fn rtl_vertical_and_special_modes_keep_original_scalars() {
    use fontcore::TextDirection;
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    for direction in [TextDirection::RightToLeft, TextDirection::TopToBottom] {
        let options = face.engine().options().with_text_direction(direction);
        let engine = face.engine().with_options(options);
        for (source, _) in PAIRS {
            assert_eq!(engine.shape(source).unwrap().glyphs.len(), 3);
        }
    }
    for (source, _) in PAIRS {
        assert_eq!(
            face.engine()
                .shape_sideways_ltr(source)
                .unwrap()
                .glyphs
                .len(),
            3
        );
        assert_eq!(
            face.engine()
                .shape_vertical_mixed_ltr(source)
                .unwrap()
                .glyphs
                .len(),
            3
        );
    }
}

#[test]
#[cfg(feature = "layout")]
fn failed_gsub_restores_original_mark_to_mark_attachment() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let failing = file.face(5).unwrap();
    let nominal = file.face(6).unwrap();
    let text = "a\u{303}\u{301}B";
    same(
        &failing.engine().shape(text).unwrap(),
        &nominal.engine().shape(text).unwrap(),
    );
}
