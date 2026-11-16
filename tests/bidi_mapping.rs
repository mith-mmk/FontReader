//! Bounded layout invariants over combinations of visible and formatting input.
use fontcore::{BidiBaseDirection, BidiParagraphLayout, FontFile};
use unicode_segmentation::UnicodeSegmentation;

#[test]
fn short_mixed_inputs_keep_original_byte_boundaries_and_finite_geometry() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/bidi-layout.ttf")).unwrap();
    let face = file.current_face().unwrap();
    let engine = face.engine().with_font_size(20.0);
    let units = [
        "a", "א", "1", "(", ")", "\u{200f}", "\u{2067}", "\u{2069}", "\u{05b0}",
    ];
    let mut supported = 0;
    for a in units {
        for b in units {
            for c in units {
                let text = format!("{a}{b}{c}");
                let layout =
                    match BidiParagraphLayout::shape(&engine, &text, BidiBaseDirection::Auto) {
                        Ok(layout) => layout,
                        Err(error) => {
                            assert_eq!(
                                error.kind(),
                                std::io::ErrorKind::Unsupported,
                                "{text:?}: {error}"
                            );
                            continue;
                        }
                    };
                supported += 1;
                assert_eq!(layout.text(), text);
                let mut boundaries = vec![0];
                boundaries.extend(text.grapheme_indices(true).map(|(i, g)| i + g.len()));
                for byte in boundaries {
                    let carets = layout
                        .visual_carets(byte)
                        .unwrap_or_else(|e| panic!("{text:?} byte {byte}: {e}"));
                    assert!(!carets.is_empty());
                    for caret in carets {
                        assert!(caret.x().is_finite() && caret.y().is_finite());
                    }
                }
                for line in layout.lines() {
                    for run in line.visual_runs() {
                        assert!(text.get(run.byte_range()).is_some());
                        for source in run.shaped().glyph_sources() {
                            let local = source.byte_range();
                            let start = run.byte_range().start;
                            assert!(text.get(start + local.start..start + local.end).is_some());
                        }
                    }
                }
                for x in [-f32::MAX, 0.0, f32::MAX] {
                    let caret = layout.hit_test(x, 0.0).unwrap();
                    assert!(text.is_char_boundary(caret.text_position().byte_offset()));
                }
            }
        }
    }
    assert!(supported > 500, "unexpectedly narrow support: {supported}");
}
