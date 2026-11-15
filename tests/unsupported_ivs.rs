use fontcore::{FontFaceDescriptor, FontFamily, FontFile, GlyphRun, TextDirection};
const FONTS: &[u8] = include_bytes!("fixtures/unsupported-ivs.ttc");

fn same(a: &GlyphRun, b: &GlyphRun) {
    assert_eq!(format!("{:?}", a.glyphs), format!("{:?}", b.glyphs));
}
fn family(primary: usize, fallback: usize) -> FontFamily {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let mut family = FontFamily::new("Primary");
    family.add_face(
        FontFaceDescriptor::new("Primary"),
        file.face(primary).unwrap(),
    );
    family.add_face(
        FontFaceDescriptor::new("Fallback"),
        file.face(fallback).unwrap(),
    );
    family
}
#[test]
fn unsupported_pairs_keep_the_base_in_all_flows() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    for index in [0, 1] {
        let face = file.face(index).unwrap();
        for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
            let engine = face.engine().with_options(
                fontcore::FontOptions::new(&face)
                    .with_font_size(100.0)
                    .with_text_direction(direction),
            );
            for base in ["あ", "\u{20000}", "A"] {
                for vs in ["\u{e0100}", "\u{e01ef}", "\u{fe00}"] {
                    let text = format!("{base}{vs}");
                    same(&engine.shape(&text).unwrap(), &engine.shape(base).unwrap());
                    same(
                        &engine.shape_vertical_mixed_ltr(&text).unwrap(),
                        &engine.shape_vertical_mixed_ltr(base).unwrap(),
                    );
                    assert_eq!(
                        engine.render_svg(&text).unwrap(),
                        engine.render_svg(base).unwrap()
                    );
                }
            }
            same(
                &engine.shape("\u{e0100}あ\u{e0100}\u{e01ef}い").unwrap(),
                &engine.shape("あい").unwrap(),
            );
            same(
                &engine.shape("\u{10ffff}\u{e0100}").unwrap(),
                &engine.shape("\u{10ffff}").unwrap(),
            );
        }
    }
}
#[test]
fn exact_nondefault_and_default_remain_distinct() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    for (index, expected) in [(2, "い"), (3, "あ")] {
        let face = file.face(index).unwrap();
        for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
            let engine = face
                .engine()
                .with_options(fontcore::FontOptions::new(&face).with_text_direction(direction));
            same(
                &engine.shape("あ\u{e0100}").unwrap(),
                &engine.shape(expected).unwrap(),
            );
            same(
                &engine.shape("あ\u{e0100}\u{e01ef}").unwrap(),
                &engine.shape(expected).unwrap(),
            );
        }
    }
}
#[test]
fn family_prefers_exact_variants_then_complete_base_fallback() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    // A mapped variant wins over both absent-format14 and absent-pair primary.
    // A claimed variant without full mark coverage or with unrepaired glyph0 loses.
    for (primary, fallback, text, winner) in [
        (0, 2, "あ\u{e0100}", 2),
        (1, 2, "あ\u{e0100}", 2),
        (0, 3, "あ\u{e0100}", 3),
        (0, 5, "あ\u{e0100}", 0),
        (0, 6, "あ\u{e0100}\u{301}", 0),
        (0, 7, "あ\u{e0100}\u{301}", 0),
        // Continuity after a base-only segment must not hide an exact variant.
        (0, 2, "いあ\u{e0100}\u{301}", 2),
    ] {
        let family = family(primary, fallback);
        let face = file.face(winner).unwrap();
        for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
            let options = family.options().with_text_direction(direction);
            same(
                &family.text2glyph_run(text, options.clone()).unwrap(),
                &face.engine().with_options(options).shape(text).unwrap(),
            );
        }
    }
}
#[test]
fn explicit_zero_is_not_replaced_by_nominal_base() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(5).unwrap();
    same(
        &face.engine().shape("あ\u{e0100}").unwrap(),
        &face.engine().shape("\u{10ffff}").unwrap(),
    );
}
#[cfg(feature = "layout")]
#[test]
fn explicit_zero_reaches_contextual_gsub() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(4).unwrap();
    for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
        let engine = face
            .engine()
            .with_options(fontcore::FontOptions::new(&face).with_text_direction(direction));
        same(
            &engine.shape("あ\u{e0100}い").unwrap(),
            &engine.shape("、い").unwrap(),
        );
    }
}

