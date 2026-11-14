//! Explicit horizontal-in-vertical composition, without external fonts.
use fontcore::{
    Command, FontEngine, FontFile, FontOptions, FontStretch, GlyphBounds, GlyphFlow, GlyphLayer,
    GlyphRun, ShapingPolicy,
};
use std::io::ErrorKind;

const FONT: &[u8] = include_bytes!("fixtures/tate-chu-yoko.ttc");

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.0002, "{a} != {b}");
}

fn point(command: &Command) -> Vec<(f32, f32)> {
    match *command {
        Command::MoveTo(x, y) | Command::Line(x, y) => vec![(x, y)],
        Command::Bezier(a, b) => vec![a, b],
        Command::CubicBezier(a, b, c) => vec![a, b, c],
        Command::Close => vec![],
    }
}

fn check_composition(e: &FontEngine<'_>, text: &str) -> GlyphRun {
    check_composition_with_cell(e, text, e.options().font_size)
}

fn check_composition_with_cell(e: &FontEngine<'_>, text: &str, size: f32) -> GlyphRun {
    let horizontal = e.clone().with_left_to_right().shape(text).unwrap();
    let actual = e.shape_tate_chu_yoko_ltr(text).unwrap();
    if horizontal.glyphs.is_empty() {
        assert!(actual.glyphs.is_empty());
        return actual;
    }
    assert_eq!(actual.glyphs.len(), 1);
    let cell = &actual.glyphs[0];
    assert_eq!((cell.x, cell.y), (0.0, 0.0));
    assert_eq!(cell.glyph.metrics.advance_x, 0.0);
    assert_eq!(cell.glyph.metrics.advance_y, size);
    let fm = cell.glyph.font.unwrap();
    assert_eq!(fm.flow, GlyphFlow::Vertical);
    assert_eq!(
        (fm.ascent, fm.descent, fm.line_gap),
        (size / 2.0, size / 2.0, 0.0)
    );
    let width: f32 = horizontal
        .glyphs
        .iter()
        .map(|g| g.glyph.metrics.advance_x)
        .sum();
    let sx = if width > size { size / width } else { 1.0 };
    let dx = -width * sx / 2.0;
    let font = horizontal.glyphs[0].glyph.font.unwrap();
    let dy = size / 2.0 + (font.ascent - font.descent) / 2.0;
    let mut index = 0;
    let mut expected_bounds: Option<GlyphBounds> = None;
    for g in &horizontal.glyphs {
        if let Some(b) = g.glyph.metrics.bounds {
            let b = GlyphBounds {
                min_x: (g.x + b.min_x) * sx + dx,
                max_x: (g.x + b.max_x) * sx + dx,
                min_y: g.y + b.min_y + dy,
                max_y: g.y + b.max_y + dy,
            };
            expected_bounds = Some(match expected_bounds {
                None => b,
                Some(old) => GlyphBounds {
                    min_x: old.min_x.min(b.min_x),
                    max_x: old.max_x.max(b.max_x),
                    min_y: old.min_y.min(b.min_y),
                    max_y: old.max_y.max(b.max_y),
                },
            });
        }
        for layer in &g.glyph.layers {
            let GlyphLayer::Path(src) = layer else {
                panic!("source path")
            };
            let GlyphLayer::Path(dst) = &cell.glyph.layers[index] else {
                panic!("composed path")
            };
            index += 1;
            assert_eq!(src.paint, dst.paint);
            assert_eq!(src.fill_rule, dst.fill_rule);
            assert_eq!(src.commands.len(), dst.commands.len());
            for (a, b) in src.commands.iter().zip(&dst.commands) {
                assert_eq!(std::mem::discriminant(a), std::mem::discriminant(b));
                for ((x, y), (nx, ny)) in point(a).into_iter().zip(point(b)) {
                    close(nx + dst.offset_x, (x + src.offset_x + g.x) * sx + dx);
                    close(ny + dst.offset_y, y + src.offset_y + g.y + dy);
                }
            }
        }
    }
    assert_eq!(index, cell.glyph.layers.len());
    match (expected_bounds, cell.glyph.metrics.bounds) {
        (None, None) => (),
        (Some(a), Some(b)) => {
            close(a.min_x, b.min_x);
            close(a.min_y, b.min_y);
            close(a.max_x, b.max_x);
            close(a.max_y, b.max_y);
        }
        _ => panic!("bounds mismatch"),
    }
    let layout = e.measure_layout_tate_chu_yoko_ltr(text).unwrap();
    assert_eq!(layout.inline_advance, size as f64);
    assert_eq!(layout.block_extent, size as f64);
    assert_eq!(layout.ink_bounds, cell.glyph.metrics.bounds);
    assert_eq!(
        layout.line_bounds,
        Some(GlyphBounds {
            min_x: -size / 2.0,
            min_y: 0.0,
            max_x: size / 2.0,
            max_y: size,
        })
    );
    assert_eq!(e.measure_tate_chu_yoko_ltr(text).unwrap(), size as f64);
    actual
}

