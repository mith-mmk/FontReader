use fontcore::{
    Command, FontFile, FontOptions, FontStretch, FontVariant, GlyphBounds, GlyphFlow, GlyphLayer,
    GlyphRun, ShapingPolicy,
};
use std::io::ErrorKind;
const FONT: &[u8] = include_bytes!("fixtures/sideways-ltr.ttc");
const BASE: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");

fn rotate_bounds(b: GlyphBounds) -> GlyphBounds {
    GlyphBounds {
        min_x: -b.max_y,
        min_y: b.min_x,
        max_x: -b.min_y,
        max_y: b.max_x,
    }
}
fn rotate_command(c: &Command) -> Command {
    let r = |(x, y): (f32, f32)| (-y, x);
    match *c {
        Command::MoveTo(x, y) => Command::MoveTo(-y, x),
        Command::Line(x, y) => Command::Line(-y, x),
        Command::Bezier(a, b) => Command::Bezier(r(a), r(b)),
        Command::CubicBezier(a, b, c) => Command::CubicBezier(r(a), r(b), r(c)),
        Command::Close => Command::Close,
    }
}
fn assert_rotation(actual: &GlyphRun, horizontal: &GlyphRun) {
    assert_eq!(actual.glyphs.len(), horizontal.glyphs.len());
    for (a, h) in actual.glyphs.iter().zip(&horizontal.glyphs) {
        assert_eq!((a.x, a.y), (-h.y, h.x));
        assert_eq!(
            (a.glyph.metrics.advance_x, a.glyph.metrics.advance_y),
            (-h.glyph.metrics.advance_y, h.glyph.metrics.advance_x)
        );
        assert_eq!(
            a.glyph.metrics.bounds,
            h.glyph.metrics.bounds.map(rotate_bounds)
        );
        assert_eq!(
            (a.glyph.metrics.bearing_x, a.glyph.metrics.bearing_y),
            (h.glyph.metrics.bearing_y, -h.glyph.metrics.bearing_x)
        );
        let af = a.glyph.font.unwrap();
        let hf = h.glyph.font.unwrap();
        assert_eq!(af.flow, GlyphFlow::Vertical);
        assert_eq!(
            (af.ascent, af.descent, af.line_gap),
            (hf.ascent, hf.descent, hf.line_gap)
        );
        assert_eq!(a.glyph.layers.len(), h.glyph.layers.len());
        for (al, hl) in a.glyph.layers.iter().zip(&h.glyph.layers) {
            let (GlyphLayer::Path(ap), GlyphLayer::Path(hp)) = (al, hl) else {
                panic!("outline expected")
            };
            assert_eq!((ap.offset_x, ap.offset_y), (-hp.offset_y, hp.offset_x));
            assert_eq!(ap.paint, hp.paint);
            assert_eq!(
                format!("{:?}", ap.commands),
                format!(
                    "{:?}",
                    hp.commands.iter().map(rotate_command).collect::<Vec<_>>()
                )
            );
        }
    }
}

#[test]
fn rotates_horizontal_metrics_positions_and_outlines_about_baseline() {
    let file = FontFile::from_buffer(BASE).unwrap();
    let face = file.current_face().unwrap();
    let engine = face.engine().with_font_size(100.0);
    let horizontal = engine.shape("AB A").unwrap();
    assert_rotation(&engine.shape_sideways_ltr("AB A").unwrap(), &horizontal);
    assert_eq!(engine.measure_sideways_ltr("AB A").unwrap(), 280.0);
    let h = engine.measure_layout("AB A").unwrap();
    let s = engine.measure_layout_sideways_ltr("AB A").unwrap();
    assert_eq!(s.inline_advance, h.inline_advance);
    assert_eq!(s.ink_bounds, h.ink_bounds.map(rotate_bounds));
    assert_eq!(s.line_bounds, h.line_bounds.map(rotate_bounds));
    assert_eq!(s.block_extent, h.block_extent);
    assert!(engine
        .render_svg_sideways_ltr("A")
        .unwrap()
        .contains("viewBox=\"-4 1 78 48\""));
    assert_eq!(engine.shape("AB A").unwrap().glyphs[1].x, 60.0);
}

