use fontcore::{FontFile, LineBreakKind, WrapMode, WrappedLineLayout};

const VERTICAL_METRICS_TTF: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");

#[test]
fn japanese_kinsoku_pushes_out_line_start_prohibited_comma() {
    let file = FontFile::from_buffer(VERTICAL_METRICS_TTF).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // In vertical-metrics.ttf: 'A' = 600, 'B' = 1000, '、' (0x3001) = 1000.
    // "AAA、B": "AAA" = 1800, "A、" = 1600. With limit 1800:
    // Grapheme mode wraps after "AAA" -> ["AAA", "、B"].
    // JapaneseKinsoku avoids starting line 2 with "、" by pushing out 'A' -> ["AA", "A、..."].
    let text = "AAA、A";
    let grapheme = WrappedLineLayout::shape(&engine, text, 1800.0, WrapMode::Grapheme).unwrap();
    assert_eq!(&text[grapheme.lines()[0].content_range()], "AAA");
    assert!(text[grapheme.lines()[1].content_range()].starts_with('、'));

    let kinsoku =
        WrappedLineLayout::shape(&engine, text, 1800.0, WrapMode::JapaneseKinsoku).unwrap();
    assert_eq!(kinsoku.wrap_mode(), WrapMode::JapaneseKinsoku);
    let lines: Vec<&str> = kinsoku
        .lines()
        .iter()
        .map(|l| &text[l.content_range()])
        .collect();
    assert_eq!(lines[0], "AA");
    assert_eq!(lines[1], "A、");
    assert_eq!(lines[2], "A");
    assert_eq!(lines.concat(), text);
}

#[test]
fn japanese_kinsoku_pushes_out_line_end_prohibited_opening_bracket() {
    let file = FontFile::from_buffer(VERTICAL_METRICS_TTF).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // 'A' = 600, '「' = 1000, 'B' = 1000.
    // "AA「B": limit 2200 fits "AA「" (2200), but '「' is line-end prohibited.
    // Push-out moves '「' to the next line: ["AA", "「B"] (2000 <= 2200).
    let text = "AA「B";
    let kinsoku =
        WrappedLineLayout::shape(&engine, text, 2200.0, WrapMode::JapaneseKinsoku).unwrap();
    let lines: Vec<&str> = kinsoku
        .lines()
        .iter()
        .map(|l| &text[l.content_range()])
        .collect();
    assert_eq!(lines, vec!["AA", "「B"]);
}

#[test]
fn japanese_kinsoku_keeps_inseparable_ellipsis_pair_together() {
    let file = FontFile::from_buffer(VERTICAL_METRICS_TTF).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // 'A' = 600; '…' uses .notdef advance (0 or fallback). Use vertical flow or inspect line split
    // where "AA…" fits, then second '…' would normally wrap onto line 2.
    let a_adv = engine.measure("A").unwrap() as f32;
    let two_a_one_ellipsis = engine.measure("AA\u{2026}").unwrap() as f32;
    let limit = two_a_one_ellipsis + 1.0;
    let _ = a_adv;

    let text = "AA\u{2026}\u{2026}B";
    // Even if '…' has zero advance in this synthetic font, test non-zero case via 'A' + "……"
    // by forcing limit strictly between 1-ellipsis and 2-ellipsis if distinct, or checking no split.
    let layout = WrappedLineLayout::shape(&engine, text, limit, WrapMode::JapaneseKinsoku).unwrap();
    for line in layout.lines() {
        let s = &text[line.content_range()];
        assert!(
            !(s.ends_with('\u{2026}') && !s.ends_with("\u{2026}\u{2026}")),
            "inseparable ellipsis split across lines: {s:?}"
        );
    }
}

#[test]
fn japanese_kinsoku_hanging_allows_single_trailing_punctuation_past_extent() {
    let file = FontFile::from_buffer(VERTICAL_METRICS_TTF).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // "AA、B": 'A' = 600, '、' = 1000. Limit 1500 fits "AA" (1200), while "AA、" is 2200.
    // Hanging mode allows '、' to hang on line 0, leaving "B" on line 1.
    let text = "AA、B";
    let layout =
        WrappedLineLayout::shape(&engine, text, 1500.0, WrapMode::JapaneseKinsokuHanging).unwrap();
    assert_eq!(layout.lines().len(), 2);
    assert_eq!(&text[layout.lines()[0].content_range()], "AA、");
    assert_eq!(layout.lines()[0].break_kind(), LineBreakKind::Soft);
    assert!(layout.lines()[0].inline_advance() > 1500.0);
    assert_eq!(&text[layout.lines()[1].content_range()], "B");
    assert_eq!(layout.lines()[1].break_kind(), LineBreakKind::EndOfText);
}

#[test]
fn japanese_kinsoku_always_makes_forward_progress_when_unavoidable() {
    let file = FontFile::from_buffer(VERTICAL_METRICS_TTF).unwrap();
    let face = file.face(0).unwrap();
    let engine = face
        .engine()
        .with_font_size(1000.0)
        .with_line_height(1200.0);

    // All characters are line-start prohibited; push-out falls back to chosen_count >= 1.
    let text = "、、、";
    let layout =
        WrappedLineLayout::shape(&engine, text, 1000.0, WrapMode::JapaneseKinsoku).unwrap();
    assert_eq!(layout.lines().len(), 3);
    for line in layout.lines() {
        assert_eq!(&text[line.content_range()], "、");
    }
}
