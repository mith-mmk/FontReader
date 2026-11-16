use super::Font;
use crate::{FontFile, FontOptions};

#[test]
fn mapped_parser_keeps_original_offsets_across_discarded_controls() {
    let text = "\r\u{fe0f}a\u{308}\r\n\tß";
    let units = Font::parse_text_units_mapped(text);
    assert_eq!(
        units.iter().map(|(_, r)| r.clone()).collect::<Vec<_>>(),
        vec![4..7, 7..9, 9..10, 10..12]
    );
    for (_, range) in units {
        assert!(text.get(range).is_some());
    }
}

#[test]
fn mapped_shape_matches_legacy_and_retains_original_multiline_bytes() {
    let file = FontFile::from_buffer(include_bytes!("../../tests/fixtures/european-fallback.ttc"))
        .unwrap();
    let face = file.face(1).unwrap();
    let options = FontOptions::new(&face);
    let text = "\r\u{fe0f}a\u{308}\r\n\tß";
    let (mapped, ranges) = face.font().text2glyph_run_mapped(text, &options).unwrap();
    let legacy = face.font().text2glyph_run(text, &options).unwrap();
    assert_eq!(format!("{mapped:?}"), format!("{legacy:?}"));
    assert_eq!(mapped.glyphs.len(), ranges.len());
    assert_eq!(ranges.last(), Some(&(10..12)));
    assert!(ranges[..ranges.len() - 1]
        .iter()
        .all(|range| *range == (4..7)));
}

#[test]
fn mapped_ligature_owns_all_input_graphemes() {
    let file = FontFile::from_buffer(include_bytes!("../../tests/fixtures/european-fallback.ttc"))
        .unwrap();
    let face = file.face(1).unwrap();
    let (_, ranges) = face
        .font()
        .text2glyph_run_mapped("ßfiẞ", &FontOptions::new(&face))
        .unwrap();
    #[cfg(feature = "layout")]
    assert_eq!(ranges, vec![0..2, 2..4, 4..7]);
    #[cfg(not(feature = "layout"))]
    assert_eq!(ranges, vec![0..2, 2..3, 3..4, 4..7]);
}

#[test]
fn bidi_core_removes_only_bidi_controls_after_analysis() {
    let file = FontFile::from_buffer(include_bytes!("../../tests/fixtures/european-fallback.ttc"))
        .unwrap();
    let face = file.face(1).unwrap();
    for direction in [
        crate::TextDirection::LeftToRight,
        crate::TextDirection::RightToLeft,
    ] {
        let options = FontOptions::new(&face).with_text_direction(direction);
        let text =
            "\u{2066}B\u{061c}\u{200e}\u{200f}\u{202a}\u{202b}\u{202c}\u{2067}\u{2068}\u{2069}ß";
        let (run, ranges) = face
            .font()
            .text2glyph_run_bidi_mapped(text, &options)
            .unwrap();
        let reference = face.font().text2glyph_run("Bß", &options).unwrap();
        assert_eq!(format!("{run:?}"), format!("{reference:?}"));
        assert_eq!(ranges, vec![3..4, text.len() - 2..text.len()]);
        // No change to legacy APIs: they still produce nominal control glyphs.
        assert!(
            face.font()
                .text2glyph_run(text, &options)
                .unwrap()
                .glyphs
                .len()
                > run.glyphs.len()
        );
    }
    let units = face
        .font()
        .shape_text_units_with_bidi_policy(
            "B\u{200c}\u{200d}B",
            false,
            true,
            None,
            crate::FontVariant::Normal,
            super::TextRunMode::Standard,
            true,
            true,
        )
        .unwrap();
    let chars: Vec<_> = units
        .iter()
        .filter_map(|unit| match unit {
            super::ResolvedTextUnit::Glyph(g) => Some(g.ch),
            _ => None,
        })
        .collect();
    assert_eq!(chars, vec!['B', '\u{200c}', '\u{200d}', 'B']);
}

#[test]
fn bidi_core_rejects_unsupported_modes_and_unencoded_mirrors() {
    let file = FontFile::from_buffer(include_bytes!("../../tests/fixtures/european-fallback.ttc"))
        .unwrap();
    let face = file.face(1).unwrap();
    let options = FontOptions::new(&face).with_text_direction(crate::TextDirection::RightToLeft);
    for text in ["B\u{202d}B", "B\u{202e}B", "B\tB", "B\nB", "\u{22ff}"] {
        assert_eq!(
            face.font()
                .text2glyph_run_bidi_mapped(text, &options)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::Unsupported
        );
    }
    let vertical = options.with_text_direction(crate::TextDirection::TopToBottom);
    assert_eq!(
        face.font()
            .text2glyph_run_bidi_mapped("B", &vertical)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::Unsupported
    );
}

