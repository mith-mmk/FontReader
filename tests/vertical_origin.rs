use fontcore::{FontFile, FontOptions, FontStretch, GlyphBounds, GlyphLayer};

const METRICS: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");
const NO_METRICS: &[u8] = include_bytes!("fixtures/vertical-no-metrics.ttf");

fn bounds(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> GlyphBounds {
    GlyphBounds {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

#[test]
fn truetype_vertical_origins_use_glyph_top_and_top_side_bearing() {
    let file = FontFile::from_buffer(METRICS).unwrap();
    let face = file.current_face().unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = engine.shape("AB\u{3001}\u{300c}").unwrap();
    let expected = if cfg!(feature = "layout") {
        [
            bounds(-25.0, 10.0, 15.0, 80.0),
            bounds(-40.0, 5.0, 30.0, 65.0),
            bounds(10.0, 20.0, 30.0, 40.0),
            bounds(-40.0, 20.0, -20.0, 100.0),
        ]
    } else {
        [
            bounds(-25.0, 15.0, 15.0, 85.0),
            bounds(-40.0, 20.0, 30.0, 80.0),
            bounds(10.0, 40.0, 30.0, 60.0),
            bounds(-40.0, 10.0, -20.0, 90.0),
        ]
    };
    for (glyph, expected) in run.glyphs.iter().zip(expected) {
        assert_eq!(glyph.glyph.metrics.bounds, Some(expected));
        assert_eq!(glyph.x, 0.0, "logical column origin must remain unchanged");
    }
    let measured = engine.measure_layout("AB\u{3001}\u{300c}").unwrap();
    assert_eq!(measured.inline_advance, 400.0);
    assert_eq!(measured.line_bounds.unwrap(), bounds(0.0, 0.0, 0.0, 400.0));
    if cfg!(feature = "layout") {
        assert_eq!(measured.ink_bounds, Some(bounds(-40.0, 10.0, 30.0, 400.0)));
    }
    let svg = engine.render_svg("AB\u{3001}\u{300c}").unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn missing_metrics_center_truetype_outlines_in_the_synthesized_cell() {
    let file = FontFile::from_buffer(NO_METRICS).unwrap();
    let face = file.current_face().unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = engine.shape("AB").unwrap();
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds,
        Some(bounds(-25.0, 15.0, 15.0, 85.0))
    );
    assert_eq!(
        run.glyphs[1].glyph.metrics.bounds,
        Some(bounds(-40.0, 20.0, 30.0, 80.0))
    );
    assert_eq!(run.glyphs[1].y, 100.0);
    let horizontal = face.engine().with_font_size(100.0).shape("A").unwrap();
    assert_eq!(
        horizontal.glyphs[0].glyph.metrics.bounds,
        Some(bounds(5.0, -70.0, 45.0, 0.0))
    );
}

#[test]
fn truetype_vertical_origin_scales_once_with_font_size_and_stretch() {
    let file = FontFile::from_buffer(METRICS).unwrap();
    let face = file.current_face().unwrap();
    let options = FontOptions::new(&face)
        .with_font_size(200.0)
        .with_font_stretch(FontStretch(1.5))
        .with_vertical_flow();
    let run = face.engine().with_options(options).shape("A").unwrap();
    let y = if cfg!(feature = "layout") {
        (20.0, 160.0)
    } else {
        (30.0, 170.0)
    };
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds,
        Some(bounds(-75.0, y.0, 45.0, y.1))
    );
    let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
        panic!("expected path")
    };
    assert_eq!(path.offset_x, -90.0);
}

#[cfg(feature = "layout")]
#[test]
fn vertical_mark_attachments_share_the_base_origin_translation() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin-cases.ttc")).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = engine.shape("A\u{0301}\u{0301}B").unwrap();
    assert_eq!(run.glyphs.len(), 4);
    for index in [1, 2] {
        let positioned = &run.glyphs[index];
        let GlyphLayer::Path(path) = &positioned.glyph.layers[0] else {
            panic!("expected path")
        };
        assert_eq!((path.offset_x, path.offset_y), (-30.0, 80.0));
        assert_eq!(positioned.glyph.metrics.advance_y, 0.0);
    }
    assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (20.0, 0.0));
    assert_eq!((run.glyphs[2].x, run.glyphs[2].y), (20.0, -10.0));
    assert_eq!(
        run.glyphs[1].glyph.metrics.bounds,
        Some(bounds(-20.0, 0.0, -10.0, 10.0))
    );
    assert_eq!(run.glyphs[3].y, 120.0);
}

#[cfg(feature = "layout")]
#[test]
fn vertical_origin_uses_the_substituted_punctuation_glyph() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin-cases.ttc")).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = engine.shape("\u{3001}").unwrap();
    // vert substitutes comma with B, whose yMax=500, tsb=50 and width=1000.
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds,
        Some(bounds(-40.0, 5.0, 30.0, 65.0))
    );
    assert_eq!(run.glyphs[0].glyph.metrics.advance_y, 80.0);
}

#[cfg(feature = "layout")]
#[test]
fn negative_top_side_bearings_preserve_ink_above_the_vertical_origin() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin-cases.ttc")).unwrap();
    let face = file.face(1).unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = engine.shape("B").unwrap();
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds,
        Some(bounds(-40.0, -15.0, 30.0, 45.0))
    );
    assert_eq!(
        engine.measure_layout("B").unwrap().ink_bounds,
        Some(bounds(-40.0, -15.0, 30.0, 45.0))
    );
    assert_eq!(engine.measure("B").unwrap(), 80.0);
}

