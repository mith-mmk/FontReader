use fontcore::{FontFile, FontVariant, GlyphRun};

const FONTS: &[u8] = include_bytes!("fixtures/vertical-gsub-order.ttc");
const BASE: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");

fn assert_run(actual: &GlyphRun, expected: &GlyphRun) {
    assert_eq!(actual.glyphs.len(), expected.glyphs.len());
    for (a, b) in actual.glyphs.iter().zip(&expected.glyphs) {
        assert_eq!(a.glyph.metrics.bounds, b.glyph.metrics.bounds);
        assert_eq!(a.glyph.metrics.advance_x, b.glyph.metrics.advance_x);
        assert_eq!(a.glyph.metrics.advance_y, b.glyph.metrics.advance_y);
        assert_eq!((a.x, a.y), (b.x, b.y));
        // Includes outline coordinates and layer origin, not just metrics.
        assert_eq!(
            format!("{:?}", a.glyph.layers),
            format!("{:?}", b.glyph.layers)
        );
    }
}

#[cfg(feature = "layout")]
#[test]
fn vertical_substitution_follows_composition_and_selected_variant() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    for index in [0, 1, 4] {
        let face = file.face(index).unwrap();
        let engine = face
            .engine()
            .with_font_size(100.0)
            .with_vertical_flow()
            .with_font_variant(FontVariant::SmallCaps);
        let expected = engine.shape("\u{3001}\u{3001}").unwrap();
        assert_run(&engine.shape("AA").unwrap(), &expected);
        assert_eq!(engine.measure("AA").unwrap(), 200.0);
        assert_eq!(
            engine.render_svg("A").unwrap(),
            engine.render_svg("\u{3001}").unwrap()
        );
    }
}

#[cfg(feature = "layout")]
#[test]
fn vertical_substitution_follows_ligatures_and_preserves_boundaries() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(2).unwrap();
    let engine = face
        .engine()
        .with_font_size(100.0)
        .with_line_height(150.0)
        .with_vertical_flow();
    assert_run(
        &engine.shape("fifi").unwrap(),
        &engine.shape("\u{300c}\u{300c}").unwrap(),
    );
    assert_eq!(engine.measure("fi").unwrap(), 140.0);
    assert_run(
        &engine.shape("fi\nfi\tfi").unwrap(),
        &engine.shape("\u{300c}\n\u{300c}\t\u{300c}").unwrap(),
    );
    assert_eq!(engine.shape("f\ni").unwrap().glyphs.len(), 2);
    assert_eq!(engine.shape("f\ti").unwrap().glyphs.len(), 2);
}

#[cfg(feature = "layout")]
#[test]
fn locale_keeps_existing_precomposition_order_then_vertical_runs_last() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(3).unwrap();
    // Existing source pipeline localizes A -> B before ccmp B -> comma.
    // This test deliberately does not impose a new general GSUB ordering.
    let horizontal = face.engine().with_font_size(100.0).with_locale("ja-JP");
    assert_run(
        &horizontal.shape("A").unwrap(),
        &horizontal.shape("\u{3001}").unwrap(),
    );
    let vertical = horizontal.with_vertical_flow();
    assert_run(
        &vertical.shape("A").unwrap(),
        &vertical.shape("\u{300c}").unwrap(),
    );
}

#[cfg(feature = "layout")]
#[test]
fn final_vertical_glyph_supplies_metrics_origin_and_mark_anchors() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    let actual = engine.shape("A\u{0301}A").unwrap();
    assert_run(&actual, &engine.shape("\u{3001}\u{0301}\u{3001}").unwrap());
    assert_eq!(actual.glyphs[0].glyph.metrics.advance_y, 100.0);
    assert_eq!(actual.glyphs[2].y, 100.0);
}

#[cfg(feature = "layout")]
#[test]
fn variation_selector_mapping_precedes_final_vertical_stage() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(100.0).with_vertical_flow();
    assert_run(
        &engine.shape("A\u{fe00}").unwrap(),
        &engine.shape("\u{3001}").unwrap(),
    );
}

