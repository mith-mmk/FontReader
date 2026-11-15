//! Original MIT fixtures, independent of installed fonts and Python at test time.
use fontcore::{FontFaceDescriptor, FontFamily, FontFile, GlyphRun, TextDirection};
#[cfg(feature = "layout")]
use fontcore::{FontOptions, FontStretch};
const FONT: &[u8] = include_bytes!("fixtures/tibetan-stack-substitution.ttc");
const CONTROL: usize = 30;

fn shape(index: usize, text: &str, locale: Option<&str>) -> GlyphRun {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(index).unwrap();
    let mut engine = face.engine().with_font_size(1000.0);
    if let Some(locale) = locale {
        engine = engine.with_locale(locale);
    }
    engine.shape(text).unwrap()
}
fn widths(run: &GlyphRun) -> Vec<f32> {
    run.glyphs
        .iter()
        .map(|g| {
            g.glyph
                .metrics
                .bounds
                .map(|b| b.max_x - b.min_x)
                .unwrap_or(0.0)
        })
        .collect()
}
fn same(a: &GlyphRun, b: &GlyphRun) {
    assert_eq!(widths(a), widths(b));
    for (a, b) in a.glyphs.iter().zip(&b.glyphs) {
        assert_eq!(
            (
                a.x,
                a.y,
                a.glyph.metrics.advance_x,
                a.glyph.metrics.advance_y
            ),
            (
                b.x,
                b.y,
                b.glyph.metrics.advance_x,
                b.glyph.metrics.advance_y
            )
        );
        assert_eq!(
            format!("{:?}", a.glyph.layers),
            format!("{:?}", b.glyph.layers)
        );
    }
}

#[test]
#[cfg(feature = "layout")]
fn preprocessing_precedes_ordered_presentation_with_explicit_tibt_for_all_locales() {
    for locale in [
        None,
        Some("bo"),
        Some("dz"),
        Some("tibt"),
        Some("en"),
        Some("bo-Tibt-CN"),
    ] {
        for text in ["\u{f40}\u{f90}", "\u{f40}\u{f71}", "\u{f40}\u{f74}"] {
            assert_eq!(
                widths(&shape(0, text, locale)),
                [170.0],
                "{text:?} {locale:?}"
            );
        }
        assert_eq!(
            widths(&shape(0, "\u{f66}\u{f92}\u{fb2}\u{f74}", locale)),
            [180.0]
        );
        assert_eq!(widths(&shape(0, "\u{f59}\u{f72}", locale)), [160.0, 190.0]);
        let run = shape(0, "\u{f40}\u{f72}", locale);
        assert_eq!(widths(&run), [160.0, 190.0]);
        assert_eq!(
            (
                run.glyphs[1].x - run.glyphs[0].x,
                run.glyphs[1].y - run.glyphs[0].y
            ),
            (250.0, -100.0)
        );
        assert_eq!(widths(&shape(2, "\u{f40}\u{f90}", locale)), [170.0]);
        assert_eq!(widths(&shape(4, "\u{f40}\u{f72}", locale)), [150.0, 130.0]);
    }
    assert_eq!(widths(&shape(3, "\u{f40}\u{f90}", Some("ja"))), [180.0]);
    assert_eq!(widths(&shape(3, "\u{f40}\u{f90}", Some("bo"))), [170.0]);
}

