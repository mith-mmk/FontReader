//! MIT synthetic contextual shaping regressions; no installed-font dependency.
use fontcore::{FontFile, GlyphRun};
const FONT: &[u8] = include_bytes!("fixtures/contextual-gsub.ttc");
const BASE: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");

fn same(actual: &GlyphRun, expected: &GlyphRun) {
    assert_eq!(actual.glyphs.len(), expected.glyphs.len());
    for (a, b) in actual.glyphs.iter().zip(&expected.glyphs) {
        assert_eq!(
            format!("{:?}", a.glyph.metrics),
            format!("{:?}", b.glyph.metrics)
        );
        assert_eq!((a.x, a.y), (b.x, b.y));
        assert_eq!(
            format!("{:?}", a.glyph.layers),
            format!("{:?}", b.glyph.layers)
        );
    }
}

#[test]
fn contextual_features_are_not_rtl_only() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.face(0).unwrap();
    let reference = base_face.engine();
    for i in 0..4 {
        let face = file.face(i).unwrap();
        let e = face.engine();
        same(
            &e.shape("ABAB").unwrap(),
            &reference
                .shape(if cfg!(feature = "layout") {
                    "A、A、"
                } else {
                    "ABAB"
                })
                .unwrap(),
        );
        same(
            &e.clone().with_right_to_left().shape("ABAB").unwrap(),
            &reference
                .clone()
                .with_right_to_left()
                .shape(if cfg!(feature = "layout") {
                    "A、A、"
                } else {
                    "ABAB"
                })
                .unwrap(),
        );
        same(
            &e.shape("A\nB\tAB").unwrap(),
            &reference
                .shape(if cfg!(feature = "layout") {
                    "A\nB\tA、"
                } else {
                    "A\nB\tAB"
                })
                .unwrap(),
        );
    }
}

#[test]
fn ivs_cleanup_respects_lookup_order_in_horizontal_and_vertical() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.face(0).unwrap();
    for index in [4, 5] {
        let face = file.face(index).unwrap();
        let mut e = face.engine();
        let mut reference = base_face.engine();
        if index == 5 {
            e = e.with_vertical_flow();
            reference = reference.with_vertical_flow();
        }
        same(
            &e.shape("日\u{e01e1}本").unwrap(),
            &reference
                .shape(if cfg!(feature = "layout") {
                    "、"
                } else {
                    "「B"
                })
                .unwrap(),
        );
        same(
            &e.shape("日\u{e01e1}").unwrap(),
            &reference
                .shape(if cfg!(feature = "layout") { "A" } else { "「" })
                .unwrap(),
        );
    }
}

#[test]
fn vertical_route_selects_only_one_feature() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(6).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.face(0).unwrap();
    let e = face.engine();
    let reference = base_face.engine();
    same(
        &e.clone().with_vertical_flow().shape("日本").unwrap(),
        &reference
            .clone()
            .with_vertical_flow()
            .shape(if cfg!(feature = "layout") {
                "A「"
            } else {
                "AB"
            })
            .unwrap(),
    );
    same(
        &e.shape_vertical_mixed_ltr("日本").unwrap(),
        &reference
            .with_vertical_flow()
            .shape(if cfg!(feature = "layout") {
                "A、"
            } else {
                "AB"
            })
            .unwrap(),
    );
}

#[test]
fn shared_lookup_runs_once_and_nested_multiple_keeps_all_outputs() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.face(0).unwrap();
    let reference = base_face.engine();
    let shared = file.face(7).unwrap();
    same(
        &shared.engine().shape("AA").unwrap(),
        &reference
            .shape(if cfg!(feature = "layout") { "BB" } else { "AA" })
            .unwrap(),
    );
    let expanded = file.face(8).unwrap();
    same(
        &expanded.engine().shape("ABAB").unwrap(),
        &reference
            .shape(if cfg!(feature = "layout") {
                "A、「A、「"
            } else {
                "ABAB"
            })
            .unwrap(),
    );
}

#[test]
fn sideways_context_only_uses_vrtr() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(9).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.face(0).unwrap();
    let e = face.engine();
    let reference = base_face.engine();
    same(&e.shape("AB").unwrap(), &reference.shape("AB").unwrap());
    same(
        &e.shape_sideways_ltr("AB").unwrap(),
        &reference
            .shape_sideways_ltr(if cfg!(feature = "layout") {
                "A、"
            } else {
                "AB"
            })
            .unwrap(),
    );
}

#[cfg(feature = "layout")]
#[test]
fn contextual_ligature_preserves_last_component_mark_anchor() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(10).unwrap();
    let run = face
        .engine()
        .with_font_size(1000.0)
        .shape("AB\u{301}")
        .unwrap();
    assert_eq!(run.glyphs.len(), 2);
    assert_eq!(run.glyphs[1].x, 700.0);
    assert_eq!(run.glyphs[1].y, 200.0);
}

#[test]
fn flagged_ligature_is_retained_when_gdef_proves_no_exclusions() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(11).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.face(0).unwrap();
    same(
        &face.engine().shape("AB").unwrap(),
        &base_face
            .engine()
            .shape(if cfg!(feature = "layout") {
                "、"
            } else {
                "AB"
            })
            .unwrap(),
    );
    // IgnoreMarks ligature skips the intervening mark and preserves it after the ligature.
    same(
        &face.engine().shape("A\u{301}B").unwrap(),
        &base_face
            .engine()
            .shape(if cfg!(feature = "layout") {
                "、\u{301}"
            } else {
                "A\u{301}B"
            })
            .unwrap(),
    );
}

#[test]
fn orientation_scope_stays_global_with_an_explicit_locale() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(12).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.face(0).unwrap();
    let e = face.engine().with_locale("ja-JP").with_vertical_flow();
    let reference = base_face.engine().with_vertical_flow();
    same(
        &e.shape("A").unwrap(),
        &reference
            .shape(if cfg!(feature = "layout") { "「" } else { "A" })
            .unwrap(),
    );
}
