use fontcore::{
    BidiBaseDirection, BidiParagraphLayout, CaretAffinity, FontFaceDescriptor, FontFamily,
    FontFile, FontOptions, LineBreakKind, ParagraphOptions, TextAlignment, TextPosition, WrapMode,
    WrappedLineLayout,
};
use std::io::ErrorKind;

const BIDI_FONT: &[u8] = include_bytes!("fixtures/bidi-layout.ttf");
const EUROPEAN_TTC: &[u8] = include_bytes!("fixtures/european-fallback.ttc");

fn pos(byte: usize, affinity: CaretAffinity) -> TextPosition {
    TextPosition::new(byte, affinity)
}

#[test]
fn wrapped_line_layout_supports_indent_alignment_justify_and_tabs() {
    let file = FontFile::from_buffer(BIDI_FONT).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // In bidi-layout.ttf: 'a'/'b' = 600, ' ' = 300.
    // "a b a b": each "a b" is 1500.
    // With max_inline_extent = 2400, first_line_indent = 600 (first line limit = 1800),
    // line 0 takes "a b " (1800), line 1 takes "a b" (1500).
    let text = "a b a b";
    let start_layout = WrappedLineLayout::shape_with_paragraph(
        &engine,
        text,
        2400.0,
        WrapMode::WordOrGrapheme,
        ParagraphOptions {
            alignment: TextAlignment::Start,
            first_line_indent: 600.0,
            tab_width: None,
        },
    )
    .unwrap();
    assert_eq!(start_layout.lines().len(), 2);
    assert_eq!(&text[start_layout.lines()[0].content_range()], "a b ");
    assert_eq!(start_layout.lines()[0].origin_x(), 600.0);
    assert_eq!(&text[start_layout.lines()[1].content_range()], "a b");
    assert_eq!(start_layout.lines()[1].origin_x(), 0.0);

    // Center alignment on single line "ab" (1200) inside 2400: origin_x = 600.
    let center = WrappedLineLayout::shape_with_paragraph(
        &engine,
        "ab",
        2400.0,
        WrapMode::Grapheme,
        ParagraphOptions {
            alignment: TextAlignment::Center,
            first_line_indent: 0.0,
            tab_width: None,
        },
    )
    .unwrap();
    assert!((center.lines()[0].origin_x() - 600.0).abs() < 0.01);
    assert!((center.hit_test(600.0, 0.0).unwrap().x() - 600.0).abs() < 0.01);
    assert!((center.hit_test(1800.0, 0.0).unwrap().x() - 1800.0).abs() < 0.01);

    // End alignment on "ab" (1200) inside 2400: origin_x = 1200.
    let end_layout = WrappedLineLayout::shape_with_paragraph(
        &engine,
        "ab",
        2400.0,
        WrapMode::Grapheme,
        ParagraphOptions {
            alignment: TextAlignment::End,
            first_line_indent: 0.0,
            tab_width: None,
        },
    )
    .unwrap();
    assert!((end_layout.lines()[0].origin_x() - 1200.0).abs() < 0.01);

    // Justify expands internal space on soft-wrapped line.
    // "a b a b" with extent = 2400, indent = 0 -> line 0 is "a b " (1800), wait:
    // "a b a" is 600+300+600+300+600 = 2400 -> fits on line 0 with "a b " + "a"? Let's use "aa b cc"
    // "aa " = 1500, "b " = 900 -> "aa b " = 2400; use extent = 2000 -> line 0 is "aa b" (1200+300+600=2100 > 2000),
    // "a b cd": "a b " = 1800, "cd" = 1200. Extent 2100 -> line 0 "a b " (soft), internal space at 1..2 gets +300 -> inline_advance = 2100.
    let justified = WrappedLineLayout::shape_with_paragraph(
        &engine,
        "a b cd",
        2100.0,
        WrapMode::WordOrGrapheme,
        ParagraphOptions {
            alignment: TextAlignment::Justify,
            first_line_indent: 0.0,
            tab_width: None,
        },
    )
    .unwrap();
    assert_eq!(justified.lines()[0].break_kind(), LineBreakKind::Soft);
    assert!((justified.lines()[0].inline_advance() - 2100.0).abs() < 0.01);
    let b_pos = justified
        .caret_positions(pos(2, CaretAffinity::Downstream))
        .unwrap()[0];
    // 'a'(600) + ' '(300) + gap(300) = 1200
    assert!((b_pos.x() - 1200.0).abs() < 0.01);

    // Tab expansion: "a\tb" with tab_width = 1000 -> 'a' at 0..600, '\t' advances to 1000, 'b' at 1000..1600.
    let tabbed = WrappedLineLayout::shape_with_paragraph(
        &engine,
        "a\tb",
        4000.0,
        WrapMode::Grapheme,
        ParagraphOptions {
            alignment: TextAlignment::Start,
            first_line_indent: 0.0,
            tab_width: Some(1000.0),
        },
    )
    .unwrap();
    assert!((tabbed.lines()[0].inline_advance() - 1600.0).abs() < 0.01);
    assert!(
        (tabbed
            .caret_positions(pos(2, CaretAffinity::Downstream))
            .unwrap()[0]
            .x()
            - 1000.0)
            .abs()
            < 0.01
    );
}