#[test]
fn unsupported_selector_does_not_break_existing_mark_continuity() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let family = family(0, 8);
    let face = file.face(8).unwrap();
    let text = "うあ\u{e0100}\u{301}";
    same(
        &family.text2glyph_run(text, family.options()).unwrap(),
        &face.engine().shape(text).unwrap(),
    );
}
#[test]
fn trailing_supported_selector_does_not_override_first_pair() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let face = file.face(2).unwrap();
    same(
        &face.engine().shape("あ\u{e01ef}\u{e0100}").unwrap(),
        &face.engine().shape("あ").unwrap(),
    );
    // The base has exact support, but the second (mark, VS) pair does not.
    let family = family(0, 2);
    let nominal = file.face(0).unwrap();
    let text = "あ\u{e0100}\u{301}\u{e0100}";
    same(
        &family.text2glyph_run(text, family.options()).unwrap(),
        &nominal.engine().shape(text).unwrap(),
    );
}

#[cfg(feature = "cff")]
fn reject_invalid_cff_variant(bytes: &[u8]) {
    let file = FontFile::from_buffer(bytes).unwrap();
    let primary = file.face(0).unwrap();
    for (index, texts) in [
        (1, vec!["A\u{e0100}", "BA\u{e0100}"]),
        (2, vec!["A\u{e0100}\u{301}\u{e0100}"]),
    ] {
        let mut family = FontFamily::new("Primary");
        family.add_face(FontFaceDescriptor::new("Primary"), file.face(0).unwrap());
        family.add_face(
            FontFaceDescriptor::new("Fallback"),
            file.face(index).unwrap(),
        );
        for text in texts {
            for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
                let options = family.options().with_text_direction(direction);
                same(
                    &family.text2glyph_run(text, options.clone()).unwrap(),
                    &primary.engine().with_options(options).shape(text).unwrap(),
                );
            }
        }
    }
}
#[cfg(feature = "cff")]
#[test]
fn invalid_cff_exact_variant_does_not_steal_family_selection() {
    reject_invalid_cff_variant(include_bytes!("fixtures/unsupported-ivs-cff.ttc"));
}
#[cfg(feature = "cff2")]
#[test]
fn invalid_cff2_exact_variant_does_not_steal_family_selection() {
    reject_invalid_cff_variant(include_bytes!("fixtures/unsupported-ivs-cff2.ttc"));
}

#[cfg(feature = "layout")]
#[test]
fn invalid_post_gsub_mark_rejects_exact_variant_face() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let family = family(0, 9);
    let primary = file.face(0).unwrap();
    let text = "あ\u{e0100}\u{301}";
    same(
        &family.text2glyph_run(text, family.options()).unwrap(),
        &primary.engine().shape(text).unwrap(),
    );
}

#[cfg(feature = "layout")]
#[test]
fn family_can_prefer_explicit_zero_repaired_inside_the_cluster() {
    let file = FontFile::from_buffer(FONTS).unwrap();
    let family = family(0, 4);
    let repaired = file.face(4).unwrap();
    let text = "あ\u{e0100}\u{301}";
    for direction in [TextDirection::LeftToRight, TextDirection::TopToBottom] {
        let options = family.options().with_text_direction(direction);
        same(
            &family.text2glyph_run(text, options.clone()).unwrap(),
            &repaired.engine().with_options(options).shape(text).unwrap(),
        );
    }
}

#[cfg(feature = "raw")]
#[test]
fn raw_uvs_wrappers_keep_unsupported_bases_and_explicit_zero() {
    fn svg_markup(svg: String) -> String {
        // The leading diagnostic comment names the input scalar, not its glyph.
        svg.split_once("<svg").unwrap().1.to_owned()
    }
    let file = FontFile::from_buffer(FONTS).unwrap();
    for (index, expected) in [(1, 'あ'), (5, '\u{10ffff}')] {
        let face = file.face(index).unwrap();
        let raw = face.raw_font();
        for vertical in [false, true] {
            // GriphData exposes Debug but no public glyph-id/outline fields.
            assert_eq!(
                format!(
                    "{:?}",
                    raw.get_glyph_with_uvs_axis('あ', '\u{e0100}', vertical)
                ),
                format!(
                    "{:?}",
                    raw.get_glyph_with_uvs_axis(expected, '\0', vertical)
                ),
            );
            assert_eq!(
                svg_markup(
                    raw.get_svg_with_uvs_axis('あ', '\u{e0100}', 32.0, "px", vertical)
                        .unwrap()
                ),
                svg_markup(
                    raw.get_svg_with_uvs_axis(expected, '\0', 32.0, "px", vertical)
                        .unwrap()
                ),
            );
        }
        assert_eq!(
            format!("{:?}", raw.get_glyph_with_uvs('あ', '\u{e0100}')),
            format!("{:?}", raw.get_glyph(expected)),
        );
        assert_eq!(
            svg_markup(raw.get_svg_with_uvs('あ', '\u{e0100}', 32.0, "px").unwrap()),
            svg_markup(raw.get_svg(expected, 32.0, "px").unwrap()),
        );
    }
}
