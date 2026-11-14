use fontcore::{FontFile, FontOptions, FontStretch};

const FONT: &[u8] = include_bytes!("fixtures/composite-gvar.ttc");
// Independent fontTools instancing, not values derived from fontcore output.
const CASES: &[(usize, u16, char, [i16; 8])] = include!("fixtures/composite-gvar-oracle.rs");

#[test]
fn component_domain_metrics_and_outlines_match_reference_instances() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for &(index, weight, glyph, expected) in CASES {
        let [advance, lsb, _, _, x_min, y_min, x_max, y_max] = expected.map(f32::from);
        let face = file.face(index).unwrap();
        let engine = face
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", f32::from(weight));
        let text = format!("{glyph}{glyph}");
        let run = engine.shape(&text).unwrap();
        let metrics = &run.glyphs[0].glyph.metrics;
        let context = format!("face {index}, weight {weight}, glyph {glyph}");
        assert_eq!(metrics.advance_x, advance, "{context}");
        assert_eq!(metrics.bearing_x, lsb, "{context}");
        let bounds = metrics.bounds.unwrap();
        assert_eq!(
            [bounds.min_x, -bounds.max_y, bounds.max_x, -bounds.min_y],
            [x_min, y_min, x_max, y_max],
            "{context}"
        );
        assert_eq!(run.glyphs[1].x, advance, "{context}");
        assert_eq!(
            engine.measure(&text).unwrap(),
            f64::from(advance * 2.0),
            "{context}"
        );
    }
}

#[cfg(feature = "layout")]
#[test]
fn composite_vertical_phantoms_match_reference_instances() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for &(index, weight, glyph, expected) in CASES {
        let [_, _, advance, tsb, _, y_min, _, y_max] = expected.map(f32::from);
        let face = file.face(index).unwrap();
        let engine = face
            .engine()
            .with_font_size(1000.0)
            .with_vertical_flow()
            .with_variation("wght", f32::from(weight));
        let text = format!("{glyph}{glyph}");
        let run = engine.shape(&text).unwrap();
        let metrics = &run.glyphs[0].glyph.metrics;
        let context = format!("face {index}, weight {weight}, glyph {glyph}");
        assert_eq!(metrics.advance_y, advance, "{context}");
        let bounds = metrics.bounds.unwrap();
        assert_eq!(bounds.min_y, tsb, "{context}");
        assert_eq!(bounds.max_y, tsb + y_max - y_min, "{context}");
        assert_eq!(run.glyphs[1].y, advance, "{context}");
        assert_eq!(
            engine.measure(&text).unwrap(),
            f64::from(advance * 2.0),
            "{context}"
        );
    }
}

#[test]
fn optional_hvar_mapping_and_mapped_zero_preserve_precedence() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (index, advance, lsb) in [(2, 700.0, 150.0), (3, 1000.0, 220.0)] {
        let run = file
            .face(index)
            .unwrap()
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", 900.0)
            .shape("BB")
            .unwrap();
        assert_eq!(run.glyphs[0].glyph.metrics.advance_x, advance);
        assert_eq!(run.glyphs[0].glyph.metrics.bearing_x, lsb);
        assert_eq!(run.glyphs[0].glyph.metrics.bounds.unwrap().min_x, 210.0);
        assert_eq!(run.glyphs[1].x, advance);
    }
}

#[cfg(feature = "layout")]
#[test]
fn optional_vvar_mapping_and_mapped_zero_preserve_precedence() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (index, advance, tsb) in [(2, 1200.0, 100.0), (3, 1600.0, 130.0)] {
        let run = file
            .face(index)
            .unwrap()
            .engine()
            .with_font_size(1000.0)
            .with_vertical_flow()
            .with_variation("wght", 900.0)
            .shape("BB")
            .unwrap();
        assert_eq!(run.glyphs[0].glyph.metrics.advance_y, advance);
        assert_eq!(run.glyphs[0].glyph.metrics.bounds.unwrap().min_y, tsb);
        assert_eq!(run.glyphs[1].y, advance);
    }
}

#[test]
fn composite_size_stretch_measure_and_svg_use_one_scale() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (index, advance, lsb) in [(0, 135.0, 33.0), (1, 142.5, 30.75)] {
        let face = file.face(index).unwrap();
        let options = FontOptions::new(&face)
            .with_font_size(100.0)
            .with_font_stretch(FontStretch(1.5));
        let engine = face
            .engine()
            .with_options(options)
            .with_variation("wght", 900.0);
        let run = engine.shape("BB").unwrap();
        assert!((run.glyphs[0].glyph.metrics.advance_x - advance).abs() < 0.001);
        assert!((run.glyphs[0].glyph.metrics.bearing_x - lsb).abs() < 0.001);
        assert!((run.glyphs[1].x - advance).abs() < 0.001);
        assert!((engine.measure("BB").unwrap() - f64::from(advance * 2.0)).abs() < 0.001);
        let measured = engine.measure_layout("BB").unwrap();
        let bounds = measured.ink_bounds.unwrap();
        assert!((bounds.min_x - 31.5).abs() < 0.001);
        assert!((bounds.max_x - (91.5 + advance)).abs() < 0.001);
        assert!((bounds.min_y + 81.0).abs() < 0.001);
        assert!((bounds.max_y + 11.0).abs() < 0.001);
        let svg = engine.render_svg("BB").unwrap();
        let view_box = svg
            .split("viewBox=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let values: Vec<f32> = view_box
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        for (actual, expected) in values.iter().zip([
            bounds.min_x - 4.0,
            bounds.min_y - 4.0,
            bounds.max_x - bounds.min_x + 8.0,
            bounds.max_y - bounds.min_y + 8.0,
        ]) {
            assert!((actual - expected).abs() < 0.001, "{svg}");
        }
    }
}

