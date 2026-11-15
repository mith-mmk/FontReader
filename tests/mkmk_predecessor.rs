use fontcore::FontFile;

const FONT: &[u8] = include_bytes!("fixtures/mkmk-predecessor.ttc");
const CASES: usize = 35;

#[cfg(feature = "layout")]
fn positions(case: usize, text: &str, locale: Option<&str>, vertical: bool) -> Vec<(f32, f32)> {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(case).unwrap();
    let mut engine = face.engine().with_font_size(1000.0);
    if let Some(locale) = locale {
        engine = engine.with_locale(locale);
    }
    if vertical {
        engine = engine.with_vertical_flow();
    }
    engine
        .shape(text)
        .unwrap()
        .glyphs
        .iter()
        .map(|g| (g.x, g.y))
        .collect()
}

#[test]
#[cfg(feature = "layout")]
fn each_lookup_selects_its_own_predecessor() {
    // Same CCC=230 preserves this order. All inputs have explicit mark-to-base
    // anchors, so failures cannot be confused with synthetic fallback policy.
    for extension in [0, CASES] {
        for locale in [None, Some("en"), Some("ja"), Some("jpn"), Some("ain")] {
            for (case, expected) in [
                (0, (200.0, 0.0)),
                (1, (200.0, -100.0)),
                (2, (200.0, -100.0)),
                (3, (200.0, -100.0)),
                (4, (200.0, -100.0)),
                (5, (200.0, 0.0)),
                (6, (200.0, 0.0)),
                (7, (200.0, 0.0)),
                (8, (200.0, 0.0)),
                (9, (200.0, -100.0)),
                (22, (200.0, -100.0)),
                (23, (200.0, -100.0)),
            ] {
                for text in ["A\u{301}\u{302}\u{301}", "A\u{301}\u{302}\u{302}\u{301}"] {
                    let actual = positions(extension + case, text, locale, false);
                    assert_eq!(actual.len(), text.chars().count());
                    assert_eq!(
                        *actual.last().unwrap(),
                        expected,
                        "case {case} extension {extension} locale {locale:?} text {text:?}"
                    );
                }
            }
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn selected_origin_and_zero_anchor_are_real_successes() {
    for extension in [0, CASES] {
        // The final acute inherits the previously attached acute, not the base.
        let run = positions(extension + 1, "A\u{301}\u{301}\u{302}\u{301}", None, false);
        assert_eq!(run.len(), 5);
        assert_eq!(*run.last().unwrap(), (200.0, -200.0));
        // A zero Mkmk delta must stop before mark-to-base fallback. Move the
        // selected predecessor using an earlier nonzero mark-to-base origin.
        let run = positions(extension + 7, "A\u{303}\u{302}\u{301}", None, false);
        assert_eq!(run.len(), 4);
        assert_eq!(run[1], (300.0, 0.0));
        assert_eq!(*run.last().unwrap(), (300.0, 0.0));
        for vertical in [false, true] {
            let adjacent = positions(extension + 1, "A\u{301}\u{301}", None, vertical);
            let skipped = positions(extension + 1, "A\u{301}\u{302}\u{301}", None, vertical);
            assert_eq!(adjacent.len(), 3);
            assert_eq!(skipped.len(), 4);
            assert_eq!(adjacent.last(), skipped.last());
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn explicit_nonmarks_are_barriers_even_when_mark2_covered() {
    for extension in [0, CASES] {
        for case in [10, 11, 12] {
            let run = positions(extension + case, "A\u{301}\u{302}\u{301}", None, false);
            // Covered comma is explicitly Base/Ligature/Component. It cannot
            // supply a mark origin and the older acute cannot be searched.
            assert_eq!(*run.last().unwrap(), (600.0, 0.0), "case {case}");
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn unknown_targets_preserve_immediate_attachment() {
    for extension in [0, CASES] {
        for case in [13, 14, 15] {
            assert_eq!(
                positions(extension + case, "A\u{301}\u{301}", None, false).last(),
                Some(&(200.0, -100.0)),
                "case {case}"
            );
            assert_eq!(
                positions(extension + case, "A\u{301}\u{302}\u{301}", None, false).last(),
                Some(&(600.0, 0.0)),
                "case {case}"
            );
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn substitutions_and_unknown_base_do_not_authorize_new_skipping() {
    for extension in [0, CASES] {
        for case in [18, 19, 20, 21, 24, 25, 29, 30, 31, 32] {
            let prefix = if matches!(case, 20 | 25 | 32) {
                "AA"
            } else {
                "A"
            };
            let skip = positions(
                extension + case,
                &format!("{prefix}\u{301}\u{302}\u{301}"),
                None,
                false,
            );
            let adjacent = positions(
                extension + case,
                &format!("{prefix}\u{301}\u{301}"),
                None,
                false,
            );
            assert_eq!(skip.last().unwrap().1, 0.0, "skip case {case}");
            assert_eq!(adjacent.last().unwrap().1, -100.0, "adjacent case {case}");
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn predecessor_search_stays_inside_original_cluster_and_run() {
    for extension in [0, CASES] {
        for case in [1, 2, 22, 23] {
            for text in [
                "\u{301}\u{302}\u{301}",
                "A\u{302}\u{301}",
                "A\u{301}B\u{302}\u{301}",
                "A\u{301}\tA\u{302}\u{301}",
            ] {
                let run = positions(extension + case, text, None, false);
                assert_eq!(run.last().unwrap().1, 0.0, "case {case} text {text:?}");
            }
            let after_newline =
                positions(extension + case, "A\u{301}\nA\u{302}\u{301}", None, false);
            let control = positions(extension + case, "A\nA\u{302}\u{301}", None, false);
            assert_eq!(after_newline.last(), control.last());
        }
    }
}

#[test]
#[cfg(not(feature = "layout"))]
fn fixtures_leave_no_layout_unchanged() {
    let file = FontFile::from_buffer(FONT).unwrap();
    for vertical in [false, true] {
        let mut expected = None;
        for index in 0..2 * CASES {
            let face = file.face(index).unwrap();
            let mut engine = face.engine().with_font_size(1000.0);
            if vertical {
                engine = engine.with_vertical_flow();
            }
            let run = engine.shape("A\u{301}\u{302}\u{301}").unwrap();
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

#[test]
#[cfg(feature = "layout")]
fn orientations_and_nonuniform_scale_preserve_selected_origin() {
    use fontcore::{FontStretch, TextDirection};
    let file = FontFile::from_buffer(FONT).unwrap();
    for extension in [0, CASES] {
        for case in [1, 2] {
            let face = file.face(extension + case).unwrap();
            for direction in [
                TextDirection::LeftToRight,
                TextDirection::RightToLeft,
                TextDirection::TopToBottom,
            ] {
                let options = face
                    .engine()
                    .options()
                    .with_font_size(50.0)
                    .with_font_stretch(FontStretch(2.0))
                    .with_text_direction(direction);
                let engine = face.engine().with_options(options);
                for orientation in 0..3 {
                    let shape = |text| match orientation {
                        0 => engine.shape(text).unwrap(),
                        1 => engine.shape_sideways_ltr(text).unwrap(),
                        _ => engine.shape_vertical_mixed_ltr(text).unwrap(),
                    };
                    let adjacent = shape("A\u{301}\u{301}");
                    let skipped = shape("A\u{301}\u{302}\u{301}");
                    assert_eq!(adjacent.glyphs.len(), 3);
                    assert_eq!(skipped.glyphs.len(), 4);
                    let a = adjacent.glyphs.last().unwrap();
                    let s = skipped.glyphs.last().unwrap();
                    assert_eq!(
                        (a.x, a.y),
                        (s.x, s.y),
                        "case {case} direction {direction:?} orientation {orientation}"
                    );
                }
            }
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn selectors_and_mapped_mark_joiners_do_not_authorize_skipping() {
    for extension in [0, CASES] {
        for case in [1, 2] {
            // The unsupported selector disappears from output, but the original
            // grapheme was not a nominal one-scalar/one-glyph sequence.
            let selected = positions(
                extension + case,
                "A\u{301}\u{fe0f}\u{302}\u{301}",
                None,
                false,
            );
            assert_eq!(selected.len(), 4);
            assert_eq!(*selected.last().unwrap(), (200.0, 0.0));
            for joiner in ['\u{200c}', '\u{200d}'] {
                // Both controls map to comma, an explicit GDEF Mark excluded by
                // the lookup. This exercises ownership eligibility rather than
                // relying on an unmapped .notdef glyph to stop traversal.
                let run = positions(
                    extension + case,
                    &format!("A\u{301}{joiner}\u{302}\u{301}"),
                    None,
                    false,
                );
                assert_eq!(run.len(), 5, "joiner {joiner:?}");
                assert_eq!(run[2], (300.0, 0.0), "mapped joiner {joiner:?}");
                assert_eq!(*run.last().unwrap(), (200.0, 0.0), "joiner {joiner:?}");
            }
        }
    }
}

#[test]
#[cfg(feature = "layout")]
fn class_preserving_singles_retain_simple_owner() {
    for extension in [0, CASES] {
        for case in [16, 17, 26, 27, 28, 33, 34] {
            assert_eq!(
                positions(extension + case, "A\u{301}\u{302}\u{301}", None, false).last(),
                Some(&(200.0, -100.0))
            );
        }
    }
}
