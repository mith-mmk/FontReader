use fontcore::{
    CaretAffinity, CompositeFont, CompositeFontRule, FontFaceDescriptor, FontFamily, FontFile,
    FontOptions, HardLineLayout, LineBreakKind, TextDirection, TextPosition, UnicodeRange,
    WrapMode, WrappedLineLayout,
};
use std::io::ErrorKind;

const EUROPEAN_TTC: &[u8] = include_bytes!("fixtures/european-fallback.ttc");

fn pos(byte: usize, affinity: CaretAffinity) -> TextPosition {
    TextPosition::new(byte, affinity)
}

#[test]
fn wrapped_word_or_grapheme_prefers_space_and_preserves_all_bytes() {
    let file = FontFile::from_buffer(EUROPEAN_TTC).unwrap();
    let face = file.face(1).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    let full = engine.shape_mapped("Bß Bß").unwrap();
    let total_advance: f32 = full
        .glyph_run()
        .glyphs
        .iter()
        .map(|g| g.glyph.metrics.advance_x)
        .sum();
    let first_word_with_space = engine.shape_mapped("Bß ").unwrap();
    let first_adv: f32 = first_word_with_space
        .glyph_run()
        .glyphs
        .iter()
        .map(|g| g.glyph.metrics.advance_x)
        .sum();
    assert!(first_adv < total_advance);

    // Choose a width that fits "Bß " plus part of the second word, but not the whole line.
    let limit = first_adv + (total_advance - first_adv) * 0.5;
    let layout =
        WrappedLineLayout::shape(&engine, "Bß Bß", limit, WrapMode::WordOrGrapheme).unwrap();

    assert_eq!(layout.text(), "Bß Bß");
    assert_eq!(layout.lines().len(), 2);
    assert_eq!(&layout.text()[layout.lines()[0].content_range()], "Bß ");
    assert_eq!(layout.lines()[0].break_kind(), LineBreakKind::Soft);
    assert_eq!(layout.lines()[0].break_range(), 4..4);
    assert_eq!(layout.lines()[0].origin_x(), 0.0);
    assert_eq!(layout.lines()[0].origin_y(), 0.0);

    assert_eq!(&layout.text()[layout.lines()[1].content_range()], "Bß");
    assert_eq!(layout.lines()[1].break_kind(), LineBreakKind::EndOfText);
    assert_eq!(layout.lines()[1].origin_x(), 0.0);
    assert_eq!(layout.lines()[1].origin_y(), 1200.0);

    // Soft-wrap boundary byte 4 has two distinct visual carets.
    let dual = layout.visual_carets(4).unwrap();
    assert_eq!(dual.len(), 2);
    assert_eq!(dual[0].line_index(), 0);
    assert_eq!(dual[0].text_position(), pos(4, CaretAffinity::Upstream));
    assert_eq!(dual[0].y(), 0.0);
    assert!(dual[0].x() > 0.0);

    assert_eq!(dual[1].line_index(), 1);
    assert_eq!(dual[1].text_position(), pos(4, CaretAffinity::Downstream));
    assert_eq!(dual[1].x(), 0.0);
    assert_eq!(dual[1].y(), 1200.0);

    // caret_positions respects affinity at the soft-wrap boundary.
    assert_eq!(
        layout
            .caret_positions(pos(4, CaretAffinity::Upstream))
            .unwrap(),
        vec![dual[0]]
    );
    assert_eq!(
        layout
            .caret_positions(pos(4, CaretAffinity::Downstream))
            .unwrap(),
        vec![dual[1]]
    );

    // hit_test round-trips to the exact same visual line at both sides of the wrap.
    let hit_end = layout.hit_test(dual[0].x() + 500.0, 0.0).unwrap();
    assert_eq!(hit_end, dual[0]);
    assert_eq!(
        layout.caret_positions(hit_end.text_position()).unwrap(),
        vec![dual[0]]
    );

    let hit_start = layout.hit_test(-100.0, 1200.0).unwrap();
    assert_eq!(hit_start, dual[1]);
    assert_eq!(
        layout.caret_positions(hit_start.text_position()).unwrap(),
        vec![dual[1]]
    );
}

