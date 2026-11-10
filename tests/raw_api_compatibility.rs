#![cfg(feature = "raw")]

use fontcore::fontheader::FontHeaders;
use fontcore::fontreader::{FontData, GlyphFormat};

#[test]
fn raw_font_variant_spellings_and_debug_output_are_preserved() {
    assert_eq!(format!("{:?}", GlyphFormat::CFF), "CFF");
    assert_eq!(format!("{:?}", GlyphFormat::SVG), "SVG");
    assert!(matches!(FontData::CFF(Vec::new()), FontData::CFF(_)));
    assert!(matches!(FontData::SVG(String::new()), FontData::SVG(_)));
    assert_eq!(format!("{:?}", FontData::CFF(Vec::new())), "CFF([])");
}

#[test]
fn raw_woff_header_variant_is_preserved() {
    let mut reader =
        bin_rs::reader::BytesReader::new(include_bytes!("fixtures/vertical-origin.woff"));
    let header = fontcore::fontheader::get_font_type(&mut reader).expect("WOFF header");
    assert!(matches!(header, FontHeaders::WOFF(_)));
}
