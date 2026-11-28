use fontcore::{
    AnnotatedLineLayout, EmphasisMark, FontFile, RubyAlignment, RubyAnnotation, RubyLayoutOptions,
    VerticalWrappedLayout, WrapMode, WrappedLineLayout,
};
use std::io::ErrorKind;

const VERTICAL_METRICS: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");
const MIXED_TTC: &[u8] = include_bytes!("fixtures/mixed-vertical-ltr.ttc");

#[test]
fn horizontal_ruby_alignments_and_emphasis_positions() {
    let file = FontFile::from_buffer(VERTICAL_METRICS).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // "BBAB": 'B' = 1000, 'A' = 600.
    // Ruby on "BB" (0..2) with reading "A" (width 300 at 0.5 scale) and gap 50.
    // Emphasis on "AB" (2..4).
    let text = "BBAB";
    let wrapped = WrappedLineLayout::shape(&engine, text, 10000.0, WrapMode::Grapheme).unwrap();

    let center_layout = AnnotatedLineLayout::layout(
        &wrapped,
        &engine,
        &[RubyAnnotation {
            base_range: 0..2,
            reading: "A".to_string(),
        }],
        &[EmphasisMark {
            range: 2..4,
            mark: '、',
        }],
        RubyLayoutOptions {
            ruby_scale: 0.5,
            ruby_gap: 50.0,
            alignment: RubyAlignment::Center,
            emphasis_scale: 0.5,
            emphasis_gap: 40.0,
        },
    )
    .unwrap();

    assert_eq!(center_layout.rubies().len(), 1);
    let ruby = &center_layout.rubies()[0];
    assert_eq!(ruby.line_index(), 0);
    // Base "BB" spans [0, 2000], ruby "A" width = 600 * 0.5 = 300 -> centered origin_x = 850.0
    assert!((ruby.origin_x() - 850.0).abs() < 0.01);
    // Base ascent = 800, ruby_gap = 50 -> origin_y = -850.0
    assert!((ruby.origin_y() - (-850.0)).abs() < 0.01);

    // Start alignment puts origin_x at base_min_x = 0.0.
    let start_layout = AnnotatedLineLayout::layout(
        &wrapped,
        &engine,
        &[RubyAnnotation {
            base_range: 0..2,
            reading: "A".to_string(),
        }],
        &[],
        RubyLayoutOptions {
            alignment: RubyAlignment::Start,
            ..Default::default()
        },
    )
    .unwrap();
    assert!((start_layout.rubies()[0].origin_x() - 0.0).abs() < 0.01);

    // SpaceAround with multiple ruby glyphs distributes extra space between glyphs.
    let space_around = AnnotatedLineLayout::layout(
        &wrapped,
        &engine,
        &[RubyAnnotation {
            base_range: 0..2,
            reading: "AA".to_string(),
        }],
        &[],
        RubyLayoutOptions {
            alignment: RubyAlignment::SpaceAround,
            ..Default::default()
        },
    )
    .unwrap();
    let glyphs = &space_around.rubies()[0].glyph_run().glyphs;
    assert_eq!(glyphs.len(), 2);
    // Base width = 2000, ruby "AA" width = 600 -> step = 700 -> glyph 0 shifted by 350, glyph 1 by 1050.
    assert!((glyphs[0].x - 350.0).abs() < 0.01);
    assert!((glyphs[1].x - (300.0 + 1050.0)).abs() < 0.01);

    // Emphasis has 2 placements corresponding to 'A' (2..3) and 'B' (3..4).
    assert_eq!(center_layout.emphases().len(), 1);
    let emp = &center_layout.emphases()[0];
    assert_eq!(emp.placements().len(), 2);
    assert_eq!(emp.placements()[0].cluster_range(), 2..3);
    assert_eq!(emp.placements()[1].cluster_range(), 3..4);
    // 'A' midpoint = 2300, mark '、' width = 500 -> origin_x = 2050.
    assert!((emp.placements()[0].origin_x() - 2050.0).abs() < 0.01);
    assert!((emp.placements()[0].origin_y() - (-840.0)).abs() < 0.01);
}

#[test]
fn vertical_wrapped_ruby_and_multiline_emphasis() {
    let file = FontFile::from_buffer(MIXED_TTC).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(100.0).with_line_height(120.0);

    // In mixed-vertical-ltr.ttc face 0: '漢' is upright (advance_y = 100), 'V' is sideways.
    let text = "漢漢\n漢漢";
    let v_wrapped = VerticalWrappedLayout::shape_vertical_mixed_ltr(
        &engine,
        text,
        1000.0,
        WrapMode::Grapheme,
        &[],
    )
    .unwrap();

    let annotated = AnnotatedLineLayout::layout_vertical_wrapped(
        &v_wrapped,
        &engine,
        &[RubyAnnotation {
            base_range: 0..6,
            reading: "漢".to_string(),
        }],
        &[EmphasisMark {
            range: 7..13,
            mark: '、',
        }],
        RubyLayoutOptions::default(),
    )
    .unwrap();

    assert_eq!(annotated.rubies().len(), 1);
    let ruby = &annotated.rubies()[0];
    assert_eq!(ruby.line_index(), 0);
    // Right side of vertical column (origin_x = 0 + half_em).
    assert!(ruby.origin_x() > 0.0);
    assert!(ruby.origin_y() >= 0.0);

    assert_eq!(annotated.emphases().len(), 1);
    assert_eq!(annotated.emphases()[0].line_index(), 1);
    assert_eq!(annotated.emphases()[0].placements().len(), 2);
}

#[test]
fn ruby_layout_rejects_line_crossing_and_invalid_inputs() {
    let file = FontFile::from_buffer(VERTICAL_METRICS).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    let text = "AA\nBB";
    let wrapped = WrappedLineLayout::shape(&engine, text, 5000.0, WrapMode::Grapheme).unwrap();

    // Crossing line break
    let err = AnnotatedLineLayout::layout(
        &wrapped,
        &engine,
        &[RubyAnnotation {
            base_range: 1..4,
            reading: "A".to_string(),
        }],
        &[],
        RubyLayoutOptions::default(),
    )
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidInput);

    // Overlapping ruby & emphasis
    let err = AnnotatedLineLayout::layout(
        &wrapped,
        &engine,
        &[RubyAnnotation {
            base_range: 0..2,
            reading: "A".to_string(),
        }],
        &[EmphasisMark {
            range: 1..2,
            mark: '、',
        }],
        RubyLayoutOptions::default(),
    )
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}
