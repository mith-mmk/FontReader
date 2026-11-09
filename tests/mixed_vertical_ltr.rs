//! Deterministic single-face mixed vertical tests; no installed-font dependency.
use fontcore::{
    FontEngine, FontFile, FontOptions, FontStretch, GlyphBounds, GlyphRun, ShapingPolicy,
};
use std::io::ErrorKind;

const FONT: &[u8] = include_bytes!("fixtures/mixed-vertical-ltr.ttc");
const BASE: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.0001, "{a} != {b}");
}

fn assert_run(actual: &GlyphRun, expected: &GlyphRun) {
    assert_eq!(actual.glyphs.len(), expected.glyphs.len());
    for (a, e) in actual.glyphs.iter().zip(&expected.glyphs) {
        close(a.x, e.x);
        close(a.y, e.y);
        assert_eq!(format!("{:?}", a.glyph), format!("{:?}", e.glyph));
    }
}

fn sideways(e: &FontEngine<'_>, text: &str) -> GlyphRun {
    let mut run = e.shape_sideways_ltr(text).unwrap();
    if let Some(font) = run.glyphs.first().and_then(|g| g.glyph.font) {
        for g in &mut run.glyphs {
            g.x -= (font.ascent - font.descent) / 2.0;
        }
    }
    run
}

fn joined(runs: Vec<GlyphRun>) -> GlyphRun {
    let mut glyphs = Vec::new();
    let (mut x, mut y) = (0.0, 0.0);
    for mut run in runs {
        let (dx, dy) = run.glyphs.iter().fold((0.0, 0.0), |(x, y), g| {
            (x + g.glyph.metrics.advance_x, y + g.glyph.metrics.advance_y)
        });
        for g in &mut run.glyphs {
            g.x += x;
            g.y += y;
        }
        glyphs.extend(run.glyphs);
        x += dx;
        y += dy;
    }
    GlyphRun::new(glyphs)
}

#[test]
fn han_av_han_keeps_horizontal_kern_and_joins_advance_not_center() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    let upright = e.clone().with_vertical_flow().shape("漢").unwrap();
    let actual = e.shape_vertical_mixed_ltr("漢AV漢").unwrap();
    assert_run(
        &actual,
        &joined(vec![upright.clone(), sideways(&e, "AV"), upright]),
    );
    close(actual.glyphs[1].x, -30.0);
    close(actual.glyphs[1].y, 100.0);
    close(actual.glyphs[3].x, 0.0);
    close(
        actual.glyphs[3].y,
        if cfg!(feature = "layout") {
            253.0
        } else {
            260.0
        },
    );
    close(
        e.measure_vertical_mixed_ltr("漢AV漢").unwrap() as f32,
        if cfg!(feature = "layout") {
            353.0
        } else {
            360.0
        },
    );
}

#[cfg(feature = "layout")]
#[test]
fn gpos_placement_is_not_carried_but_both_advance_components_are() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(1).unwrap();
    let e = face.engine().with_font_size(100.0);
    let run = e.shape_vertical_mixed_ltr("漢AV漢").unwrap();
    // Pair records: A=(20,100,-40,120), V=(-30,-200,60,-80), scale=.1.
    // Clockwise rotation turns summed YAdvance=4 into group advance_x=-4.
    close(run.glyphs[1].x, -20.0);
    close(run.glyphs[1].y, 102.0);
    close(run.glyphs[2].x, -62.0);
    close(run.glyphs[2].y, 153.0);
    close(run.glyphs[3].x, -4.0);
    close(run.glyphs[3].y, 262.0);
    let u = e.clone().with_vertical_flow().shape("漢").unwrap();
    assert_run(&run, &joined(vec![u.clone(), sideways(&e, "AV"), u]));
}

#[test]
fn first_scalar_controls_cluster_and_any_enclosing_mark_forces_upright() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    // U+302A is U, but must follow the R base within this EGC.
    assert_run(
        &e.shape_vertical_mixed_ltr("A\u{302a}V").unwrap(),
        &sideways(&e, "A\u{302a}V"),
    );
    // Both the enclosing keycap and a bare combining enclosing mark force U.
    for text in ["1\u{fe0f}\u{20e3}", "A\u{20e3}", "\u{20e3}"] {
        assert_run(
            &e.shape_vertical_mixed_ltr(text).unwrap(),
            &e.clone().with_vertical_flow().shape(text).unwrap(),
        );
    }
    assert_run(
        &e.shape_vertical_mixed_ltr("漢\u{301}").unwrap(),
        &e.clone().with_vertical_flow().shape("漢\u{301}").unwrap(),
    );
}

