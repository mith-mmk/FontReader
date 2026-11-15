//! Public loading policy for safe malformed extension headers in an original MIT fixture.
use fontcore::FontFile;

const FONT: &[u8] = include_bytes!("fixtures/mark-coordinates.ttf");

fn u16_at(bytes: &[u8], offset: usize) -> usize {
    usize::from(u16::from_be_bytes(
        bytes[offset..offset + 2].try_into().unwrap(),
    ))
}

fn invalid_extension(tag: &[u8; 4], kind: u16) -> Vec<u8> {
    let mut bytes = FONT.to_vec();
    let record = (0..u16_at(&bytes, 4))
        .map(|i| 12 + i * 16)
        .find(|&i| &bytes[i..i + 4] == tag)
        .unwrap();
    let table = u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
    let list = table + u16_at(&bytes, table + 8);
    let lookup = list + u16_at(&bytes, list + 2);
    let subtable = lookup + u16_at(&bytes, lookup + 6);
    bytes[lookup..lookup + 2].copy_from_slice(&kind.to_be_bytes());
    // Invalid format and an absent target, never a recursive baseline input.
    bytes[subtable..subtable + 8].copy_from_slice(&[0, 2, 0, 1, 255, 255, 255, 255]);
    bytes
}

#[test]
fn malformed_extension_is_rejected_only_when_layout_tables_are_loaded() {
    for (tag, kind) in [(b"GSUB", 7), (b"GPOS", 9)] {
        let bytes = invalid_extension(tag, kind);
        let result = FontFile::from_buffer(&bytes);
        #[cfg(feature = "layout")]
        assert!(result.is_err());
        #[cfg(not(feature = "layout"))]
        {
            let original = FontFile::from_buffer(FONT).unwrap();
            let changed = result.unwrap();
            assert_eq!(
                original.face(0).unwrap().engine().render_svg("AB").unwrap(),
                changed.face(0).unwrap().engine().render_svg("AB").unwrap()
            );
        }
    }
}