#[test]
fn wrapped_rejects_invalid_paragraph_options() {
    let file = FontFile::from_buffer(BIDI_FONT).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(1000.0);

    for bad in [
        ParagraphOptions {
            alignment: TextAlignment::Start,
            first_line_indent: -1.0,
            tab_width: None,
        },
        ParagraphOptions {
            alignment: TextAlignment::Start,
            first_line_indent: 1000.0,
            tab_width: None,
        },
        ParagraphOptions {
            alignment: TextAlignment::Start,
            first_line_indent: 0.0,
            tab_width: Some(0.0),
        },
    ] {
        let err =
            WrappedLineLayout::shape_with_paragraph(&engine, "ab", 1000.0, WrapMode::Grapheme, bad)
                .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn bidi_paragraph_layout_supports_font_family_fallback() {
    let euro_file = FontFile::from_buffer(EUROPEAN_TTC).unwrap();
    let bidi_file = FontFile::from_buffer(BIDI_FONT).unwrap();

    // EUROPEAN_TTC face 1 has 'ß', 'f', 'i', 'B', etc., but not Hebrew 'א'.
    // BIDI_FONT face 0 has 'א', 'ב', 'a', 'b'.
    let mut family = FontFamily::new("BidiFallback");
    family.add_face(
        FontFaceDescriptor::new("BidiFallback"),
        euro_file.face(1).unwrap(),
    );
    family.add_face(
        FontFaceDescriptor::new("BidiFallback"),
        bidi_file.face(0).unwrap(),
    );
    let options = FontOptions::from_family(&family)
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    let text = "ßאבß";
    let layout = BidiParagraphLayout::shape_family(
        &family,
        text,
        options.clone(),
        BidiBaseDirection::LeftToRight,
    )
    .unwrap();
    assert_eq!(layout.lines().len(), 1);
    let runs = layout.lines()[0].visual_runs();
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0].shaped().glyph_sources()[0].face_index(), 0);
    assert_eq!(runs[1].shaped().glyph_sources()[0].face_index(), 1);
    assert_eq!(runs[2].shaped().glyph_sources()[0].face_index(), 0);

    let wrapped = BidiParagraphLayout::shape_family_wrapped(
        &family,
        text,
        options,
        BidiBaseDirection::LeftToRight,
        1500.0,
        WrapMode::Grapheme,
    )
    .unwrap();
    assert!(wrapped.lines().len() >= 2);
}

#[test]
fn bidi_paragraph_layout_wraps_mixed_runs_and_preserves_boundary_roundtrip() {
    let file = FontFile::from_buffer(BIDI_FONT).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // "a אב b": 'a'(600) + ' '(300) + 'א'(600) + 'ב'(600) + ' '(300) + 'b'(600) = 3000.
    // With limit 2200 and WordOrGrapheme:
    // line 0 fits "a אב " (2400 > 2200) -> wraps at space "a " (900) or "a אב" (2100)?
    // Actually binary search finds "a אב" = 2100 <= 2200, then WordOrGrapheme scans back for space:
    // ' ' at index 1..2 -> "a " (900). Let's test Grapheme wrap at 2200 so line 0 = "a אב" (mixed LTR+RTL)
    // and line 1 = " b"!
    let text = "a אב b";
    let layout = BidiParagraphLayout::shape_wrapped(
        &engine,
        text,
        BidiBaseDirection::LeftToRight,
        2200.0,
        WrapMode::Grapheme,
    )
    .unwrap();
    assert_eq!(layout.lines().len(), 2);
    assert_eq!(&text[layout.lines()[0].content_range()], "a אב");
    assert_eq!(&text[layout.lines()[1].content_range()], " b");

    // Soft-wrap boundary byte 6 ("a אב".len() == 6) has dual carets: Upstream on line 0, Downstream on line 1.
    let dual = layout.visual_carets(6).unwrap();
    assert_eq!(dual.len(), 2);
    let up = layout
        .caret_positions(pos(6, CaretAffinity::Upstream))
        .unwrap()[0];
    let down = layout
        .caret_positions(pos(6, CaretAffinity::Downstream))
        .unwrap()[0];
    assert_eq!(up.line_index(), 0);
    assert_eq!(down.line_index(), 1);
    assert_eq!(down.x(), 0.0);
    assert_eq!(down.y(), 1200.0);

    // hit_test on line 1 start round-trips to Downstream on line 1.
    let hit_line1 = layout.hit_test(-100.0, 1200.0).unwrap();
    assert_eq!(hit_line1, down);
    assert_eq!(
        layout.caret_positions(hit_line1.text_position()).unwrap(),
        vec![down]
    );

    // RTL base direction flips logical Start alignment to the right edge (limit - width).
    let rtl_start = BidiParagraphLayout::shape_wrapped_with_options(
        &engine,
        "אב",
        BidiBaseDirection::RightToLeft,
        2400.0,
        WrapMode::Grapheme,
        ParagraphOptions {
            alignment: TextAlignment::Start,
            first_line_indent: 200.0,
            tab_width: None,
        },
    )
    .unwrap();
    // "אב" width = 1200; limit = 2400, indent = 200 -> placed in [1000, 2200], so run origin_x = 2200.
    assert!((rtl_start.lines()[0].visual_runs()[0].origin_x() - 2200.0).abs() < 0.01);
    let rtl_start_caret = rtl_start
        .caret_positions(pos(0, CaretAffinity::Downstream))
        .unwrap()[0];
    assert!((rtl_start_caret.x() - 2200.0).abs() < 0.01);
}
