use fontcore::{
    CaretAffinity, FontFaceDescriptor, FontFamily, FontFile, FontOptions, LineBreakKind,
    TextPosition, VerticalWrappedLayout, WrapMode,
};
use std::io::ErrorKind;

const MIXED_TTC: &[u8] = include_bytes!("fixtures/mixed-vertical-ltr.ttc");
const VERTICAL_METRICS: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");
const TCY_TTC: &[u8] = include_bytes!("fixtures/tate-chu-yoko.ttc");

fn pos(byte: usize, affinity: CaretAffinity) -> TextPosition {
    TextPosition::new(byte, affinity)
}

#[test]
fn vertical_wrapped_layout_wraps_columns_and_embeds_tate_chu_yoko() {
    let file = FontFile::from_buffer(TCY_TTC).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // In tate-chu-yoko.ttc face 0 (built from vertical-metrics.ttf):
    // '1' -> A (vmtx adv 1200), '2' -> B (vmtx adv 800), '、' -> comma (vmtx adv 1000).
    // TCY("12") has advance_y = 1000 (one em cell).
    // Text: "、12、":
    //   '、' (0..3): 1000
    //   "12" (3..5, TCY): 1000 -> total 2000
    //   '、' (5..8): 1000 -> total 3000
    // With max_inline_extent = 2100, column 0 takes "、12" (never splits "12") and column 1 takes "、".
    let text = "、12、";
    let tcy = [std::ops::Range { start: 3, end: 5 }];
    let layout = VerticalWrappedLayout::shape_vertical_mixed_ltr(
        &engine,
        text,
        2100.0,
        WrapMode::Grapheme,
        &tcy,
    )
    .unwrap();

    assert_eq!(layout.lines().len(), 2);
    assert_eq!(&text[layout.lines()[0].content_range()], "、12");
    assert_eq!(layout.lines()[0].break_kind(), LineBreakKind::Soft);
    assert_eq!(layout.lines()[0].origin_x(), 0.0);
    assert_eq!(layout.lines()[0].origin_y(), 0.0);

    assert_eq!(&text[layout.lines()[1].content_range()], "、");
    assert_eq!(layout.lines()[1].break_kind(), LineBreakKind::EndOfText);
    assert_eq!(layout.lines()[1].origin_x(), -1200.0);
    assert_eq!(layout.lines()[1].origin_y(), 0.0);

    // Soft-wrap boundary byte 5 resolves to both columns.
    let dual = layout.visual_carets(5).unwrap();
    assert_eq!(dual.len(), 2);
    assert_eq!(dual[0].line_index(), 0);
    assert_eq!(dual[0].text_position(), pos(5, CaretAffinity::Upstream));
    assert_eq!(dual[1].line_index(), 1);
    assert_eq!(dual[1].text_position(), pos(5, CaretAffinity::Downstream));
    assert_eq!(dual[1].x(), -1200.0);
    assert_eq!(dual[1].y(), 0.0);

    // Hit-test selects the nearest column and round-trips.
    let hit = layout.hit_test(-1150.0, 10.0).unwrap();
    assert_eq!(hit, dual[1]);
    assert_eq!(
        layout.caret_positions(hit.text_position()).unwrap(),
        vec![dual[1]]
    );
}

#[test]
fn vertical_wrapped_layout_honors_japanese_kinsoku() {
    let file = FontFile::from_buffer(VERTICAL_METRICS).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // 'B' is sideways (Rotated), so its horizontal advance (1000) rotates to advance_y = 1000.
    // '、' is upright with vmtx advance_y = 1000.
    // "BB、B": limit 2100 fits "BB" (2000), leaving '、' at start of column 1.
    // JapaneseKinsoku pushes out the second 'B' to column 1 so '、' is not at column start.
    let text = "BB、B";
    let layout = VerticalWrappedLayout::shape_vertical_mixed_ltr(
        &engine,
        text,
        2100.0,
        WrapMode::JapaneseKinsoku,
        &[],
    )
    .unwrap();
    assert_eq!(&text[layout.lines()[0].content_range()], "B");
    assert_eq!(&text[layout.lines()[1].content_range()], "B、");
    assert_eq!(&text[layout.lines()[2].content_range()], "B");
}

#[test]
fn vertical_mixed_family_selects_fallback_faces_and_wraps() {
    let mixed_file = FontFile::from_buffer(MIXED_TTC).unwrap();
    let base_file = FontFile::from_buffer(VERTICAL_METRICS).unwrap();

    // Face 0 in VERTICAL_METRICS has '、' (0x3001) and '「' (0x300C), but not '漢' (0x6F22).
    // Face 0 in MIXED_TTC has '漢', 'A', 'V', etc.
    let mut family = FontFamily::new("TestMixedVertical");
    family.add_face(
        FontFaceDescriptor::new("TestMixedVertical"),
        base_file.face(0).unwrap(),
    );
    family.add_face(
        FontFaceDescriptor::new("TestMixedVertical"),
        mixed_file.face(0).unwrap(),
    );

    let options = FontOptions::from_family(&family)
        .with_font_size(100.0)
        .with_line_height(120.0);

    let single = family
        .shape_vertical_mixed_ltr_mapped("、漢A", options.clone())
        .unwrap();
    assert_eq!(single.text(), "、漢A");
    assert!(!single.glyph_sources().is_empty());
    // '、' is covered by face 0, '漢' falls back to face 1.
    assert_eq!(single.glyph_sources()[0].face_index(), 0);
    assert!(single.glyph_sources().iter().any(|s| s.face_index() == 1));

    let wrapped = VerticalWrappedLayout::shape_vertical_mixed_family(
        &family,
        "、漢A",
        options,
        150.0,
        WrapMode::Grapheme,
        &[],
    )
    .unwrap();
    assert!(wrapped.lines().len() >= 2);
}

#[test]
fn vertical_wrapped_rejects_invalid_tate_chu_yoko_ranges() {
    let file = FontFile::from_buffer(TCY_TTC).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    let bad_cases: &[(&str, &[std::ops::Range<usize>])] = &[
        ("、12", &[std::ops::Range { start: 0, end: 0 }]),
        (
            "、12",
            &[
                std::ops::Range { start: 0, end: 2 },
                std::ops::Range { start: 1, end: 3 },
            ],
        ),
        ("、12", &[std::ops::Range { start: 0, end: 2 }]),
        ("、\n12", &[std::ops::Range { start: 0, end: 5 }]),
    ];
    for &(text, bad) in bad_cases {
        let err = VerticalWrappedLayout::shape_vertical_mixed_ltr(
            &engine,
            text,
            5000.0,
            WrapMode::Grapheme,
            bad,
        )
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}
