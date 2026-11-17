//! Public adapter contracts for supported GDEF ligature caret metadata.
#![cfg(feature = "layout")]
use fontcore::{
    CaretAffinity, CaretLayout, CaretQuality, CaretSource, CompositeFont, FontFaceDescriptor,
    FontFamily, FontFile, HardLineLayout, ShapedText, TextPosition,
};
const GDEF: &[u8] = include_bytes!("fixtures/gdef-carets.ttc");
const FALLBACK: &[u8] = include_bytes!("fixtures/european-fallback.ttc");
fn pos(byte: usize) -> TextPosition {
    TextPosition::new(byte, CaretAffinity::Downstream)
}

fn check_fallback_metadata(mapped: &ShapedText, expected_caret: f32) {
    assert_eq!(mapped.text(), "\u{308}fiB");
    assert_eq!(mapped.glyph_sources().len(), 3);
    assert_eq!(mapped.glyph_sources()[0].face_index(), 0);
    assert_eq!(mapped.glyph_sources()[1].face_index(), 1);
    assert_eq!(mapped.glyph_sources()[1].byte_range(), 2..4);
    assert_eq!(mapped.glyph_sources()[2].byte_range(), 4..5);
    assert_eq!(
        mapped.glyph_ligature_carets(),
        &[None, Some(vec![expected_caret]), None]
    );
    let layout = CaretLayout::new(mapped).unwrap();
    let internal = layout.caret_positions(pos(3)).unwrap()[0];
    assert_eq!(internal.x(), expected_caret); // Primary's standalone combining mark has zero advance.
    assert_eq!(internal.source(), CaretSource::FontAuthored);
    assert_eq!(internal.quality(), CaretQuality::Approximate); // Legacy enum stays conservative.
    assert_eq!(
        layout.hit_test(expected_caret, 0.0).unwrap().source(),
        CaretSource::FontAuthored
    );
    assert_eq!(
        layout.caret_positions(pos(4)).unwrap()[0].source(),
        CaretSource::Advance
    );
}

#[test]
fn family_fallback_preserves_authored_carets_and_global_mapping() {
    let gdef = FontFile::from_buffer(GDEF).unwrap();
    let other = FontFile::from_buffer(FALLBACK).unwrap();
    let mut family = FontFamily::new("Primary");
    // This primary contains only the combining mark. f/i/B must fall back.
    family.add_face(FontFaceDescriptor::new("Primary"), other.face(2).unwrap());
    family.add_face(FontFaceDescriptor::new("Fallback"), gdef.face(0).unwrap());
    let options = family.options().with_font_size(1000.0);
    let mapped = family.shape_mapped("\u{308}fiB", options.clone()).unwrap();
    check_fallback_metadata(&mapped, 220.0);
    assert_eq!(
        format!("{:?}", mapped.glyph_run()),
        format!(
            "{:?}",
            family.text2glyph_run("\u{308}fiB", options).unwrap()
        )
    );
}

#[test]
fn composite_fallback_preserves_authored_carets_and_global_mapping() {
    let gdef = FontFile::from_buffer(GDEF).unwrap();
    let other = FontFile::from_buffer(FALLBACK).unwrap();
    let primary = other.face(2).unwrap();
    let fallback = gdef.face(0).unwrap();
    let composite = CompositeFont::new(vec![&primary, &fallback])
        .unwrap()
        .with_font_size(1000.0);
    let mapped = composite.shape_mapped_ltr("\u{308}fiB").unwrap();
    check_fallback_metadata(&mapped, 220.0);
    assert_eq!(
        format!("{:?}", mapped.glyph_run()),
        format!("{:?}", composite.shape_ltr("\u{308}fiB").unwrap())
    );
}

#[test]
fn hard_line_wrappers_keep_font_authored_provenance() {
    let file = FontFile::from_buffer(GDEF).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);
    let layout = HardLineLayout::shape(&engine, "fi\r\nfl").unwrap();
    assert_eq!(
        layout.lines()[0].shaped().glyph_ligature_carets(),
        &[Some(vec![220.0])]
    );
    assert_eq!(
        layout.lines()[1].shaped().glyph_ligature_carets(),
        &[Some(vec![730.0])]
    );
    for (byte, x, y, line) in [(1, 220.0, 0.0, 0), (5, 730.0, 1200.0, 1)] {
        let caret = layout.caret_positions(pos(byte)).unwrap()[0];
        assert_eq!((caret.x(), caret.y(), caret.line_index()), (x, y, line));
        assert_eq!(caret.source(), CaretSource::FontAuthored);
        let hit = layout.hit_test(x, y).unwrap();
        assert_eq!(hit.text_position().byte_offset(), byte);
        assert_eq!(hit.source(), CaretSource::FontAuthored);
    }
    assert!(layout.caret_positions(pos(3)).is_err()); // CRLF interior remains invalid.
}