#[test]
fn bidi_core_mirror_keeps_original_scalar_and_byte_envelope() {
    let file = FontFile::from_buffer(include_bytes!("../../tests/fixtures/european-fallback.ttc"))
        .unwrap();
    let face = file.face(1).unwrap();
    // Inspect nominal provenance as well as outlines: mapping must never report
    // the substituted ')' scalar or a modified source string.
    for rtl in [false, true] {
        let units = face
            .font()
            .shape_text_units_with_bidi_policy(
                "ß(\u{308}B",
                false,
                rtl,
                None,
                crate::FontVariant::Normal,
                super::TextRunMode::Standard,
                true,
                true,
            )
            .unwrap();
        let glyph = units
            .iter()
            .find_map(|unit| match unit {
                super::ResolvedTextUnit::Glyph(g) if g.ch == '(' => Some(g),
                _ => None,
            })
            .unwrap();
        assert_eq!((glyph.byte_start, glyph.byte_end), (2, 5));
        #[cfg(feature = "layout")]
        assert_eq!(glyph.shaping_ch, if rtl { ')' } else { '(' });
    }
}

#[test]
fn bidi_core_mirrors_authored_outlines_without_rewriting_source() {
    let file =
        FontFile::from_buffer(include_bytes!("../../tests/fixtures/bidi-layout.ttf")).unwrap();
    let face = file.current_face().unwrap();
    let rtl = FontOptions::new(&face).with_text_direction(crate::TextDirection::RightToLeft);
    let ltr = rtl
        .clone()
        .with_text_direction(crate::TextDirection::LeftToRight);
    for (original, mirrored) in [
        ('(', ')'),
        ('[', ']'),
        ('{', '}'),
        ('<', '>'),
        ('«', '»'),
        ('∈', '∋'),
    ] {
        let text = format!("\u{200f}{original}A");
        let (run, sources) = face.font().text2glyph_run_bidi_mapped(&text, &rtl).unwrap();
        let expected = face
            .font()
            .text2glyph_run(&format!("{mirrored}A"), &rtl)
            .unwrap();
        let unmirrored = face
            .font()
            .text2glyph_run(&format!("{original}A"), &rtl)
            .unwrap();
        assert_eq!(format!("{run:?}"), format!("{expected:?}"), "{original}");
        assert_ne!(
            format!("{run:?}"),
            format!("{unmirrored:?}"),
            "fixture must distinguish {original}"
        );
        assert_eq!(
            sources,
            vec![
                3..3 + original.len_utf8(),
                3 + original.len_utf8()..text.len()
            ]
        );
        let (even, _) = face.font().text2glyph_run_bidi_mapped(&text, &ltr).unwrap();
        let expected_even = face
            .font()
            .text2glyph_run(&format!("{original}A"), &ltr)
            .unwrap();
        assert_eq!(format!("{even:?}"), format!("{expected_even:?}"));
    }
}

#[cfg(feature = "layout")]
#[test]
fn bidi_core_filters_controls_before_joining_not_after_glyph_selection() {
    let file =
        FontFile::from_buffer(include_bytes!("../../tests/fixtures/bidi-layout.ttf")).unwrap();
    let face = file.current_face().unwrap();
    let rtl = FontOptions::new(&face).with_text_direction(crate::TextDirection::RightToLeft);
    let text = "ب\u{200e}ب\u{202c}ب";
    let (run, sources) = face.font().text2glyph_run_bidi_mapped(text, &rtl).unwrap();
    let expected = face.font().text2glyph_run("ببب", &rtl).unwrap();
    assert_eq!(format!("{run:?}"), format!("{expected:?}"));
    assert_eq!(sources, vec![0..2, 5..7, 10..12]);
    // Check actual contextual forms, not just that the nominal glyphs agree.
    let isolated = face.font().text2glyph_run("ب", &rtl).unwrap();
    for glyph in &run.glyphs {
        assert_ne!(
            format!("{:?}", glyph.glyph.layers),
            format!("{:?}", isolated.glyphs[0].glyph.layers)
        );
    }
    let legacy = face.font().text2glyph_run(text, &rtl).unwrap();
    assert_eq!(legacy.glyphs.len(), 5);
    assert!(legacy
        .glyphs
        .iter()
        .any(|g| g.glyph.metrics.advance_x > run.glyphs[0].glyph.metrics.advance_x));
}

#[cfg(feature = "layout")]
#[test]
fn bidi_core_empty_isolates_break_arabic_joining() {
    let file =
        FontFile::from_buffer(include_bytes!("../../tests/fixtures/bidi-layout.ttf")).unwrap();
    let face = file.current_face().unwrap();
    let rtl = FontOptions::new(&face).with_text_direction(crate::TextDirection::RightToLeft);
    let isolated = face.font().text2glyph_run("ب", &rtl).unwrap();
    for isolate in ['\u{2066}', '\u{2067}', '\u{2068}'] {
        let text = format!("ب{isolate}\u{2069}ب");
        let (run, sources) = face.font().text2glyph_run_bidi_mapped(&text, &rtl).unwrap();
        assert_eq!(sources, vec![0..2, 8..10]);
        assert_eq!(run.glyphs.len(), 2);
        for glyph in &run.glyphs {
            assert_eq!(
                format!("{:?}", glyph.glyph.layers),
                format!("{:?}", isolated.glyphs[0].glyph.layers)
            );
        }
    }
    // A zero-ink boundary must also prevent a Latin ligature across an isolate.
    let (run, _) = face
        .font()
        .text2glyph_run_bidi_mapped("f\u{2066}\u{2069}i", &rtl)
        .unwrap();
    assert_eq!(run.glyphs.len(), 2);
}
