#![cfg(feature = "cff2")]
//! CFF2 variable outline coordinate sensitivity and malformed-payload guards.

use fontcore::{FontFile, GlyphLayer};

const FONT: &[u8] = include_bytes!("fixtures/cff2-vertical-origin.ttc");

#[test]
fn cff2_outlines_vary_with_axis_coordinates() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(1).unwrap();

    let mut signatures = Vec::new();
    for weight in [400.0_f32, 650.0, 900.0] {
        let run = face
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", weight)
            .shape("A")
            .unwrap();
        let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
            panic!("expected CFF2 path layer");
        };
        assert!(!path.commands.is_empty());
        signatures.push(format!("{:?}", path.commands));
    }

    // 400, 650, and 900 produce distinct, deterministic variable CFF2 outlines.
    assert_ne!(signatures[0], signatures[1]);
    assert_ne!(signatures[1], signatures[2]);
    assert_ne!(signatures[0], signatures[2]);

    // Repeat call with 650 matches deterministically.
    let repeat = face
        .engine()
        .with_font_size(1000.0)
        .with_variation("wght", 650.0)
        .shape("A")
        .unwrap();
    let GlyphLayer::Path(repeat_path) = &repeat.glyphs[0].glyph.layers[0] else {
        panic!("expected path layer");
    };
    assert_eq!(format!("{:?}", repeat_path.commands), signatures[1]);
}

#[test]
fn truncated_cff2_table_fails_safely_without_panic() {
    // Minimal OTTO font with truncated CFF2 header / data must return Err, never panic.
    let mut font = Vec::new();
    // sfnt header: OTTO, 1 table
    font.extend_from_slice(b"OTTO");
    font.extend_from_slice(&1u16.to_be_bytes());
    font.extend_from_slice(&16u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());
    // CFF2 directory entry: offset 28, length 4
    font.extend_from_slice(b"CFF2");
    font.extend_from_slice(&0u32.to_be_bytes());
    font.extend_from_slice(&28u32.to_be_bytes());
    font.extend_from_slice(&4u32.to_be_bytes());
    // Truncated CFF2 header (major=2, minor=0, headerSize=5, topDictLength=2 -> truncated)
    font.extend_from_slice(&[2, 0, 5, 2]);

    let result = FontFile::from_buffer(&font).and_then(|f| f.face(0)?.engine().shape("A"));
    assert!(result.is_err());
}