#[cfg(feature = "layout")]
#[test]
fn composite_glyph_origins_use_composite_bounds_and_ignore_truetype_vorg() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin-cases.ttc")).unwrap();
    let face = file.face(2).unwrap();
    let run = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("B")
        .unwrap();
    // B contains A translated (100,200): yMax=900, tsb=50, origin_y=950.
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds,
        Some(bounds(-35.0, 5.0, 5.0, 75.0))
    );
}

#[test]
fn static_colr_origins_use_empty_base_metrics() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin-cases.ttc")).unwrap();
    let face = file.face(3).unwrap();
    let run = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("\u{3001}")
        .unwrap();
    assert_eq!(run.glyphs[0].glyph.layers.len(), 2);
    for layer in &run.glyphs[0].glyph.layers {
        let GlyphLayer::Path(path) = layer else {
            panic!("expected path")
        };
        assert_eq!(
            (path.offset_x, path.offset_y),
            if cfg!(feature = "layout") {
                (-30.0, 80.0)
            } else {
                (0.0, 0.0)
            }
        );
    }
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds,
        Some(if cfg!(feature = "layout") {
            bounds(-25.0, 10.0, 50.0, 90.0)
        } else {
            bounds(5.0, -70.0, 80.0, 10.0)
        })
    );
}

#[cfg(feature = "layout")]
#[test]
fn woff_and_sfnt_use_the_same_vertical_origins() {
    let sfnt = FontFile::from_buffer(METRICS).unwrap();
    let woff = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin.woff")).unwrap();
    let sfnt_face = sfnt.current_face().unwrap();
    let woff_face = woff.current_face().unwrap();
    let sfnt_run = sfnt_face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("AB")
        .unwrap();
    let woff_run = woff_face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("AB")
        .unwrap();
    for (a, b) in sfnt_run.glyphs.iter().zip(&woff_run.glyphs) {
        assert_eq!(a.glyph.metrics.bounds, b.glyph.metrics.bounds);
        assert_eq!((a.x, a.y), (b.x, b.y));
    }
}

#[cfg(feature = "svg-fonts")]
#[test]
fn pathified_svg_origins_remain_unchanged() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin-cases.ttc")).unwrap();
    let face = file.face(4).unwrap();
    let run = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("A\u{fe0f}")
        .unwrap();
    let GlyphLayer::Path(path) = &run.glyphs[0].glyph.layers[0] else {
        panic!("expected SVG converted to path")
    };
    assert_eq!((path.offset_x, path.offset_y), (0.0, 0.0));
}

#[test]
fn raster_origins_remain_unchanged() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin-cases.ttc")).unwrap();
    let face = file.face(5).unwrap();
    let run = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("A\u{fe0f}")
        .unwrap();
    let GlyphLayer::Raster(raster) = &run.glyphs[0].glyph.layers[0] else {
        panic!("expected sbix raster")
    };
    assert_eq!((raster.offset_x, raster.offset_y), (3.0, 4.0));
}

#[cfg(feature = "layout")]
#[test]
fn varied_outlines_and_phantom_metrics_resolve_vertical_origin_together() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-origin-cases.ttc")).unwrap();
    let face = file.face(6).unwrap();
    let run = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .with_variation("wght", 900.0)
        .shape("A")
        .unwrap();
    // Outline moves up 100 units while the top phantom stays at y=800.
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds,
        Some(bounds(-25.0, 0.0, 15.0, 70.0))
    );
    assert_eq!(run.glyphs[0].glyph.metrics.advance_y, 120.0);
}

#[test]
fn empty_and_zero_stretch_vertical_glyphs_stay_finite() {
    let file = FontFile::from_buffer(NO_METRICS).unwrap();
    let face = file.current_face().unwrap();
    let options = FontOptions::new(&face)
        .with_font_size(100.0)
        .with_font_stretch(FontStretch(0.0))
        .with_vertical_flow();
    let run = face.engine().with_options(options).shape(" A").unwrap();
    assert_eq!(run.glyphs[0].glyph.metrics.bounds, None);
    assert_eq!(
        run.glyphs[1].glyph.metrics.bounds,
        Some(bounds(0.0, 15.0, 0.0, 85.0))
    );
    assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (0.0, 100.0));
}

#[cfg(feature = "layout")]
#[test]
fn vertical_svg_and_multicolumn_measure_share_translated_bounds() {
    let file = FontFile::from_buffer(METRICS).unwrap();
    let face = file.current_face().unwrap();
    let engine = face
        .engine()
        .with_font_size(100.0)
        .with_line_height(150.0)
        .with_vertical_flow();
    let run = engine.shape("A\nB").unwrap();
    assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (-150.0, 0.0));
    let layout = engine.measure_layout("A\nB").unwrap();
    assert_eq!(layout.ink_bounds, Some(bounds(-190.0, 5.0, 15.0, 80.0)));
    assert_eq!(layout.block_extent, 150.0);
    let svg = engine.render_svg("A\nB").unwrap();
    assert!(svg.contains("viewBox=\"-194 1 213 83\""), "{svg}");
}