#[test]
fn egc17_not_egc18_musical_symbol_boundary_is_pinned() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    // U+1D127 changes GCB Other -> Extend in Unicode18, but this API uses EGC17.
    // It is U in VO18, so the two EGC17 clusters have different orientations.
    let run = e.shape_vertical_mixed_ltr("A\u{1d127}").unwrap();
    assert_run(
        &run,
        &joined(vec![
            sideways(&e, "A"),
            e.clone().with_vertical_flow().shape("\u{1d127}").unwrap(),
        ]),
    );
    close(run.glyphs[1].x, 0.0);
    close(run.glyphs[1].y, 60.0);
}

#[test]
fn tr_without_alternate_identity_and_vert_are_all_upright() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.current_face().unwrap();
    let vertical = base_face
        .engine()
        .with_font_size(100.0)
        .with_vertical_flow();
    for index in [2, 3, 4] {
        let face = file.face(index).unwrap();
        let e = face.engine().with_font_size(100.0);
        let expected = if cfg!(feature = "layout") && index == 2 {
            "B"
        } else {
            "「"
        };
        assert_run(
            &e.shape_vertical_mixed_ltr("「").unwrap(),
            &vertical.shape(expected).unwrap(),
        );
        // Neither vrt2's A nor vrtr's comma may replace the upright bracket.
        close(
            e.measure_vertical_mixed_ltr("「").unwrap() as f32,
            if cfg!(feature = "layout") && index == 2 {
                80.0
            } else {
                100.0
            },
        );
    }
}

#[cfg(feature = "layout")]
#[test]
fn ccmp_multiple_ligatures_and_marks_stay_within_orientation_groups() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let base_face = file.face(0).unwrap();
    let base = base_face.engine().with_font_size(100.0);
    for (index, input, expected, is_sideways) in [
        (5, "A", "「", true),
        (6, "A", "、「", true),
        (7, "fi", "、", true),
        (8, "漢", "「", false),
        (9, "漢漢", "「", false),
        (11, "漢", "、「", false),
    ] {
        let face = file.face(index).unwrap();
        let e = face.engine().with_font_size(100.0);
        for suffix in ["", "\u{301}"] {
            let input = format!("{input}{suffix}");
            let expected = format!("{expected}{suffix}");
            let expected = if is_sideways {
                sideways(&base, &expected)
            } else {
                base.clone().with_vertical_flow().shape(&expected).unwrap()
            };
            assert_run(&e.shape_vertical_mixed_ltr(&input).unwrap(), &expected);
        }
    }
}

#[cfg(feature = "layout")]
#[test]
fn orientation_boundaries_deliberately_block_ligatures_and_pair_positioning() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(12).unwrap();
    let e = face.engine().with_font_size(100.0);
    assert_eq!(e.shape("漢A").unwrap().glyphs.len(), 1);
    assert_eq!(e.shape_vertical_mixed_ltr("漢A").unwrap().glyphs.len(), 2);
    assert_eq!(e.shape("A漢").unwrap().glyphs.len(), 1);
    assert_eq!(e.shape_vertical_mixed_ltr("A漢").unwrap().glyphs.len(), 2);
}

#[test]
fn centered_from_scaled_horizontal_metrics_not_ink_gap_lineheight_or_stretch() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [0, 10] {
        let face = file.face(index).unwrap();
        for stretch in [0.0, 1.0, 1.5] {
            let e = face.engine().with_options(
                FontOptions::new(&face)
                    .with_font_size(120.0)
                    .with_line_height(777.0)
                    .with_font_stretch(FontStretch(stretch)),
            );
            let h = e.shape("AV").unwrap();
            let font = h.glyphs[0].glyph.font.unwrap();
            let expected_shift = -(font.ascent - font.descent) / 2.0;
            let run = e.shape_vertical_mixed_ltr("AV").unwrap();
            assert_run(&run, &sideways(&e, "AV"));
            close(run.glyphs[0].x, expected_shift);
            close(expected_shift, if index == 0 { -36.0 } else { -30.0 });
        }
    }
}

fn check_outline_faces(bytes: &[u8], variable: bool) {
    let file = FontFile::from_buffer(bytes).unwrap();
    for index in 0..file.face_count() {
        let face = file.face(index).unwrap();
        for stretch in [0.0, 1.5] {
            let mut e = face.engine().with_options(
                FontOptions::new(&face)
                    .with_font_size(75.0)
                    .with_font_stretch(FontStretch(stretch)),
            );
            if variable {
                e = e.with_variation("wght", 900.0);
            }
            let upright = e.clone().with_vertical_flow().shape("、\u{301}").unwrap();
            assert_run(
                &e.shape_vertical_mixed_ltr("、\u{301}AB、\u{301}").unwrap(),
                &joined(vec![upright.clone(), sideways(&e, "AB"), upright]),
            );
        }
    }
}