#[test]
fn explicit_call_ignores_policy_without_mutating_engine() {
    let file = FontFile::from_buffer(BASE).unwrap();
    let face = file.current_face().unwrap();
    let horizontal = face.engine().with_font_size(100.0).shape("AB").unwrap();
    for policy in [
        ShapingPolicy::LeftToRight,
        ShapingPolicy::RightToLeft,
        ShapingPolicy::TopToBottom,
    ] {
        let e = face
            .engine()
            .with_font_size(100.0)
            .with_shaping_policy(policy);
        let before = format!("{:?}", e.shape("AB").unwrap());
        assert_rotation(&e.shape_sideways_ltr("AB").unwrap(), &horizontal);
        let ltr = face.engine().with_font_size(100.0);
        assert_eq!(
            e.measure_layout_sideways_ltr("AB").unwrap(),
            ltr.measure_layout_sideways_ltr("AB").unwrap()
        );
        assert_eq!(
            e.measure_sideways_ltr("AB").unwrap(),
            ltr.measure_sideways_ltr("AB").unwrap()
        );
        assert_eq!(
            e.render_svg_sideways_ltr("AB").unwrap(),
            ltr.render_svg_sideways_ltr("AB").unwrap()
        );
        assert_eq!(e.shaping_policy(), policy);
        assert_eq!(before, format!("{:?}", e.shape("AB").unwrap()));
    }
}

#[cfg(feature = "layout")]
#[test]
fn vrtr_runs_after_ordinary_substitution_before_final_metrics_and_marks() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for (index, text) in [(0, "A"), (1, "A"), (2, "A"), (3, "fi"), (4, "A")] {
        let face = file.face(index).unwrap();
        let e = face
            .engine()
            .with_font_size(100.0)
            .with_font_variant(FontVariant::SmallCaps)
            .with_locale("ja-JP");
        assert_rotation(
            &e.shape_sideways_ltr(text).unwrap(),
            &e.shape("\u{300c}").unwrap(),
        );
        let marked = format!("{text}\u{0301}{text}");
        assert_rotation(
            &e.shape_sideways_ltr(&marked).unwrap(),
            &e.shape("\u{300c}\u{0301}\u{300c}").unwrap(),
        );
        assert_eq!(e.measure_sideways_ltr(text).unwrap(), 90.0);
        let doubled = format!("{text}{text}");
        assert_rotation(
            &e.shape_sideways_ltr(&doubled).unwrap(),
            &e.shape("\u{300c}\u{300c}").unwrap(),
        );
        assert_eq!(e.measure_sideways_ltr(&doubled).unwrap(), 173.0);
    }
    let face = file.face(1).unwrap();
    let e = face.engine().with_font_size(100.0);
    assert_rotation(
        &e.shape_sideways_ltr("A\u{fe00}").unwrap(),
        &e.shape("\u{300c}").unwrap(),
    );
}

#[test]
fn upright_features_never_substitute_sideways_or_horizontal() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let face = file.face(5).unwrap();
    let base_face = base.current_face().unwrap();
    let e = face.engine().with_font_size(100.0);
    let expected = base_face
        .engine()
        .with_font_size(100.0)
        .shape("AB")
        .unwrap();
    assert_rotation(&e.shape_sideways_ltr("AB").unwrap(), &expected);
    assert_eq!(
        format!("{:?}", e.shape("AB").unwrap()),
        format!("{:?}", expected)
    );
}

#[cfg(not(feature = "layout"))]
#[test]
fn no_layout_ignores_gsub_but_keeps_uvs_and_rotation() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.current_face().unwrap();
    let h = base_face.engine().with_font_size(100.0);
    for index in 0..6 {
        let face = file.face(index).unwrap();
        let e = face
            .engine()
            .with_font_size(100.0)
            .with_font_variant(FontVariant::SmallCaps)
            .with_locale("ja-JP");
        assert_rotation(
            &e.shape_sideways_ltr("Afi").unwrap(),
            &h.shape("AAB").unwrap(),
        );
        assert_rotation(
            &e.shape_sideways_ltr("A\u{fe00}").unwrap(),
            &h.shape("B").unwrap(),
        );
    }
}

