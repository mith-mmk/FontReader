use fontcore::FontFile;

const FONTS: &[u8] = include_bytes!("fixtures/vertical-lookup-chain.ttc");
const BASE: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");

fn check(index: usize, replacement: &str) {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(index).unwrap();
    let engine = face.engine().with_font_size(1000.0).with_vertical_flow();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.current_face().unwrap();
    let expected = base_face
        .engine()
        .with_font_size(1000.0)
        .with_vertical_flow();
    let run = engine.shape("AA").unwrap();
    let reference = expected.shape(replacement).unwrap();
    assert_eq!(run.glyphs.len(), 2, "face {index}");
    assert_eq!(
        run.glyphs[0].glyph.metrics.bounds, reference.glyphs[0].glyph.metrics.bounds,
        "face {index}"
    );
    assert_eq!(
        run.glyphs[0].glyph.metrics.advance_y, reference.glyphs[0].glyph.metrics.advance_y,
        "face {index}"
    );
    assert_eq!(
        run.glyphs[1].y, reference.glyphs[0].glyph.metrics.advance_y,
        "face {index}"
    );
    assert_eq!(
        engine.measure("A").unwrap(),
        expected.measure(replacement).unwrap(),
        "face {index}"
    );
    assert_eq!(
        engine.render_svg("A").unwrap(),
        expected.render_svg(replacement).unwrap(),
        "face {index}"
    );
}

#[cfg(feature = "layout")]
#[test]
fn vertical_lookups_chain_in_lookup_list_order() {
    for index in [0, 1, 2, 13, 16, 18, 19] {
        check(index, "\u{3001}");
    }
}

#[cfg(feature = "layout")]
#[test]
fn vertical_lookups_apply_once_and_stop_at_first_matching_subtable() {
    for index in [3, 7] {
        check(index, "B");
    }
    check(4, "\u{3001}");
    for index in [5, 6] {
        check(index, "A");
    }
}

#[cfg(feature = "layout")]
#[test]
fn vertical_lookups_do_not_flatten_or_ignore_context() {
    for index in [8, 9, 10, 11, 12, 14, 15, 17] {
        check(index, "A");
    }
}

#[cfg(not(feature = "layout"))]
#[test]
fn vertical_lookup_chain_is_inactive_without_layout() {
    for index in 0..42 {
        check(index, "A");
    }
}

#[test]
fn horizontal_lookup_chain_is_inactive() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let base = FontFile::from_buffer(BASE).unwrap();
    let base_face = base.current_face().unwrap();
    let expected = base_face.engine().with_font_size(1000.0);
    for index in 0..42 {
        let face = file.face(index).unwrap();
        let engine = face.engine().with_font_size(1000.0);
        assert_eq!(
            engine.render_svg("A").unwrap(),
            expected.render_svg("A").unwrap()
        );
        assert_eq!(engine.measure("A").unwrap(), 600.0);
    }
}

#[cfg(feature = "layout")]
#[test]
fn vertical_lookups_skip_gdef_dependent_flags_but_keep_right_to_left() {
    check(21, "B"); // A -> B, then IgnoreBaseGlyphs must not turn B into comma.
    for index in 20..40 {
        check(index, if index % 2 == 0 { "A" } else { "B" });
    }
    check(40, "B");
    check(41, "\u{3001}");
}