#[test]
fn selected_tt_faces_missing_metrics_and_supported_variations() {
    check_outline_faces(include_bytes!("fixtures/vertical-mixed.ttc"), false);
    check_outline_faces(include_bytes!("fixtures/vertical-variations.ttc"), true);
}
#[cfg(feature = "cff")]
#[test]
fn selected_cff_faces_preserve_existing_vertical_origins() {
    check_outline_faces(include_bytes!("fixtures/cff-vertical-origin.ttc"), false);
}
#[cfg(feature = "cff2")]
#[test]
fn selected_cff2_faces_and_supported_variations() {
    check_outline_faces(include_bytes!("fixtures/cff2-vertical-origin.ttc"), true);
}

#[test]
fn explicit_call_overrides_policy_without_mutation() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let expected = face.engine().with_font_size(100.0);
    for policy in [
        ShapingPolicy::LeftToRight,
        ShapingPolicy::RightToLeft,
        ShapingPolicy::TopToBottom,
    ] {
        let e = expected.clone().with_shaping_policy(policy);
        let before = format!("{:?}", e.shape("漢AV漢").unwrap());
        assert_run(
            &e.shape_vertical_mixed_ltr("漢AV漢").unwrap(),
            &expected.shape_vertical_mixed_ltr("漢AV漢").unwrap(),
        );
        assert_eq!(
            e.measure_layout_vertical_mixed_ltr("漢AV漢").unwrap(),
            expected
                .measure_layout_vertical_mixed_ltr("漢AV漢")
                .unwrap()
        );
        assert_eq!(
            e.render_svg_vertical_mixed_ltr("漢AV漢").unwrap(),
            expected.render_svg_vertical_mixed_ltr("漢AV漢").unwrap()
        );
        assert_eq!(e.shaping_policy(), policy);
        assert_eq!(before, format!("{:?}", e.shape("漢AV漢").unwrap()));
    }
}

fn assert_error(e: &FontEngine<'_>, text: &str, kind: ErrorKind) {
    assert_eq!(e.shape_vertical_mixed_ltr(text).unwrap_err().kind(), kind);
    assert_eq!(e.measure_vertical_mixed_ltr(text).unwrap_err().kind(), kind);
    assert_eq!(
        e.measure_layout_vertical_mixed_ltr(text)
            .unwrap_err()
            .kind(),
        kind
    );
    assert_eq!(
        e.render_svg_vertical_mixed_ltr(text).unwrap_err().kind(),
        kind
    );
}

#[test]
fn empty_space_orphan_selectors_controls_and_option_validation() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    for text in ["", " ", "  ", "\u{200b}", "\u{fe00}", "\u{fe0f}\u{301}"] {
        assert_run(
            &e.shape_vertical_mixed_ltr(text).unwrap(),
            &sideways(&e, text),
        );
        assert!(e
            .render_svg_vertical_mixed_ltr(text)
            .unwrap()
            .contains("<svg"));
    }
    let empty = e.measure_layout_vertical_mixed_ltr("").unwrap();
    assert_eq!(empty.ink_bounds, None);
    assert_eq!(empty.line_bounds, None);
    assert_eq!(empty.inline_advance, 0.0);
    assert_eq!(empty.block_extent, 0.0);
    for control in [
        '\t', '\r', '\n', '\u{b}', '\u{c}', '\u{85}', '\u{2028}', '\u{2029}',
    ] {
        assert_error(
            &e,
            &format!("漢A\u{200d}{control}B"),
            ErrorKind::InvalidInput,
        );
    }
    for text in ["", "\u{fe00}", "漢A"] {
        assert_error(
            &e.clone().with_font_size(f32::NAN),
            text,
            ErrorKind::InvalidInput,
        );
    }
}

#[test]
fn unsupported_upright_and_sideways_layers_fail_atomically_without_poisoning() {
    let color = FontFile::from_buffer(include_bytes!("fixtures/colr-vertical-origin.ttc")).unwrap();
    let face = color.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    for text in ["、", "A、A", "、A"] {
        assert_error(&e, text, ErrorKind::Unsupported);
    }
    assert!(e.shape_vertical_mixed_ltr("A").is_ok());
    assert!(e.render_svg("、").unwrap().contains("#ff0000"));
    let side = FontFile::from_buffer(include_bytes!("fixtures/sideways-ltr.ttc")).unwrap();
    let face = side.face(8).unwrap();
    let e = face.engine().with_font_size(100.0);
    assert_error(&e, "「A\u{fe0f}「", ErrorKind::Unsupported);
    assert!(e.shape_vertical_mixed_ltr("「A「").is_ok());
}