#[test]
fn empty_space_and_single_line_rejection_are_consistent() {
    let file = FontFile::from_buffer(BASE).unwrap();
    let face = file.current_face().unwrap();
    let e = face.engine().with_font_size(100.0);
    for text in ["", " ", "  ", "\u{200b}"] {
        assert_rotation(
            &e.shape_sideways_ltr(text).unwrap(),
            &e.shape(text).unwrap(),
        );
        assert_eq!(
            e.measure_sideways_ltr(text).unwrap(),
            e.measure(text).unwrap()
        );
        assert!(e.render_svg_sideways_ltr(text).unwrap().contains("<svg"));
    }
    for text in [
        "A\tB",
        "A\nB",
        "A\rB",
        "A\r\nB",
        "A\u{000b}B",
        "A\u{000c}B",
        "A\u{0085}B",
        "A\u{2028}B",
        "A\u{2029}B",
    ] {
        assert_eq!(
            e.shape_sideways_ltr(text).unwrap_err().kind(),
            ErrorKind::InvalidInput,
            "{text:?}"
        );
        assert_eq!(
            e.measure_layout_sideways_ltr(text).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            e.measure_sideways_ltr(text).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            e.render_svg_sideways_ltr(text).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }
}

#[test]
fn selected_color_layers_fail_atomically_without_poisoning_engine() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/colr-vertical-origin.ttc")).unwrap();
    let face = file.face(0).unwrap();
    let e = face.engine().with_font_size(100.0);
    for text in ["\u{3001}", "A\u{3001}A"] {
        assert_eq!(
            e.shape_sideways_ltr(text).unwrap_err().kind(),
            ErrorKind::Unsupported
        );
        assert_eq!(
            e.measure_layout_sideways_ltr(text).unwrap_err().kind(),
            ErrorKind::Unsupported
        );
        assert_eq!(
            e.measure_sideways_ltr(text).unwrap_err().kind(),
            ErrorKind::Unsupported
        );
        assert_eq!(
            e.render_svg_sideways_ltr(text).unwrap_err().kind(),
            ErrorKind::Unsupported
        );
    }
    assert_rotation(&e.shape_sideways_ltr("A").unwrap(), &e.shape("A").unwrap());
    assert!(e.render_svg("\u{3001}").unwrap().contains("#ff0000"));
    // A foreground-only COLR layer is a supported CurrentColor outline.
    assert_rotation(
        &e.shape_sideways_ltr("A\u{0301}").unwrap(),
        &e.shape("A\u{0301}").unwrap(),
    );
}

fn check_outline_fixture(bytes: &[u8], variable: bool) {
    let file = FontFile::from_buffer(bytes).unwrap();
    for index in 0..file.face_count() {
        let face = file.face(index).unwrap();
        for stretch in [0.0, 1.5] {
            let options = FontOptions::new(&face)
                .with_font_size(75.0)
                .with_font_stretch(FontStretch(stretch));
            let mut e = face.engine().with_options(options);
            if variable {
                e = e.with_variation("wght", 900.0);
            }
            assert_rotation(
                &e.shape_sideways_ltr("AB\u{0301}").unwrap(),
                &e.shape("AB\u{0301}").unwrap(),
            );
        }
    }
}
#[test]
fn selected_truetype_faces_size_stretch_and_variable_outlines() {
    check_outline_fixture(include_bytes!("fixtures/vertical-mixed.ttc"), false);
    check_outline_fixture(include_bytes!("fixtures/vertical-variations.ttc"), true);
}
#[cfg(feature = "cff")]
#[test]
fn cff_outlines_rotate_without_vertical_origins() {
    check_outline_fixture(include_bytes!("fixtures/cff-vertical-origin.ttc"), false);
}
#[cfg(feature = "cff2")]
#[test]
fn cff2_variable_outlines_rotate_without_vertical_origins() {
    check_outline_fixture(include_bytes!("fixtures/cff2-vertical-origin.ttc"), true);
}