#[test]
fn unsupported_inherited_metrics_keep_table_fallback() {
    let file = FontFile::from_buffer(FONT).unwrap();
    // U has USE_MY_METRICS. Its own phantom deltas must not be applied as
    // if inheritance were implemented. The existing hmtx/HVAR fallback stays.
    for (index, advance) in [(0, 700.0), (2, 700.0), (3, 1000.0)] {
        let run = file
            .face(index)
            .unwrap()
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", 900.0)
            .shape("U")
            .unwrap();
        assert_eq!(run.glyphs[0].glyph.metrics.advance_x, advance);
        assert_eq!(run.glyphs[0].glyph.metrics.bearing_x, 150.0);
        assert_eq!(run.glyphs[0].glyph.metrics.bounds.unwrap().min_x, 210.0);
    }
}

#[test]
fn point_attachment_uses_nonempty_legacy_fallback_not_partial_variation() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let default = face.engine().with_font_size(1000.0).shape("P").unwrap();
    let varied = face
        .engine()
        .with_font_size(1000.0)
        .with_variation("wght", 900.0)
        .shape("P")
        .unwrap();
    // This is a fallback diagnostic, not a point-attachment correctness claim.
    assert!(!varied.glyphs[0].glyph.layers.is_empty());
    assert!(varied.glyphs[0].glyph.metrics.bounds.is_some());
    assert_eq!(
        varied.glyphs[0].glyph.metrics.advance_x,
        default.glyphs[0].glyph.metrics.advance_x
    );
    assert_eq!(
        varied.glyphs[0].glyph.metrics.bearing_x,
        default.glyphs[0].glyph.metrics.bearing_x
    );
    assert_eq!(
        varied.glyphs[0].glyph.metrics.bounds,
        default.glyphs[0].glyph.metrics.bounds
    );
}

#[cfg(feature = "raw")]
#[test]
fn raw_layout_getters_match_shaped_composite_metrics() {
    let mut font = fontcore::Font::get_font_from_buffer(FONT).unwrap();
    for &(index, weight, glyph, expected) in CASES {
        font.set_font(index).unwrap();
        let glyph_id = match glyph {
            'B' => 2,
            'C' => 3,
            'D' => 4,
            'E' => 5,
            'S' => 6,
            'T' => 7,
            _ => unreachable!(),
        };
        let coordinates = [(f32::from(weight) - 400.0) / 500.0];
        let horizontal = font.get_horizontal_layout_with_coords(glyph_id, &coordinates);
        let vertical = font
            .get_vertical_layout_with_coords(glyph_id, &coordinates)
            .unwrap();
        assert_eq!(
            [
                horizontal.advance_width,
                horizontal.lsb,
                vertical.advance_height,
                vertical.tsb
            ],
            expected[..4]
                .iter()
                .copied()
                .map(isize::from)
                .collect::<Vec<_>>()
                .as_slice(),
            "face {index}, weight {weight}, glyph {glyph}"
        );
    }
}

#[cfg(feature = "layout")]
#[test]
fn unsupported_inherited_vertical_metrics_keep_table_fallback() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (index, advance) in [(0, 1200.0), (2, 1200.0), (3, 1600.0)] {
        let run = file
            .face(index)
            .unwrap()
            .engine()
            .with_font_size(1000.0)
            .with_vertical_flow()
            .with_variation("wght", 900.0)
            .shape("UU")
            .unwrap();
        assert_eq!(run.glyphs[0].glyph.metrics.advance_y, advance);
        assert_eq!(run.glyphs[0].glyph.metrics.bounds.unwrap().min_y, 100.0);
        assert_eq!(run.glyphs[1].y, advance);
    }
}

#[test]
fn nested_fractional_component_offsets_round_once() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let run = file
        .face(0)
        .unwrap()
        .engine()
        .with_font_size(1000.0)
        .with_variation("wght", 412.5)
        .shape("C")
        .unwrap();
    let metrics = &run.glyphs[0].glyph.metrics;
    assert_eq!(metrics.bearing_x, 201.0);
    assert_eq!(metrics.bounds.unwrap().min_x, 201.0);
    assert_eq!(metrics.bounds.unwrap().max_y, -23.0);
}