#[cfg(feature = "svg-fonts")]
#[test]
fn raw_svg_rejected_and_current_color_pathified_svg_accepted() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/sideways-ltr.ttc")).unwrap();
    let raw_face = file.face(9).unwrap();
    assert_error(&raw_face.engine(), "「A\u{fe0f}「", ErrorKind::Unsupported);
    let path_face = file.face(10).unwrap();
    assert!(path_face
        .engine()
        .shape_vertical_mixed_ltr("「A\u{fe0f}「")
        .is_ok());
}

fn include(bounds: &mut Option<GlyphBounds>, x: f32, y: f32) {
    if let Some(b) = bounds {
        b.min_x = b.min_x.min(x);
        b.min_y = b.min_y.min(y);
        b.max_x = b.max_x.max(x);
        b.max_y = b.max_y.max(y);
    } else {
        *bounds = Some(GlyphBounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        });
    }
}

#[test]
fn measurement_uses_final_geometry_and_full_vector_endpoints() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(1).unwrap();
    let e = face.engine().with_font_size(100.0);
    let text = "漢AV漢";
    let run = e.shape_vertical_mixed_ltr(text).unwrap();
    let (mut ink, mut line, mut advance) = (None, None, 0.0);
    for g in &run.glyphs {
        let m = g.glyph.metrics;
        advance += m.advance_y;
        include(&mut line, g.x, g.y);
        include(&mut line, g.x + m.advance_x, g.y + m.advance_y);
        if let Some(b) = m.bounds {
            include(&mut ink, g.x + b.min_x, g.y + b.min_y);
            include(&mut ink, g.x + b.max_x, g.y + b.max_y);
        }
    }
    let layout = e.measure_layout_vertical_mixed_ltr(text).unwrap();
    assert_eq!(layout.ink_bounds, ink);
    assert_eq!(layout.line_bounds, line);
    assert_eq!(layout.inline_advance, f64::from(advance.abs()));
    assert_eq!(
        layout.block_extent,
        f64::from(line.unwrap().max_x - line.unwrap().min_x)
    );
    assert_eq!(
        e.measure_vertical_mixed_ltr(text).unwrap(),
        layout.inline_advance
    );
    assert!(e
        .render_svg_vertical_mixed_ltr(text)
        .unwrap()
        .contains("<path"));
}

#[test]
fn sideways_center_uses_same_instance_mvar_horizontal_metrics() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(13).unwrap();
    for weight in [400.0, 900.0] {
        let e = face
            .engine()
            .with_font_size(100.0)
            .with_variation("wght", weight);
        let h = e.shape("AV").unwrap();
        let font = h.glyphs[0].glyph.font.unwrap();
        let expected = -(font.ascent - font.descent) / 2.0;
        close(
            expected,
            if weight == 400.0 {
                -30.0
            } else {
                -450.0 / 13.0
            },
        );
        let run = e.shape_vertical_mixed_ltr("漢AV漢").unwrap();
        close(run.glyphs[1].x, expected);
        close(run.glyphs[3].x, 0.0);
        let upright = e.clone().with_vertical_flow().shape("漢").unwrap();
        assert_run(
            &run,
            &joined(vec![upright.clone(), sideways(&e, "AV"), upright]),
        );
    }
}

#[test]
fn every_unicode18_enclosing_mark_forces_its_base_upright() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    for mark in [
        0x0488, 0x0489, 0x1ABE, 0x20DD, 0x20DE, 0x20DF, 0x20E0, 0x20E2, 0x20E3, 0x20E4, 0xA670,
        0xA671, 0xA672,
    ] {
        let text = format!("A{}", char::from_u32(mark).unwrap());
        assert_run(
            &e.shape_vertical_mixed_ltr(&text).unwrap(),
            &e.clone().with_vertical_flow().shape(&text).unwrap(),
        );
    }
}

#[cfg(feature = "layout")]
#[test]
fn upright_advances_join_sideways_without_carrying_upright_placement() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(14).unwrap();
    let e = face.engine().with_font_size(100.0);
    let upright = e.clone().with_vertical_flow().shape("漢「").unwrap();
    let actual = e.shape_vertical_mixed_ltr("漢「A").unwrap();
    assert_run(&actual, &joined(vec![upright, sideways(&e, "A")]));
    close(actual.glyphs[0].x, 2.0);
    close(actual.glyphs[0].y, -10.0);
    close(actual.glyphs[1].x, -7.0);
    close(actual.glyphs[1].y, 132.0);
    close(actual.glyphs[2].x, -28.0);
    close(actual.glyphs[2].y, 204.0);
}
