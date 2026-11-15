//! Declared GSUB/GPOS bounds, using only the original MIT synthetic fixture.
use fontcore::FontFile;

const FONT: &[u8] = include_bytes!("fixtures/mark-coordinates.ttf");
fn word(bytes: &[u8], at: usize) -> usize {
    u16::from_be_bytes(bytes[at..at + 2].try_into().unwrap()) as usize
}
fn long(bytes: &[u8], at: usize) -> usize {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}
fn set_long(bytes: &mut [u8], at: usize, value: usize) {
    bytes[at..at + 4].copy_from_slice(&(value as u32).to_be_bytes());
}
fn replace(tag: &[u8; 4], table: &[u8], declared: usize) -> Vec<u8> {
    let mut bytes = FONT.to_vec();
    let record = (0..word(&bytes, 4))
        .map(|i| 12 + i * 16)
        .find(|&i| &bytes[i..i + 4] == tag)
        .unwrap();
    bytes.resize((bytes.len() + 3) & !3, 0);
    let offset = bytes.len();
    bytes.extend_from_slice(table);
    set_long(&mut bytes, record + 8, offset);
    set_long(&mut bytes, record + 12, declared);
    bytes
}
fn empty_table() -> Vec<u8> {
    // v1.0: empty ScriptList at 10, FeatureList at 12, LookupList at 16.
    vec![0, 1, 0, 0, 0, 10, 0, 12, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0]
}

#[test]
fn layout_reads_stay_inside_declared_table() {
    #[cfg(feature = "layout")]
    let mut accepted = Vec::new();
    for tag in [b"GSUB", b"GPOS"] {
        for declared in [8, 15, 16, 17] {
            let bytes = replace(tag, &empty_table(), declared);
            let result = FontFile::from_buffer(&bytes);
            #[cfg(feature = "layout")]
            if result.is_ok() {
                accepted.push((tag, declared));
            }
            #[cfg(not(feature = "layout"))]
            assert!(
                result.is_ok(),
                "layout-disabled loading ignores table bodies"
            );
        }
        assert!(FontFile::from_buffer(&replace(tag, &empty_table(), 18)).is_ok());
    }
    #[cfg(feature = "layout")]
    assert!(
        accepted.is_empty(),
        "out-of-table reads accepted: {accepted:?}"
    );
}

#[test]
fn large_extension_targets_stay_bounded_and_shared_ttc_tables_load() {
    for (tag, kind) in [(b"GSUB", 7u16), (b"GPOS", 9u16)] {
        let mut table = empty_table();
        table[16..18].copy_from_slice(&1u16.to_be_bytes());
        table.extend_from_slice(&4u16.to_be_bytes()); // Lookup at 20.
        for value in [kind, 0, 1, 8, 1, 1] {
            table.extend_from_slice(&value.to_be_bytes());
        }
        table.extend_from_slice(&65536u32.to_be_bytes()); // Extension at 28.
        table.resize(28 + 65536, 0);
        for value in [1u16, 6, 0, 1, 0] {
            table.extend_from_slice(&value.to_be_bytes());
        }
        // Target outside, exactly at the end, and a leaf Coverage that starts
        // inside but finishes outside. All backing bytes remain in the font.
        for declared in [table.len() - 11, table.len() - 10, table.len() - 1] {
            let bytes = replace(tag, &table, declared);
            let loaded = FontFile::from_buffer(&bytes);
            #[cfg(feature = "layout")]
            assert!(loaded.is_err(), "{tag:?} declared {declared}");
            #[cfg(not(feature = "layout"))]
            assert!(loaded.is_ok());
        }
        let bytes = replace(tag, &table, table.len());
        assert!(FontFile::from_buffer(&bytes).is_ok(), "{tag:?}");
        // Two faces deliberately share the same table directory and bodies.
        let mut ttc = b"ttcf\0\x01\0\0\0\0\0\x02\0\0\0\x14\0\0\0\x14".to_vec();
        ttc.extend_from_slice(&bytes);
        for i in 0..word(&bytes, 4) {
            let record = 12 + i * 16;
            set_long(&mut ttc, 20 + record + 8, 20 + long(&bytes, record + 8));
        }
        let loaded = FontFile::from_buffer(&ttc).unwrap();
        assert!(loaded.face(1).is_ok());
    }
}

fn woff(sfnt: &[u8]) -> Vec<u8> {
    let count = word(sfnt, 4);
    let mut bytes = vec![0; 44 + 20 * count];
    bytes[..4].copy_from_slice(b"wOFF");
    bytes[4..8].copy_from_slice(&sfnt[..4]);
    bytes[12..14].copy_from_slice(&(count as u16).to_be_bytes());
    let mut sfnt_size = 12 + 16 * count;
    for i in 0..count {
        let old = 12 + 16 * i;
        let new = 44 + 20 * i;
        let start = long(sfnt, old + 8);
        let len = long(sfnt, old + 12);
        bytes[new..new + 4].copy_from_slice(&sfnt[old..old + 4]);
        let mut data = sfnt[start..start + len].to_vec();
        if &sfnt[old..old + 4] == b"head" {
            data[8..12].fill(0);
        }
        data.resize((len + 3) & !3, 0);
        let checksum = data.chunks_exact(4).fold(0u32, |sum, chunk| {
            sum.wrapping_add(u32::from_be_bytes(chunk.try_into().unwrap()))
        });
        bytes[new + 16..new + 20].copy_from_slice(&checksum.to_be_bytes());
        let offset = bytes.len();
        set_long(&mut bytes, new + 4, offset);
        set_long(&mut bytes, new + 8, len);
        set_long(&mut bytes, new + 12, len);
        bytes.extend_from_slice(&sfnt[start..start + len]);
        bytes.resize((bytes.len() + 3) & !3, 0);
        sfnt_size += (len + 3) & !3;
    }
    let length = bytes.len();
    set_long(&mut bytes, 8, length);
    set_long(&mut bytes, 16, sfnt_size);
    bytes
}

#[test]
fn sfnt_matches_existing_woff_table_isolation() {
    for tag in [b"GSUB", b"GPOS"] {
        for length in [8, 15, 16, 17, 18] {
            let sfnt = replace(tag, &empty_table(), length);
            let compressed = woff(&sfnt);
            assert_eq!(
                FontFile::from_buffer(&sfnt).is_ok(),
                FontFile::from_buffer(&compressed).is_ok(),
                "{tag:?} length {length}: {:?}",
                FontFile::from_buffer(&compressed).err()
            );
            if length == 18 {
                let a = FontFile::from_buffer(&sfnt).unwrap();
                let b = FontFile::from_buffer(&compressed).unwrap();
                assert_eq!(
                    a.face(0).unwrap().engine().render_svg("AB").unwrap(),
                    b.face(0).unwrap().engine().render_svg("AB").unwrap()
                );
            }
        }
    }
}

#[test]
fn oversized_or_out_of_file_layout_records_fail_before_parsing() {
    for tag in [b"GSUB", b"GPOS"] {
        let bytes = replace(tag, &empty_table(), u32::MAX as usize);
        assert!(FontFile::from_buffer(&bytes).is_err());
        let limits = fontcore::DecodeLimits {
            max_table_bytes: usize::MAX,
            ..fontcore::DecodeLimits::default()
        };
        assert!(FontFile::from_buffer_with_limits(&bytes, limits).is_err());
    }
}