#[cfg(feature = "layout")]
#[test]
fn sideways_line_bounds_include_cross_axis_gpos_advance_endpoint() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(6).unwrap();
    let e = face.engine().with_font_size(100.0);
    let horizontal = e.shape("AB").unwrap();
    assert_eq!(horizontal.glyphs[1].glyph.metrics.advance_y, 40.0);
    let rotated = e.shape_sideways_ltr("AB").unwrap();
    assert_rotation(&rotated, &horizontal);
    let layout = e.measure_layout_sideways_ltr("AB").unwrap();
    // Horizontal layout currently ignores yAdvance for line bounds. Sideways
    // explicitly includes the complete rotated vector, preserving that legacy API.
    assert_eq!(
        layout.line_bounds,
        Some(GlyphBounds {
            min_x: -40.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 160.0
        })
    );
    assert_eq!(layout.block_extent, 40.0);
    assert_eq!(layout.inline_advance, 160.0);
    assert_eq!(e.measure_layout("AB").unwrap().block_extent, 0.0);
}

#[cfg(feature = "layout")]
#[test]
fn filtered_vrtr_lookup_preserves_prior_substitution_without_upright_fallback() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(7).unwrap();
    let e = face.engine().with_font_size(100.0);
    assert_rotation(&e.shape_sideways_ltr("A").unwrap(), &e.shape("B").unwrap());
    assert_eq!(e.measure_sideways_ltr("A").unwrap(), 100.0);
}

fn assert_public_unsupported(e: &fontcore::FontEngine<'_>, text: &str) {
    assert_eq!(
        e.shape_sideways_ltr(text).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
    assert_eq!(
        e.measure_layout_sideways_ltr(text).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
    assert_eq!(
        e.measure_sideways_ltr(text).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
    assert_eq!(
        e.render_svg_sideways_ltr(text).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
}

#[test]
fn selected_sbix_is_unsupported_but_plain_outline_remains_usable() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(8).unwrap();
    let e = face.engine().with_font_size(100.0);
    let text = "A\u{fe0f}";
    let ordinary = e.shape(text).unwrap();
    assert!(matches!(
        ordinary.glyphs[0].glyph.layers[0],
        GlyphLayer::Raster(_)
    ));
    assert_public_unsupported(&e, text);
    assert_public_unsupported(&e, "BA\u{fe0f}B");
    assert_rotation(&e.shape_sideways_ltr("A").unwrap(), &e.shape("A").unwrap());
}

#[cfg(feature = "svg-fonts")]
#[test]
fn selected_raw_svg_is_unsupported_but_current_color_pathified_svg_rotates() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let raw_face = file.face(9).unwrap();
    let raw = raw_face.engine().with_font_size(100.0);
    let text = "A\u{fe0f}";
    assert!(raw.shape(text).unwrap().glyphs[0]
        .glyph
        .layers
        .iter()
        .any(|layer| matches!(layer, GlyphLayer::Svg(_))));
    assert_public_unsupported(&raw, text);
    assert_public_unsupported(&raw, "BA\u{fe0f}B");
    let path_face = file.face(10).unwrap();
    let path = path_face.engine().with_font_size(100.0);
    let h = path.shape(text).unwrap();
    let GlyphLayer::Path(p) = &h.glyphs[0].glyph.layers[0] else {
        panic!("pathified SVG expected")
    };
    assert_eq!(p.paint, fontcore::GlyphPaint::CurrentColor);
    assert_rotation(&path.shape_sideways_ltr(text).unwrap(), &h);
    assert_eq!(path.measure_sideways_ltr(text).unwrap(), 60.0);
    assert!(path
        .measure_layout_sideways_ltr(text)
        .unwrap()
        .ink_bounds
        .is_some());
    assert!(path
        .render_svg_sideways_ltr(text)
        .unwrap()
        .contains("<path"));
    assert_rotation(
        &raw.shape_sideways_ltr("A").unwrap(),
        &raw.shape("A").unwrap(),
    );
}
