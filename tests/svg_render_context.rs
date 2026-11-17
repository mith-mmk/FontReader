#![cfg(feature = "svg-fonts")]

use fontcore::{FontFile, FontOptions, FontStretch, GlyphLayer, ShapingPolicy};

fn source() -> Vec<u8> {
    let mut bytes = include_bytes!("fixtures/composite-adjustments-svg.ttf").to_vec();
    // Same-length change makes the viewport non-identity, preserving a raw
    // alternative without changing SFNT offsets. Fontcore ignores checksums.
    let needle = b"viewBox=\"0 0 1000 1000\"";
    let at = bytes
        .windows(needle.len())
        .position(|v| v == needle)
        .unwrap();
    bytes[at + 11] = b'1';
    bytes
}

fn line_units(bytes: &[u8]) -> f32 {
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    let at = (0..count)
        .map(|i| 12 + i * 16)
        .find(|&i| &bytes[i..i + 4] == b"hhea")
        .unwrap();
    let offset = u32::from_be_bytes(bytes[at + 8..at + 12].try_into().unwrap()) as usize;
    let metric =
        |i| i16::from_be_bytes(bytes[offset + i..offset + i + 2].try_into().unwrap()) as f32;
    metric(4) - metric(6) + metric(8)
}

#[test]
fn explicit_context_preserves_legacy_run_and_nonuniform_direction_scale() {
    let bytes = source();
    let file = FontFile::from_buffer(&bytes).unwrap();
    let face = file.face(0).unwrap();
    for direction in [
        ShapingPolicy::LeftToRight,
        ShapingPolicy::RightToLeft,
        ShapingPolicy::TopToBottom,
    ] {
        let options = FontOptions::new(&face)
            .with_font_size(50.0)
            .with_font_stretch(FontStretch(1.5))
            .with_line_height(73.0);
        let engine = face
            .engine()
            .with_options(options)
            .with_shaping_policy(direction);
        let legacy = engine.shape("A\u{fe0f}A\u{fe0f}").unwrap();
        let context = engine.shape_with_svg_context("A\u{fe0f}A\u{fe0f}").unwrap();
        assert_eq!(format!("{legacy:?}"), format!("{:?}", context.run()));
        let scale_y = 50.0 / line_units(&bytes).max(1.0);
        let mut raw_count = 0;
        for (i, glyph) in context.run().glyphs.iter().enumerate() {
            for (j, layer) in glyph.glyph.layers.iter().enumerate() {
                let transform = context.svg_layer_transform(i, j);
                if let GlyphLayer::Svg(svg) = layer {
                    raw_count += 1;
                    assert_eq!(
                        transform,
                        Some([
                            scale_y * 1.5,
                            0.0,
                            0.0,
                            scale_y,
                            glyph.x + svg.offset_x,
                            glyph.y + svg.offset_y
                        ])
                    );
                    assert!(!svg.document.contains("data-fontcore"));
                    assert!(svg.document.contains("viewBox=\"0 1 1000 1000\""));
                } else {
                    assert_eq!(transform, None);
                }
            }
        }
        assert!(raw_count > 0);
        assert_eq!(context.svg_layer_transform(usize::MAX, 0), None);
        assert_eq!(context.svg_layer_transform(0, usize::MAX), None);
        assert_eq!(format!("{legacy:?}"), format!("{:?}", context.into_run()));
    }
}

#[test]
fn invalid_options_do_not_create_context() {
    let file = FontFile::from_buffer(&source()).unwrap();
    let face = file.face(0).unwrap();
    for size in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(face
            .engine()
            .with_font_size(size)
            .shape_with_svg_context("A")
            .is_err());
    }
}
