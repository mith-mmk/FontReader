use fontcore::FontFile;
const FONT: &[u8] = include_bytes!("fixtures/mark-input-filter.ttc");
const CASES: usize = 17;

#[test]
#[cfg(feature = "layout")]
fn mark_lookup_filters_only_current_input() {
    let file = FontFile::from_buffer(FONT).unwrap();
    // Original and Extension9 copies. Target IgnoreBase/IgnoreLigature flags
    // remain admitted. MarkAttachClassDef's legacy origin fallback is explicit.
    // Missing/invalid sets (11/12) retain includes_glyph rejection; HB15 instead
    // admits these missing-data fixtures. They are not parity assertions.
    for extension in [0, CASES] {
        for case in 0..CASES {
            for locale in [None, Some("en"), Some("ja"), Some("jpn")] {
                let face = file.face(extension + case).unwrap();
                let mut engine = face.engine().with_font_size(1000.0);
                if let Some(locale) = locale {
                    engine = engine.with_locale(locale);
                }
                let admitted = matches!(case, 0 | 1 | 2 | 5 | 7 | 9 | 13 | 14 | 16);
                for (text, anchor, cursor) in [
                    ("A\u{301}", (200.0, 0.0), 600.0),
                    ("AB\u{301}", (700.0, 200.0), 1000.0),
                    ("A\u{301}\u{301}", (200.0, -100.0), 600.0),
                ] {
                    let run = engine.shape(text).unwrap();
                    let mark = run.glyphs.last().unwrap();
                    let expected = if admitted {
                        anchor
                    } else if case == 6 || case == 15 {
                        (0.0, 0.0)
                    } else {
                        (cursor, 0.0)
                    };
                    assert_eq!(
                        (mark.x, mark.y),
                        expected,
                        "face {} case {} locale {:?} text {:?}",
                        extension + case,
                        case,
                        locale,
                        text
                    );
                    assert_eq!(mark.glyph.metrics.advance_x, 0.0);
                    assert_eq!(mark.glyph.metrics.advance_y, 0.0);
                }
                if matches!(case, 3 | 4 | 8 | 10 | 11 | 12) {
                    let denied_face = file.face(3).unwrap();
                    let expected = denied_face
                        .engine()
                        .with_font_size(1000.0)
                        .with_vertical_flow()
                        .shape("A\u{301}\u{301}")
                        .unwrap();
                    let actual = engine
                        .clone()
                        .with_vertical_flow()
                        .shape("A\u{301}\u{301}")
                        .unwrap();
                    for (a, b) in actual.glyphs.iter().zip(expected.glyphs.iter()) {
                        assert_eq!((a.x, a.y), (b.x, b.y));
                    }
                }
                // TTB is the same input-filter decision; untouched admitted
                // fixtures must match the unfiltered font in each orientation.
                if admitted && case != 13 && case != 14 {
                    let baseline = file.face(0).unwrap();
                    let expected = baseline
                        .engine()
                        .with_font_size(1000.0)
                        .with_vertical_flow()
                        .shape("A\u{301}\u{301}")
                        .unwrap();
                    let actual = engine
                        .with_vertical_flow()
                        .shape("A\u{301}\u{301}")
                        .unwrap();
                    for (a, b) in actual.glyphs.iter().zip(expected.glyphs.iter()) {
                        assert_eq!((a.x, a.y), (b.x, b.y));
                    }
                }
            }
        }
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn mark_lookup_filter_fixtures_do_not_change_no_layout() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for vertical in [false, true] {
        for text in ["A\u{301}", "AB\u{301}", "A\u{301}\u{301}"] {
            let mut expected = None;
            for index in 0..2 * CASES {
                let face = file.face(index).unwrap();
                let mut engine = face.engine().with_font_size(1000.0);
                if vertical {
                    engine = engine.with_vertical_flow();
                }
                let run = engine.shape(text).unwrap();
                let actual: Vec<_> = run
                    .glyphs
                    .iter()
                    .map(|g| {
                        (
                            g.x,
                            g.y,
                            g.glyph.metrics.advance_x,
                            g.glyph.metrics.advance_y,
                        )
                    })
                    .collect();
                if let Some(ref expected) = expected {
                    assert_eq!(&actual, expected, "face {index}");
                } else {
                    expected = Some(actual);
                }
            }
        }
    }
}
