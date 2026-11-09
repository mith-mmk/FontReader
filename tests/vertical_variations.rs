#![cfg(feature = "layout")]
use fontcore::FontFile;

const SINGLE: &[u8] = include_bytes!("fixtures/vertical-variations.ttf");
const COLLECTION: &[u8] = include_bytes!("fixtures/vertical-variations.ttc");

fn check(bytes: &[u8], index: usize, advance_delta: f32, tsb_delta: f32, height_delta: f32) {
    let file = FontFile::from_buffer(bytes).unwrap();
    let face = file.face(index).unwrap();
    for (weight, scalar) in [(400.0, 0.0), (650.0, 0.5), (900.0, 1.0)] {
        let engine = face
            .engine()
            .with_font_size(1000.0)
            .with_vertical_flow()
            .with_variation("wght", weight);
        let run = engine.shape("AA").unwrap();
        let advance = 1200.0 + advance_delta * scalar;
        let tsb = 100.0 + tsb_delta * scalar;
        for glyph in &run.glyphs {
            assert_eq!(
                glyph.glyph.metrics.advance_y, advance,
                "face {index}, weight {weight}"
            );
            let bounds = glyph.glyph.metrics.bounds.unwrap();
            assert_eq!(bounds.min_y, tsb, "face {index}, weight {weight}");
            assert_eq!(bounds.max_y, tsb + 700.0 + height_delta * scalar);
        }
        assert_eq!(run.glyphs[1].y, advance);
        assert_eq!(engine.measure("AA").unwrap(), f64::from(advance * 2.0));
        let horizontal = face
            .engine()
            .with_font_size(1000.0)
            .with_variation("wght", weight)
            .shape("AA")
            .unwrap();
        assert_eq!(horizontal.glyphs[0].glyph.metrics.advance_x, 600.0);
    }
}

#[test]
fn matching_vvar_and_gvar_apply_vertical_deltas_once() {
    check(SINGLE, 0, 200.0, 40.0, 100.0);
}

#[test]
fn selected_faces_choose_vertical_metric_sources_per_field() {
    // 0: gvar only; 1: both; 2: VVAR advance only, gvar supplies missing TSB.
    for index in 0..3 {
        check(COLLECTION, index, 200.0, 40.0, 100.0);
    }
    // Deliberately inconsistent diagnostic proves VVAR overrides, not adds.
    check(COLLECTION, 3, 300.0, 80.0, 100.0);
    // A TrueType glyph must ignore VVAR's CFF2-only vertical-origin mapping.
    check(COLLECTION, 4, 200.0, 40.0, 100.0);
    // VVAR still applies without a matching gvar tuple.
    check(COLLECTION, 5, 200.0, 40.0, 0.0);
    // Present zero deltas are authoritative; they are not missing mappings.
    check(COLLECTION, 6, 0.0, 0.0, 100.0);
}