#[test]
fn digit_pair_compresses_without_rotation_and_single_digit_is_not_enlarged() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    let one = check_composition(&e, "1");
    let b = one.glyphs[0].glyph.metrics.bounds.unwrap();
    assert_eq!(
        b,
        GlyphBounds {
            min_x: -25.0,
            min_y: 10.0,
            max_x: 15.0,
            max_y: 80.0
        }
    );
    let two = check_composition(&e, "12");
    let b = two.glyphs[0].glyph.metrics.bounds.unwrap();
    close(
        b.min_x,
        if cfg!(feature = "layout") {
            -46.666668
        } else {
            -46.875
        },
    );
    close(
        b.max_x,
        if cfg!(feature = "layout") {
            36.666668
        } else {
            37.5
        },
    );
    close(b.min_y, 10.0);
    close(b.max_y, 90.0);
}

#[test]
fn selected_face_size_stretch_gap_and_policy_do_not_change_cell_contract() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for index in [0, 2] {
        let face = file.face(index).unwrap();
        for size in [40.0, 100.0] {
            for stretch in [0.0, 0.5, 1.0, 2.0] {
                for policy in [
                    ShapingPolicy::LeftToRight,
                    ShapingPolicy::RightToLeft,
                    ShapingPolicy::TopToBottom,
                ] {
                    let e = face
                        .engine()
                        .with_options(
                            FontOptions::new(&face)
                                .with_font_size(size)
                                .with_font_stretch(FontStretch(stretch))
                                .with_line_height(777.0),
                        )
                        .with_shaping_policy(policy);
                    let before = format!("{:?}", e.shape("12").unwrap());
                    check_composition_with_cell(
                        &e,
                        "12",
                        if index == 2 {
                            size / 1200.0 * 1000.0
                        } else {
                            size
                        },
                    );
                    assert_eq!(before, format!("{:?}", e.shape("12").unwrap()));
                    assert_eq!(e.shaping_policy(), policy);
                }
            }
        }
    }
}

#[test]
fn spaces_marks_empty_and_ignored_selectors_have_explicit_cell_semantics() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    for text in [" ", "  ", " 1 ", "\u{301}", "\u{200b}", "1\u{301}2"] {
        check_composition(&e, text);
    }
    let space = e.measure_layout_tate_chu_yoko_ltr(" ").unwrap();
    assert_eq!(space.ink_bounds, None);
    for text in ["", "\u{fe0f}"] {
        assert!(check_composition(&e, text).glyphs.is_empty());
        let m = e.measure_layout_tate_chu_yoko_ltr(text).unwrap();
        assert_eq!((m.inline_advance, m.block_extent), (0.0, 0.0));
        assert_eq!((m.ink_bounds, m.line_bounds), (None, None));
    }
}

#[cfg(feature = "layout")]
#[test]
fn ordinary_ligatures_survive_and_vertical_features_do_not_run() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    let ligature = check_composition(&e, "11");
    assert_eq!(
        format!("{:?}", ligature),
        format!("{:?}", e.shape_tate_chu_yoko_ltr("2").unwrap())
    );
    let h = e.shape("1").unwrap();
    assert_ne!(
        format!("{:?}", h),
        format!("{:?}", e.clone().with_vertical_flow().shape("1").unwrap())
    );
    check_composition(&e, "1");
}

#[cfg(feature = "layout")]
#[test]
fn negative_total_advance_is_rejected() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(1).unwrap();
    assert_errors(&face.engine(), "12", ErrorKind::InvalidInput);
}

fn assert_errors(e: &FontEngine<'_>, text: &str, kind: ErrorKind) {
    assert_eq!(e.shape_tate_chu_yoko_ltr(text).unwrap_err().kind(), kind);
    assert_eq!(e.measure_tate_chu_yoko_ltr(text).unwrap_err().kind(), kind);
    assert_eq!(
        e.measure_layout_tate_chu_yoko_ltr(text).unwrap_err().kind(),
        kind
    );
    assert_eq!(
        e.render_svg_tate_chu_yoko_ltr(text).unwrap_err().kind(),
        kind
    );
}