#[test]
fn horizontal_pipeline_does_not_apply_vertical_features() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.current_face().unwrap();
    let expected = base_face.engine().with_font_size(100.0);
    for index in [0, 1, 4] {
        let face = file.face(index).unwrap();
        let engine = face
            .engine()
            .with_font_size(100.0)
            .with_font_variant(FontVariant::SmallCaps);
        let result = if cfg!(feature = "layout") { "B" } else { "A" };
        assert_run(
            &engine.shape("A").unwrap(),
            &expected.shape(result).unwrap(),
        );
    }
    let face = file.face(2).unwrap();
    let result = if cfg!(feature = "layout") {
        "\u{3001}"
    } else {
        "AB"
    };
    assert_run(
        &face.engine().with_font_size(100.0).shape("fi").unwrap(),
        &expected.shape(result).unwrap(),
    );
}

#[cfg(not(feature = "layout"))]
#[test]
fn no_layout_ignores_all_gsub_stages() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.current_face().unwrap();
    let expected = base_face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow();
    for index in 0..5 {
        let face = file.face(index).unwrap();
        let engine = face
            .engine()
            .with_font_size(100.0)
            .with_vertical_flow()
            .with_locale("ja-JP")
            .with_font_variant(FontVariant::SmallCaps);
        assert_run(
            &engine.shape("Afi\nA\tA").unwrap(),
            &expected.shape("AAB\nA\tA").unwrap(),
        );
        assert_run(
            &engine.shape("A\u{fe00}").unwrap(),
            &expected.shape("B").unwrap(),
        );
    }
}

#[test]
fn family_fallback_shapes_the_selected_collection_face() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let face = file.face(2).unwrap();
    let mut family = fontcore::FontFamily::new("Order fallback");
    // The first face has no f/i cmap entries; the second must shape that run.
    family.add_font_face(base.current_face().unwrap());
    family.add_font_face(face.clone());
    let options = family.options().with_font_size(100.0).with_vertical_flow();
    let actual = fontcore::text2commands("fi", options).unwrap();
    let expected = face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow()
        .shape("fi")
        .unwrap();
    assert_run(&actual, &expected);
    if cfg!(feature = "layout") {
        assert_eq!(actual.glyphs.len(), 1);
        assert_eq!(actual.glyphs[0].glyph.metrics.advance_y, 140.0);
    }
}

#[cfg(feature = "raw")]
#[test]
fn raw_text_run_matches_high_level_selected_face() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    for index in 0..5 {
        let face = file.face(index).unwrap();
        let options = fontcore::FontOptions::from_font(face.raw_font())
            .with_font_size(100.0)
            .with_vertical_flow()
            .with_locale("ja-JP")
            .with_font_variant(FontVariant::SmallCaps);
        let actual = fontcore::text2commands("Afi\nA\tA\u{fe00}", options).unwrap();
        let expected = face
            .engine()
            .with_font_size(100.0)
            .with_vertical_flow()
            .with_locale("ja-JP")
            .with_font_variant(FontVariant::SmallCaps)
            .shape("Afi\nA\tA\u{fe00}")
            .unwrap();
        assert_run(&actual, &expected);
    }
}

#[cfg(feature = "raw")]
#[test]
fn raw_single_glyph_api_keeps_cmap_uvs_then_vertical_behavior() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(0).unwrap();
    let raw = face.raw_font();
    // GriphData exposes its glyph ID only through Debug. A is gid 1;
    // single-glyph retrieval must not start running text-level ccmp.
    let a = format!("{:?}", raw.get_glyph_with_uvs_axis('A', '\0', true));
    assert!(a.starts_with("GriphData { glyph_id: 1,"), "{a}");
    let replacement = if cfg!(feature = "layout") {
        '\u{3001}'
    } else {
        'B'
    };
    // Compare within this face: unrelated vhea compression metadata differs
    // from the base fixture and must not be part of the compatibility check.
    for (ch, vs) in [('B', '\0'), ('A', '\u{fe00}')] {
        assert_eq!(
            format!("{:?}", raw.get_glyph_with_uvs_axis(ch, vs, true)),
            format!("{:?}", raw.get_glyph_with_uvs_axis(replacement, '\0', true))
        );
    }
    assert_eq!(
        format!("{:?}", raw.get_glyph_with_uvs_axis('A', '\u{fe00}', false)),
        format!("{:?}", raw.get_glyph_from_id(2))
    );
}