#[test]
fn unsupported_whole_clusters_and_cmap_aliases_keep_the_legacy_path() {
    for locale in [None, Some("bo"), Some("tibt"), Some("en")] {
        for text in [
            "A\u{f40}\u{f90}",
            "\u{f40}\u{f90}A",
            "\u{f40}\u{f90} \u{f40}\u{f90}",
            "\u{f40}\u{f90}\u{f0b}",
            "\u{f40}\u{f90}\u{f40}\u{f73}",
            "AB",
            "A\u{f90}",
            "\u{f40}\u{301}",
            "\u{f48}\u{f90}",
            "\u{f6d}\u{f90}",
            "\u{f43}\u{f90}",
            "\u{f69}\u{f90}",
            "\u{f40}\u{f93}",
            "\u{f40}\u{f98}",
            "\u{f40}\u{f73}",
            "\u{f40}\u{f75}",
            "\u{f40}\u{f76}",
            "\u{f40}\u{f77}",
            "\u{f40}\u{f78}",
            "\u{f40}\u{f79}",
            "\u{f40}\u{f81}",
            "\u{f40}\u{f71}\u{f72}",
            "\u{f40}\u{f72}\u{f71}",
            "\u{f40}\u{f72}\u{f90}",
            "\u{f40}\u{f90}\u{f72}\u{f72}",
            "\u{f40}\u{f90}\u{200d}",
            "\u{f40}\u{f90}\u{200c}",
            "\u{f40}\u{f90}\u{34f}",
            "\u{f40}\u{fe00}\u{f90}",
            "\u{f90}",
            "\u{f72}",
            "\u{f40} \u{f90}",
            "\u{f40}\t\u{f90}",
            "\u{f40}\n\u{f90}",
        ] {
            same(&shape(0, text, locale), &shape(CONTROL, text, locale));
        }
        for index in [1, 5, 6, 7] {
            same(
                &shape(index, "\u{f40}\u{f90}", locale),
                &shape(index + CONTROL, "\u{f40}\u{f90}", locale),
            );
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn whole_run_context_is_retained_and_incomplete_stacks_roll_back() {
    assert_eq!(
        widths(&shape(11, "\u{f40}\u{f90}\u{f40}", None)),
        [170.0, 180.0]
    );
    assert_eq!(
        widths(&shape(11, "\u{f40}\u{f40}\u{f90}", None)),
        [150.0, 170.0]
    );
    for index in [0, 9, 10, 20] {
        for text in [
            "\u{f40}\u{f90}\u{f72}",
            "\u{f40}\u{f90}\u{f40}\u{f90}\u{f72}",
        ] {
            same(
                &shape(index, text, None),
                &shape(index + CONTROL, text, None),
            );
        }
    }
    for index in [13, 14, 16, 17, 18, 19, 20, 25, 26] {
        same(
            &shape(index, "\u{f40}\u{f90}", None),
            &shape(index + CONTROL, "\u{f40}\u{f90}", None),
        );
    }
    for index in [12, 15, 24, 27] {
        assert_eq!(widths(&shape(index, "\u{f40}\u{f90}", None)), [170.0]);
    }
    same(
        &shape(28, "\u{f40}\u{f90}\u{f40}", None),
        &shape(28 + CONTROL, "\u{f40}\u{f90}\u{f40}", None),
    );
    assert_eq!(widths(&shape(22, "\u{f40}\u{f72}", None)), [110.0, 190.0]);
}

#[test]
fn missing_input_or_substitution_glyph_falls_back_as_one_grapheme() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for primary in [7, 8] {
        let mut family = FontFamily::new("Primary");
        family.add_face(
            FontFaceDescriptor::new("Primary"),
            file.face(primary).unwrap(),
        );
        family.add_face(FontFaceDescriptor::new("Fallback"), file.face(0).unwrap());
        for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
            let options = family
                .options()
                .with_font_size(1000.0)
                .with_text_direction(direction);
            let text = "\u{f40}\u{f90}";
            let actual = family.text2glyph_run(text, options.clone()).unwrap();
            let expected = file
                .face(if primary == 8 && !cfg!(feature = "layout") {
                    8
                } else {
                    0
                })
                .unwrap()
                .engine()
                .with_options(options)
                .shape(text)
                .unwrap();
            same(&actual, &expected);
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn scale_stretch_and_vertical_routes_keep_stack_and_mark_geometry() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_options(
        FontOptions::new(&face)
            .with_font_size(100.0)
            .with_font_stretch(FontStretch(2.0)),
    );
    let run = engine.shape("\u{f40}\u{f72}").unwrap();
    assert_eq!(widths(&run), [32.0, 38.0]);
    assert_eq!(
        (
            run.glyphs[1].x - run.glyphs[0].x,
            run.glyphs[1].y - run.glyphs[0].y
        ),
        (50.0, -10.0)
    );
    let face = file.face(21).unwrap();
    let engine = face.engine().with_font_size(1000.0);
    let text = "\u{f40}\u{f90}";
    assert_eq!(widths(&engine.shape(text).unwrap()), [170.0]);
    assert_eq!(
        widths(&engine.clone().with_vertical_flow().shape(text).unwrap()),
        [180.0]
    );
    // Tibetan has Unicode R orientation. Both routes must select vrtr's
    // distinct stage outline (160 wide), then rotate it, not just form a stack.
    for run in [
        engine.shape_vertical_mixed_ltr(text).unwrap(),
        engine.shape_sideways_ltr(text).unwrap(),
    ] {
        assert_eq!(run.glyphs.len(), 1);
        let bounds = run.glyphs[0].glyph.metrics.bounds.unwrap();
        assert_eq!(
            (bounds.max_x - bounds.min_x, bounds.max_y - bounds.min_y),
            (400.0, 160.0)
        );
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn no_layout_ignores_all_new_substitutions() {
    for index in 0..CONTROL {
        for text in [
            "\u{f40}\u{f90}",
            "\u{f40}\u{f90}\u{f72}",
            "\u{f66}\u{f92}\u{fb2}\u{f74}",
        ] {
            same(
                &shape(index, text, None),
                &shape(index + CONTROL, text, None),
            );
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn family_probe_checks_legacy_output_before_grouping_with_unsupported_neighbors() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let primary = file.face(29).unwrap();
    assert_eq!(
        widths(
            &primary
                .engine()
                .with_font_size(1000.0)
                .shape("\u{f40}\u{f90}")
                .unwrap()
        ),
        [170.0]
    );
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), primary);
    family.add_face(FontFaceDescriptor::new("Fallback"), file.face(0).unwrap());
    let options = family.options().with_font_size(1000.0).with_locale("tibt");
    for text in [
        "\u{f40}\u{f90}",
        "\u{f40}\u{f90} ",
        "\u{f40}\u{f90}A",
        "\u{f40}\u{f90} \u{f40}\u{f90}",
    ] {
        let actual = family.text2glyph_run(text, options.clone()).unwrap();
        assert!(
            actual
                .glyphs
                .iter()
                .filter_map(|g| g.glyph.metrics.bounds)
                .all(|b| b.max_x - b.min_x != 100.0),
            "{text:?}"
        );
        assert_eq!(
            widths(&actual)[0],
            if text.contains('A') { 150.0 } else { 170.0 },
            "{text:?}"
        );
    }
}
