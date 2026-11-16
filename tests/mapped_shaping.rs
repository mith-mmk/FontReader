use fontcore::{CompositeFont, FontFaceDescriptor, FontFamily, FontFile};
const FONT: &[u8] = include_bytes!("fixtures/european-fallback.ttc");

#[test]
fn mapped_run_preserves_legacy_output_and_original_utf8() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(1).unwrap();
    let engine = face.engine().with_font_size(1000.0);
    for text in [
        "",
        "fiB",
        "ßẞä",
        "a\u{308}B",
        "\u{308}B",
        "fi\nfi",
        "B\r\nß\tẞ",
        "B\u{fe0f}B",
        "\u{fe0f}B",
        "f\u{200c}i",
        "f\u{200d}i",
    ] {
        let mapped = engine.shape_mapped(text).unwrap();
        assert_eq!(mapped.text(), text);
        assert_eq!(
            format!("{:?}", mapped.glyph_run()),
            format!("{:?}", engine.shape(text).unwrap()),
            "{text:?}"
        );
        assert_eq!(
            mapped.glyph_sources().len(),
            mapped.glyph_run().glyphs.len()
        );
        for source in mapped.glyph_sources() {
            let range = source.byte_range();
            assert!(range.start < range.end, "{text:?}: {range:?}");
            assert!(text.get(range).is_some());
        }
        let rebuilt: String = mapped
            .grapheme_ranges()
            .iter()
            .map(|r| &text[r.clone()])
            .collect();
        assert_eq!(rebuilt, text);
    }
}

#[test]
fn ligature_and_combining_cluster_ownership() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let face = file.face(1).unwrap();
    let engine = face.engine();
    let mapped = engine.shape_mapped("fißa\u{308}B").unwrap();
    let ranges: Vec<_> = mapped
        .glyph_sources()
        .iter()
        .map(|s| s.byte_range())
        .collect();
    if cfg!(feature = "layout") {
        assert_eq!(ranges[0], 0..2);
    }
    assert!(ranges.contains(&(2..4)));
    assert!(ranges.contains(&(4..7)));
    assert_eq!(ranges.last().unwrap(), &(7..8));
}

#[test]
fn fallback_mapping_keeps_global_offsets_and_face_identity() {
    let file = FontFile::from_buffer(FONT).unwrap();
    let primary = file.face(0).unwrap();
    let full = file.face(1).unwrap();
    let mut family = FontFamily::new("Primary");
    family.add_face(FontFaceDescriptor::new("Primary"), primary.clone());
    family.add_face(FontFaceDescriptor::new("Fallback"), full.clone());
    let text = "BäB\r\nBa\u{308}B\tẞ";
    let mapped = family.shape_mapped(text, family.options()).unwrap();
    assert_eq!(
        format!("{:?}", mapped.glyph_run()),
        format!(
            "{:?}",
            family.text2glyph_run(text, family.options()).unwrap()
        )
    );
    for range in [1..3, 7..10, 12..15] {
        assert!(
            mapped
                .glyph_sources()
                .iter()
                .any(|s| s.byte_range() == range && s.face_index() == 1),
            "{range:?}: {:?}",
            mapped.glyph_sources()
        );
    }
    let composite = CompositeFont::new(vec![&primary, &full]).unwrap();
    assert!(composite.shape_mapped_ltr(text).is_err());
    let text = "BäBa\u{308}Bẞ";
    let mapped = composite.shape_mapped_ltr(text).unwrap();
    assert_eq!(
        format!("{:?}", mapped.glyph_run()),
        format!("{:?}", composite.shape_ltr(text).unwrap())
    );
    for s in mapped.glyph_sources() {
        assert!(text.get(s.byte_range()).is_some());
    }
}

#[test]
fn script_normalization_and_direction_ranges_are_original_boundaries() {
    use fontcore::TextDirection;
    let cases: &[(&[u8], &str)] = &[
        (
            include_bytes!("fixtures/hangul-composition.ttc"),
            "B\u{1100}\u{1161}B",
        ),
        (
            include_bytes!("fixtures/kana-composition.ttc"),
            "Bか\u{3099}B",
        ),
        (
            include_bytes!("fixtures/tibetan-normalization.ttc"),
            "B\u{f40}\u{f73}B",
        ),
        (
            include_bytes!("fixtures/arabic-joining.ttf"),
            "\u{628}\u{628}\u{628}",
        ),
    ];
    for &(data, text) in cases {
        let file = FontFile::from_buffer(data).unwrap();
        let face = file.face(0).unwrap();
        for direction in [
            TextDirection::LeftToRight,
            TextDirection::RightToLeft,
            TextDirection::TopToBottom,
        ] {
            let engine = face
                .engine()
                .with_options(fontcore::FontOptions::new(&face).with_text_direction(direction));
            let mapped = engine.shape_mapped(text).unwrap();
            assert_eq!(
                format!("{:?}", mapped.glyph_run()),
                format!("{:?}", engine.shape(text).unwrap())
            );
            for source in mapped.glyph_sources() {
                let range = source.byte_range();
                assert!(text.get(range.clone()).is_some(), "{text:?}: {range:?}");
                assert!(mapped
                    .grapheme_ranges()
                    .iter()
                    .any(|r| r.start == range.start));
                assert!(mapped.grapheme_ranges().iter().any(|r| r.end == range.end));
            }
        }
    }
}