#[test]
fn wrapped_grapheme_mode_and_forced_single_egc_progress() {
    let file = FontFile::from_buffer(EUROPEAN_TTC).unwrap();
    let face = file.face(1).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1000.0);

    // Tiny extent forces one EGC per visual line without infinite looping or dropping marks.
    let text = "a\u{308}B";
    let layout = WrappedLineLayout::shape(&engine, text, 1.0, WrapMode::Grapheme).unwrap();
    assert_eq!(layout.lines().len(), 2);
    assert_eq!(&text[layout.lines()[0].content_range()], "a\u{308}");
    assert_eq!(layout.lines()[0].break_kind(), LineBreakKind::Soft);
    assert_eq!(&text[layout.lines()[1].content_range()], "B");
    assert_eq!(layout.lines()[1].break_kind(), LineBreakKind::EndOfText);

    // Interior of combining mark sequence is still rejected as a non-EGC boundary.
    assert_eq!(
        layout
            .caret_positions(pos(1, CaretAffinity::Downstream))
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn wrapped_rtl_and_vertical_ttb_preserve_direction_axes() {
    let file = FontFile::from_buffer(EUROPEAN_TTC).unwrap();
    let face = file.face(1).unwrap();

    // RTL: negative-x inline flow, positive-y line step.
    let rtl_options = FontOptions::new(&face)
        .with_font_size(1000.0)
        .with_line_height(1100.0)
        .with_text_direction(TextDirection::RightToLeft);
    let rtl_engine = face.engine().with_options(rtl_options);
    let rtl = WrappedLineLayout::shape(&rtl_engine, "BB", 1.0, WrapMode::Grapheme).unwrap();
    assert_eq!(rtl.lines().len(), 2);
    assert_eq!(rtl.lines()[0].origin_y(), 0.0);
    assert_eq!(rtl.lines()[1].origin_y(), 1100.0);
    assert!(rtl.lines()[0].inline_advance() > 0.0);
    let rtl_end = rtl
        .caret_positions(pos(1, CaretAffinity::Upstream))
        .unwrap()[0];
    assert!(rtl_end.x() < 0.0);
    assert_eq!(rtl_end.line_index(), 0);

    // TTB: positive-y inline flow, negative-x column step.
    let ttb_options = FontOptions::new(&face)
        .with_font_size(1000.0)
        .with_line_height(1300.0)
        .with_text_direction(TextDirection::TopToBottom);
    let ttb_engine = face.engine().with_options(ttb_options);
    let ttb = WrappedLineLayout::shape(&ttb_engine, "BB", 1.0, WrapMode::Grapheme).unwrap();
    assert_eq!(ttb.lines().len(), 2);
    assert_eq!(ttb.lines()[0].origin_x(), 0.0);
    assert_eq!(ttb.lines()[1].origin_x(), -1300.0);
    assert!(ttb.lines()[0].inline_advance() > 0.0);
    let hit_col1 = ttb.hit_test(-1300.0, 0.0).unwrap();
    assert_eq!(hit_col1.line_index(), 1);
    assert_eq!(hit_col1.text_position(), pos(1, CaretAffinity::Downstream));
}

#[test]
fn hard_and_wrapped_family_fallback_and_composite_ltr() {
    let file = FontFile::from_buffer(EUROPEAN_TTC).unwrap();
    let face0 = file.face(0).unwrap();
    let face1 = file.face(1).unwrap();

    // FontFamily fallback across hard lines and wrapped lines.
    let mut family = FontFamily::new("EuropeanTest");
    family.add_face(FontFaceDescriptor::from_face(&face0), face0.clone());
    family.add_face(FontFaceDescriptor::from_face(&face1), face1.clone());
    let options = FontOptions::new(&face0)
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    let hard_fam = HardLineLayout::shape_family(&family, "Bß\r\nfi\n", options.clone()).unwrap();
    assert_eq!(hard_fam.lines().len(), 3);
    assert_eq!(&hard_fam.text()[hard_fam.lines()[0].content_range()], "Bß");
    assert_eq!(hard_fam.lines()[0].break_range(), 3..5);
    assert_eq!(&hard_fam.text()[hard_fam.lines()[1].content_range()], "fi");
    assert_eq!(&hard_fam.text()[hard_fam.lines()[2].content_range()], "");

    let wrapped_fam =
        WrappedLineLayout::shape_family(&family, "Bß\r\nfi", options, 1.0, WrapMode::Grapheme)
            .unwrap();
    assert_eq!(wrapped_fam.lines().len(), 4);
    assert_eq!(wrapped_fam.lines()[0].break_kind(), LineBreakKind::Soft);
    assert_eq!(wrapped_fam.lines()[1].break_kind(), LineBreakKind::Hard);
    assert_eq!(wrapped_fam.lines()[1].break_range(), 3..5);
    assert_eq!(wrapped_fam.lines()[2].break_kind(), LineBreakKind::Soft);
    assert_eq!(
        wrapped_fam.lines()[3].break_kind(),
        LineBreakKind::EndOfText
    );

    // CompositeFont multi-line & wrapped layout with size multiplier and baseline shift.
    let mut composite = CompositeFont::new(vec![&face0])
        .unwrap()
        .with_font_size(1000.0)
        .with_line_height(1400.0);
    composite.add_rule(
        CompositeFontRule::new(vec![UnicodeRange::new('A', 'Z').unwrap()], vec![&face1])
            .unwrap()
            .with_size_multiplier(1.2)
            .unwrap()
            .with_baseline_shift(50.0)
            .unwrap(),
    );

    let hard_comp = HardLineLayout::shape_composite_ltr(&composite, "Bß\nB").unwrap();
    assert_eq!(hard_comp.lines().len(), 2);
    assert_eq!(hard_comp.line_height(), 1400.0);
    assert_eq!(hard_comp.lines()[1].origin_y(), 1400.0);

    let wrapped_comp =
        WrappedLineLayout::shape_composite_ltr(&composite, "Bß", 1.0, WrapMode::Grapheme).unwrap();
    assert_eq!(wrapped_comp.lines().len(), 2);
    assert_eq!(wrapped_comp.lines()[1].origin_y(), 1400.0);
}

#[test]
fn wrapped_rejects_invalid_extent_and_controls() {
    let file = FontFile::from_buffer(EUROPEAN_TTC).unwrap();
    let face = file.face(1).unwrap();
    let engine = face.engine().with_font_size(1000.0);

    for bad in [0.0, -10.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            WrappedLineLayout::shape(&engine, "B", bad, WrapMode::WordOrGrapheme)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidInput
        );
    }

    assert_eq!(
        WrappedLineLayout::shape(&engine, "B\rB", 5000.0, WrapMode::WordOrGrapheme)
            .unwrap_err()
            .kind(),
        ErrorKind::Unsupported
    );
    assert_eq!(
        WrappedLineLayout::shape(&engine, "B\tB", 5000.0, WrapMode::WordOrGrapheme)
            .unwrap_err()
            .kind(),
        ErrorKind::Unsupported
    );
}
