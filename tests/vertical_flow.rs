use fontcore::{FontFile, FontOptions, FontStretch, GlyphFlow};

const NO_METRICS: &[u8] = include_bytes!("fixtures/vertical-no-metrics.ttf");
const METRICS: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");
const HEADER_ONLY: &[u8] = include_bytes!("fixtures/vertical-header-only.ttf");

#[test]
fn vertical_flow_without_metrics_uses_synthesized_vertical_advances() {
    for bytes in [NO_METRICS, HEADER_ONLY] {
        let file = FontFile::from_buffer(bytes).unwrap();
        let face = file.current_face().unwrap();
        let engine = face.engine().with_font_size(100.0).with_vertical_flow();
        let run = engine.shape("AB A").unwrap();
        assert_eq!(run.glyphs.len(), 4);
        for (index, positioned) in run.glyphs.iter().enumerate() {
            assert_eq!(positioned.glyph.font.unwrap().flow, GlyphFlow::Vertical);
            assert_eq!(positioned.glyph.metrics.advance_x, 0.0);
            assert_eq!(positioned.glyph.metrics.advance_y, 100.0);
            assert_eq!((positioned.x, positioned.y), (0.0, index as f32 * 100.0));
        }
        assert_eq!(engine.measure("AB A").unwrap(), 400.0);
        assert_eq!(
            engine
                .measure_layout("AB A")
                .unwrap()
                .line_bounds
                .unwrap()
                .max_y,
            400.0
        );
        assert!(engine.render_svg("AB A").unwrap().contains("<svg"));
    }
}

#[test]
fn vertical_flow_preserves_font_vertical_metrics_and_horizontal_flow() {
    let file = FontFile::from_buffer(METRICS).unwrap();
    let face = file.current_face().unwrap();
    let vertical = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = vertical.shape("AB").unwrap();
    let (a, b) = if cfg!(feature = "layout") {
        (120.0, 80.0)
    } else {
        (100.0, 100.0)
    };
    assert_eq!(run.glyphs[0].glyph.metrics.advance_y, a);
    assert_eq!(run.glyphs[1].glyph.metrics.advance_y, b);
    assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (0.0, a));
    assert_eq!(vertical.measure("AB").unwrap(), 200.0);
    for bytes in [NO_METRICS, HEADER_ONLY, METRICS] {
        let file = FontFile::from_buffer(bytes).unwrap();
        let face = file.current_face().unwrap();
        let run = face.engine().with_font_size(100.0).shape("AB").unwrap();
        assert_eq!(
            run.glyphs[0].glyph.font.unwrap().flow,
            GlyphFlow::Horizontal
        );
        assert_eq!(run.glyphs[0].glyph.metrics.advance_x, 60.0);
        assert_eq!(run.glyphs[1].glyph.metrics.advance_x, 100.0);
        assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (60.0, 0.0));
    }
}

#[test]
fn synthesized_vertical_advance_scales_once_and_columns_use_line_height() {
    let file = FontFile::from_buffer(NO_METRICS).unwrap();
    let face = file.current_face().unwrap();
    let options = FontOptions::new(&face)
        .with_font_size(50.0)
        .with_font_stretch(FontStretch(2.0))
        .with_line_height(75.0)
        .with_vertical_flow();
    let engine = face.engine().with_options(options);
    let run = engine.shape("AB\nA\tB").unwrap();
    let positions: Vec<_> = run.glyphs.iter().map(|g| (g.x, g.y)).collect();
    assert_eq!(
        positions,
        [(0.0, 0.0), (0.0, 50.0), (-75.0, 0.0), (-75.0, 350.0)]
    );
    assert!(run.glyphs.iter().all(|g| g.glyph.metrics.advance_y == 50.0));
}

#[test]
fn vertical_metrics_follow_the_selected_collection_face() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-mixed.ttc")).unwrap();
    assert_eq!(file.face_count(), 6);
    let with_metrics = if cfg!(feature = "layout") {
        120.0
    } else {
        100.0
    };
    for (index, expected) in [(0, 100.0), (1, with_metrics)] {
        let face = file.face(index).unwrap();
        let engine = face.engine().with_font_size(100.0).with_vertical_flow();
        let run = engine.shape("AA").unwrap();
        assert_eq!(run.glyphs[0].glyph.metrics.advance_y, expected);
        assert_eq!((run.glyphs[1].x, run.glyphs[1].y), (0.0, expected));
    }
}

#[test]
fn vertical_fallback_uses_horizontal_extent_without_line_gap() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-mixed.ttc")).unwrap();
    let face = file.face(2).unwrap();
    // 1000-unit extent / 1200-unit line height * 120px = 100px, not 120px.
    let engine = face.engine().with_font_size(120.0).with_vertical_flow();
    assert_eq!(
        engine.shape("A").unwrap().glyphs[0].glyph.metrics.advance_y,
        100.0
    );
}

#[test]
fn vertical_fallback_uses_units_per_em_for_nonpositive_extents() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-mixed.ttc")).unwrap();
    for index in [3, 4] {
        let face = file.face(index).unwrap();
        // Both fixtures retain a 1000-unit line height via lineGap. Their
        // zero/negative ascender-descender uses 1000 UPEM at the same scale.
        let engine = face.engine().with_font_size(100.0).with_vertical_flow();
        assert_eq!(
            engine.shape("A").unwrap().glyphs[0].glyph.metrics.advance_y,
            100.0
        );
    }
}

#[test]
fn explicit_zero_vertical_advance_is_not_replaced_by_fallback() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-mixed.ttc")).unwrap();
    let face = file.face(5).unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let run = engine.shape("AB").unwrap();
    let expected = if cfg!(feature = "layout") { 0.0 } else { 100.0 };
    assert_eq!(run.glyphs[0].glyph.metrics.advance_y, expected);
    assert_eq!(run.glyphs[1].y, expected);
}

#[test]
fn vertical_fallback_preserves_zero_width_marks_without_attachment_tables() {
    for bytes in [NO_METRICS, HEADER_ONLY] {
        let file = FontFile::from_buffer(bytes).unwrap();
        let face = file.current_face().unwrap();
        let engine = face.engine().with_font_size(100.0).with_vertical_flow();
        let run = engine.shape("A\u{0301}\u{200b}B").unwrap();
        assert_eq!(run.glyphs.len(), 4);
        assert_eq!(run.glyphs[1].glyph.metrics.advance_y, 0.0);
        assert_eq!(run.glyphs[2].glyph.metrics.advance_y, 0.0);
        assert_eq!(run.glyphs[3].y, 100.0);
        assert_eq!(engine.measure("A\u{0301}\u{200b}B").unwrap(), 200.0);
    }
}