#[test]
fn controls_invalid_options_and_overflow_are_rejected_by_all_entrypoints() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    assert_errors(
        &face.engine().with_font_size(f32::MAX),
        "12222",
        ErrorKind::InvalidInput,
    );
    for control in [
        '\t', '\r', '\n', '\u{b}', '\u{c}', '\u{85}', '\u{2028}', '\u{2029}',
    ] {
        assert_errors(
            &face.engine(),
            &format!("1{control}2"),
            ErrorKind::InvalidInput,
        );
    }
    for size in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_errors(
            &face.engine().with_font_size(size),
            "",
            ErrorKind::InvalidInput,
        );
    }
    for stretch in [-1.0, f32::INFINITY, f32::NAN] {
        assert_errors(
            &face
                .engine()
                .with_options(FontOptions::new(&face).with_font_stretch(FontStretch(stretch))),
            "12",
            ErrorKind::InvalidInput,
        );
    }
    assert_errors(
        &face.engine().with_options(
            FontOptions::new(&face)
                .with_font_size(f32::MAX)
                .with_font_stretch(FontStretch(f32::MAX)),
        ),
        "12",
        ErrorKind::InvalidInput,
    );
}

#[test]
fn selected_raster_and_color_paints_fail_without_partial_output() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/sideways-ltr.ttc")).unwrap();
    let face = file.face(8).unwrap();
    assert_errors(&face.engine(), "BA\u{fe0f}B", ErrorKind::Unsupported);
    let file = FontFile::from_buffer(include_bytes!("fixtures/colr-vertical-origin.ttc")).unwrap();
    let face = file.face(0).unwrap();
    assert_errors(&face.engine(), "\u{3001}", ErrorKind::Unsupported);
}

#[test]
fn single_and_pair_gpos_placement_are_preserved_inside_the_cell() {
    for bytes in [
        include_bytes!("fixtures/gpos-single.ttf").as_slice(),
        include_bytes!("fixtures/pair-coordinates.ttc").as_slice(),
    ] {
        let file = FontFile::from_buffer(bytes).unwrap();
        let face = file.face(0).unwrap();
        check_composition(&face.engine().with_font_size(100.0), "A\u{301}B");
    }
}

#[cfg(feature = "cff")]
#[test]
fn cff_cubic_paths_compose_without_using_vorg() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/cff-vertical-origin.ttc")).unwrap();
    let face = file.face(0).unwrap();
    check_composition(&face.engine().with_font_size(100.0), "AB");
}

#[cfg(feature = "cff2")]
#[test]
fn cff2_variation_coordinates_reach_the_horizontal_composition() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/cff2-vertical-origin.ttc")).unwrap();
    let face = file.face(0).unwrap();
    for weight in [100.0, 650.0, 900.0] {
        check_composition(
            &face
                .engine()
                .with_font_size(100.0)
                .with_variation("wght", weight),
            "AB",
        );
    }
}

#[test]
fn true_type_variation_outlines_reach_the_composition() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/vertical-variations.ttf")).unwrap();
    let face = file.face(0).unwrap();
    let default = check_composition(
        &face
            .engine()
            .with_font_size(100.0)
            .with_variation("wght", 400.0),
        "AB",
    );
    let varied = check_composition(
        &face
            .engine()
            .with_font_size(100.0)
            .with_variation("wght", 900.0),
        "AB",
    );
    assert_ne!(format!("{:?}", default), format!("{:?}", varied));
}

#[cfg(feature = "svg-fonts")]
#[test]
fn raw_svg_is_rejected_and_pathified_foreground_svg_is_supported() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/sideways-ltr.ttc")).unwrap();
    let raw = file.face(9).unwrap();
    assert_errors(&raw.engine(), "A\u{fe0f}", ErrorKind::Unsupported);
    let path = file.face(10).unwrap();
    check_composition(&path.engine().with_font_size(100.0), "A\u{fe0f}");
}

#[test]
fn scaled_em_matches_fullwidth_neighbors_when_hhea_exceeds_units_per_em() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(3).unwrap();
    for size in [32.0, 128.0] {
        let e = face.engine().with_font_size(size);
        let cell = size / 1500.0 * 1000.0;
        check_composition_with_cell(&e, "12", cell);
        close(
            e.measure_tate_chu_yoko_ltr("12").unwrap() as f32,
            size * 2.0 / 3.0,
        );
        #[cfg(feature = "layout")]
        {
            let upright = e.clone().with_vertical_flow().shape("\u{300c}").unwrap();
            close(upright.glyphs[0].glyph.metrics.advance_y, cell);
        }
    }
}

#[test]
fn scaled_em_uses_supported_mvar_line_metrics_once() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/mixed-vertical-ltr.ttc")).unwrap();
    let face = file.face(13).unwrap();
    for (weight, height) in [(400.0, 1000.0), (650.0, 1150.0), (900.0, 1300.0)] {
        let e = face
            .engine()
            .with_font_size(100.0)
            .with_variation("wght", weight);
        check_composition_with_cell(&e, "AB", 100.0 / height * 1000.0);
    }
}
