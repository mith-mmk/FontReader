#![cfg(feature = "svg-fonts")]
use fontcore::*;

const PRIMARY: &[u8] = include_bytes!("fixtures/svg-layout-primary.ttf");
const FALLBACK: &[u8] = include_bytes!("fixtures/svg-layout-fallback.ttf");
const A: &str = "A\u{fe0f}";

fn svg_index(run: &GlyphRun, glyph: usize) -> usize {
    run.glyphs[glyph]
        .glyph
        .layers
        .iter()
        .position(|layer| matches!(layer, GlyphLayer::Svg(_)))
        .unwrap()
}

#[test]
fn family_svg_matrix_uses_the_selected_face_scale_and_final_origin() {
    let primary = load_font_from_buffer(PRIMARY).unwrap();
    let fallback = load_font_from_buffer(FALLBACK).unwrap();
    let mut family = FontFamily::new("SvgLayoutPrimary");
    family.add_font_face(primary);
    family.add_font_face(fallback);
    let text = "A\u{fe0f}😀A\u{fe0f}";
    let options = family.options().with_font_size(64.0);
    let typed = family
        .shape_with_svg_context(text, options.clone())
        .unwrap();
    let mapped = family.shape_mapped(text, options).unwrap();
    for (index, ((scale, origin), face)) in [((0.5, 0.0), 0), ((0.25, 32.0), 1), ((0.5, 48.0), 0)]
        .into_iter()
        .enumerate()
    {
        let expected = Some([scale, 0.0, 0.0, scale, origin, 0.0]);
        assert_eq!(
            typed.svg_layer_transform(index, svg_index(typed.run(), index)),
            expected
        );
        assert_eq!(
            mapped.svg_layer_transform(index, svg_index(mapped.glyph_run(), index)),
            expected
        );
        assert_eq!(mapped.glyph_sources()[index].face_index(), face);
        assert_eq!(
            mapped.glyph_sources()[index].byte_range(),
            index * 4..(index + 1) * 4
        );
    }
    assert_eq!(typed.svg_layer_transform(4, 0), None);
}

#[test]
fn sideways_svg_matrix_encodes_rotation_while_legacy_rejects_raw_svg() {
    let font = load_font_from_buffer(PRIMARY).unwrap();
    let engine = font.engine().with_options(
        FontOptions::new(&font)
            .with_font_size(64.0)
            .with_font_stretch(FontStretch(1.5)),
    );
    let text = "A\u{fe0f}A\u{fe0f}";
    assert_eq!(
        engine.shape_sideways_ltr(text).unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
    let typed = engine.shape_sideways_ltr_with_svg_context(text).unwrap();
    assert_eq!(
        typed.svg_layer_transform(0, 0),
        Some([0.0, 0.75, -0.5, 0.0, 0.0, 0.0])
    );
    assert_eq!(
        typed.svg_layer_transform(1, 0),
        Some([0.0, 0.75, -0.5, 0.0, 0.0, 48.0])
    );
    assert_eq!(typed.run().glyphs[1].y, 48.0);
    assert_eq!(
        {
            let mapped = engine.shape_mapped(A).unwrap();
            mapped.svg_layer_transform(0, svg_index(mapped.glyph_run(), 0))
        },
        Some([0.75, 0.0, 0.0, 0.5, 0.0, 0.0])
    );
}

#[test]
fn mixed_svg_transform_matches_each_groups_placement_and_carets() {
    let font = load_font_from_buffer(PRIMARY).unwrap();
    let engine = font.engine().with_font_size(64.0);
    let text = "A\u{fe0f}一\u{fe0f}A\u{fe0f}";
    let layout = engine
        .shape_vertical_mixed_ltr_mapped_with_svg_context(text)
        .unwrap();
    let raw = engine
        .shape_vertical_mixed_ltr_with_svg_context(text)
        .unwrap();
    assert_eq!(layout.groups().len(), 3);
    for (idx, group) in layout.groups().iter().enumerate() {
        let [a, b, c, d, e, f] = group
            .shaped()
            .svg_layer_transform(0, svg_index(group.shaped().glyph_run(), 0))
            .unwrap();
        assert_eq!(group.is_sideways(), idx != 1);
        let (e, f) = group.transform_point(e, f);
        let expected = if group.is_sideways() {
            [-b, a, -d, c, e, f]
        } else {
            [a, b, c, d, e, f]
        };
        assert_eq!(layout.svg_layer_transform(idx, 0), Some(expected));
        assert_eq!(raw.svg_layer_transform(idx, 0), Some(expected));
    }
    let boundary = layout
        .caret_positions(TextPosition::new(4, CaretAffinity::Downstream))
        .unwrap();
    assert!(!boundary.is_empty());
    assert_eq!(boundary[0].group_index(), 1);
    assert_eq!(
        engine
            .shape_vertical_mixed_ltr_mapped(text)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::Unsupported
    );
}

#[test]
fn composite_mapped_context_carries_size_and_baseline_adjustments() {
    let face = load_font_from_buffer(PRIMARY).unwrap();
    let mut font = CompositeFont::new(vec![&face])
        .unwrap()
        .with_font_size(64.0);
    font.add_rule(
        CompositeFontRule::new(vec![UnicodeRange::new('一', '一').unwrap()], vec![&face])
            .unwrap()
            .with_size_multiplier(2.0)
            .unwrap()
            .with_baseline_shift(12.0)
            .unwrap(),
    );
    let text = "A\u{fe0f}一\u{fe0f}A\u{fe0f}";
    let mapped = font.shape_mapped_ltr(text).unwrap();
    let expected = Some([1.0, 0.0, 0.0, 1.0, 32.0, -12.0]);
    let typed = font.shape_ltr_with_svg_context(text).unwrap();
    assert_eq!(
        typed.svg_layer_transform(1, svg_index(typed.run(), 1)),
        expected
    );
    assert_eq!(
        mapped.svg_layer_transform(1, svg_index(mapped.glyph_run(), 1)),
        expected
    );
    assert_eq!(
        mapped.svg_layer_transform(2, svg_index(mapped.glyph_run(), 2)),
        Some([0.5, 0.0, 0.0, 0.5, 96.0, 0.0])
    );
}