#[test]
fn mixed_vertical_wrappers_transform_authored_carets_without_losing_source() {
    let file = FontFile::from_buffer(GDEF).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(1000.0);
    // U+3000 selects an upright group; the fixture's .notdef is intentional.
    // This verifies orientation adaptation, not U+3000 font coverage.
    let text = "fi\u{3000}fl";
    let layout = engine.shape_vertical_mixed_ltr_mapped(text).unwrap();
    assert_eq!(layout.groups().len(), 3);
    assert_eq!(
        format!("{:?}", layout.glyph_run()),
        format!("{:?}", engine.shape_vertical_mixed_ltr(text).unwrap())
    );
    for (group_index, byte, coordinate) in [(0, 1, 220.0), (2, 6, 730.0)] {
        let group = &layout.groups()[group_index];
        assert!(group.is_sideways());
        assert_eq!(
            group.shaped().glyph_ligature_carets(),
            &[Some(vec![coordinate])]
        );
        let expected = group.transform_point(coordinate, 0.0);
        let caret = layout.caret_positions(pos(byte)).unwrap()[0];
        assert_eq!((caret.x(), caret.y()), expected);
        assert_eq!(caret.group_index(), group_index);
        assert_eq!(caret.source(), CaretSource::FontAuthored);
        let hit = layout.hit_test(caret.x(), caret.y()).unwrap();
        assert_eq!(hit.text_position().byte_offset(), byte);
        assert_eq!(hit.source(), CaretSource::FontAuthored);
    }
}

#[test]
fn format_two_survives_family_and_composite_fallback() {
    let gdef = FontFile::from_buffer(GDEF).unwrap();
    let other = FontFile::from_buffer(FALLBACK).unwrap();
    let primary = other.face(2).unwrap();
    let fallback = gdef.face(1).unwrap();
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), primary.clone());
    family.add_face(FontFaceDescriptor::new("Fallback"), fallback.clone());
    let mapped = family
        .shape_mapped("\u{308}fiB", family.options().with_font_size(1000.0))
        .unwrap();
    check_fallback_metadata(&mapped, 280.0);
    let composite = CompositeFont::new(vec![&primary, &fallback])
        .unwrap()
        .with_font_size(1000.0);
    check_fallback_metadata(&composite.shape_mapped_ltr("\u{308}fiB").unwrap(), 280.0);
}

#[test]
fn format_two_survives_hard_lines_and_sideways_group_transforms() {
    let file = FontFile::from_buffer(GDEF).unwrap();
    let face = file.face(1).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);
    let lines = HardLineLayout::shape(&engine, "fi\r\nfl").unwrap();
    for (byte, x, y) in [(1, 280.0, 0.0), (5, 830.0, 1200.0)] {
        let caret = lines.caret_positions(pos(byte)).unwrap()[0];
        assert_eq!((caret.x(), caret.y()), (x, y));
        assert_eq!(caret.source(), CaretSource::FontAuthored);
        assert_eq!(
            lines.hit_test(x, y).unwrap().source(),
            CaretSource::FontAuthored
        );
    }
    let vertical = engine
        .shape_vertical_mixed_ltr_mapped("fi\u{3000}fl")
        .unwrap();
    for (group, byte, point) in [(0, 1, 280.0), (2, 6, 830.0)] {
        let expected = vertical.groups()[group].transform_point(point, 0.0);
        let caret = vertical.caret_positions(pos(byte)).unwrap()[0];
        assert_eq!((caret.x(), caret.y()), expected);
        assert_eq!(caret.source(), CaretSource::FontAuthored);
        assert_eq!(
            vertical.hit_test(caret.x(), caret.y()).unwrap().source(),
            CaretSource::FontAuthored
        );
    }
}
