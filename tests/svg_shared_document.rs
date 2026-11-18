#![cfg(all(feature = "svg-fonts", feature = "layout"))]

use fontcore::{FontFile, GlyphLayer};

// Upstream binary, not a fixture bundled in this repository:
// https://github.com/googlefonts/color-fonts/blob/e830e042b0e93fd5b0197a7fe9f0398f6d87f467/fonts/noto-picosvg.ttf
// SHA-256: 51f4a7aef6b813f467b4966bfb8a80bc4963815ea25caaa7d26534a336bbcfce
//
// FONTCORE_NOTO_PICOSVG_FONT=/path/to/noto-picosvg.ttf \
// cargo test --all-features --test svg_shared_document -- --ignored
#[test]
#[ignore = "requires the pinned upstream Noto picosvg font in FONTCORE_NOTO_PICOSVG_FONT"]
fn noto_picosvg_shared_document_shapes_emoji() {
    let path = std::env::var_os("FONTCORE_NOTO_PICOSVG_FONT").expect("path to Noto picosvg font");
    let font =
        FontFile::from_buffer(&std::fs::read(path).expect("read Noto picosvg font")).unwrap();
    let face = font.face(0).unwrap();
    let engine = face.engine().with_font_size(256.0);

    for text in [
        "😀",
        "😀\u{fe0f}",
        "🙂\u{fe0f}",
        "🌈\u{fe0f}",
        "🌷\u{fe0f}",
        "👍\u{fe0f}",
        "🛸\u{fe0f}",
        "🧪\u{fe0f}",
        "🐶\u{fe0f}",
        "🚀\u{fe0f}",
        "👓\u{fe0f}",
        "👩‍🚀\u{fe0f}",
        "😀👍",
    ] {
        let run = engine
            .shape_with_svg_context(text)
            .unwrap_or_else(|error| panic!("{text}: {error}"));
        assert_eq!(
            run.run().glyphs.len(),
            if text == "😀👍" { 2 } else { 1 },
            "{text}"
        );
        for (gi, glyph) in run.run().glyphs.iter().enumerate() {
            assert!(!glyph.glyph.layers.is_empty(), "{text}");
            for (li, layer) in glyph.glyph.layers.iter().enumerate() {
                match layer {
                    GlyphLayer::Svg(svg) => {
                        assert!(run.svg_layer_transform(gi, li).is_some(), "{text}");
                        assert!(
                            svg.document.len() < 64 * 1024,
                            "{text}: document not pruned"
                        );
                    }
                    GlyphLayer::Path(path) => assert!(!path.commands.is_empty(), "{text}"),
                    GlyphLayer::Raster(_) => panic!("{text}: unexpected raster layer"),
                }
            }
        }
    }
}
