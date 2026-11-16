//! Original MIT synthetic fixtures: scalar locl must not invent ligature inputs.
use fontcore::{FontFile, GlyphRun};
fn same(a: &GlyphRun, b: &GlyphRun) {
    assert_eq!(a.glyphs.len(), b.glyphs.len());
    for (a, b) in a.glyphs.iter().zip(&b.glyphs) {
        assert_eq!(
            format!("{:?}", a.glyph.metrics),
            format!("{:?}", b.glyph.metrics)
        );
        assert_eq!((a.x, a.y), (b.x, b.y));
        assert_eq!(
            format!("{:?}", a.glyph.layers),
            format!("{:?}", b.glyph.layers)
        );
    }
}
#[test]
fn scalar_locale_preserves_nominal_and_valid_replacements() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/scalar-locl.ttc")).unwrap();
    let base = file.face(0).unwrap();
    for extension in 0..2 {
        for (mode, expected) in ["A", "C", "D", "A", "D", "C", "C", "C", "A", "A"]
            .iter()
            .enumerate()
        {
            let face = file.face(1 + extension * 10 + mode).unwrap();
            for locale in ["sa", "san", "sa-Deva"] {
                let run = face.engine().with_locale(locale).shape("A").unwrap();
                same(
                    &run,
                    &base
                        .engine()
                        .shape(if cfg!(feature = "layout") {
                            expected
                        } else {
                            "A"
                        })
                        .unwrap(),
                );
            }
            if mode == 0 || mode == 9 {
                let expected = if mode == 9 && cfg!(feature = "layout") {
                    "D"
                } else {
                    "AB"
                };
                same(
                    &face.engine().with_locale("san").shape("AB").unwrap(),
                    &base.engine().shape(expected).unwrap(),
                );
            }
        }
    }
}
#[test]
fn multi_input_joining_does_not_corrupt_arabic() {
    let file = FontFile::from_buffer(include_bytes!("fixtures/scalar-locl.ttc")).unwrap();
    let base = file.face(0).unwrap();
    for index in [1, 11] {
        let face = file.face(index).unwrap();
        for text in ["ب", "بب", "ببب"] {
            same(
                &face
                    .engine()
                    .with_right_to_left()
                    .with_locale("ar")
                    .shape(text)
                    .unwrap(),
                &base
                    .engine()
                    .with_right_to_left()
                    .with_locale("ar")
                    .shape(text)
                    .unwrap(),
            );
        }
    }
}

#[test]
fn family_scalar_locale_keeps_nominal_face() {
    use fontcore::{FontFaceDescriptor, FontFamily};
    let file = FontFile::from_buffer(include_bytes!("fixtures/scalar-locl.ttc")).unwrap();
    for index in [1, 11] {
        let mut family = FontFamily::new("Primary");
        family.add_face(
            FontFaceDescriptor::new("Primary"),
            file.face(index).unwrap(),
        );
        family.add_face(FontFaceDescriptor::new("Fallback"), file.face(0).unwrap());
        let options = family.options().with_locale("san");
        same(
            &family.text2glyph_run("AB", options.clone()).unwrap(),
            &file
                .face(0)
                .unwrap()
                .engine()
                .with_options(options)
                .shape("AB")
                .unwrap(),
        );
    }
}
