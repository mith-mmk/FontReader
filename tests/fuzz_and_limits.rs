//! Fuzzing, resource-limit, WOFF2 error handling, and bounded table reader regressions.

use fontcore::{DecodeLimits, FontFile, FontOptions, FontStretch};
use std::io::ErrorKind;

const TTF_FIXTURE: &[u8] = include_bytes!("fixtures/vertical-metrics.ttf");
const TTC_FIXTURE: &[u8] = include_bytes!("fixtures/colr-vertical-origin.ttc");
const WOFF_FIXTURE: &[u8] = include_bytes!("fixtures/cff-vertical-origin.woff");
const BIDI_FIXTURE: &[u8] = include_bytes!("fixtures/bidi-layout.ttf");

struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn fill_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let chunk = self.next_u64().to_le_bytes();
            let take = (len - out.len()).min(chunk.len());
            out.extend_from_slice(&chunk[..take]);
        }
        out
    }
}

fn assert_run_finite(file: &FontFile) {
    let count = file.face_count().min(4);
    for idx in 0..count {
        let Ok(face) = file.face(idx) else {
            continue;
        };
        let _ = face.family();
        let _ = face.full_name();
        let _ = face.weight();
        let _ = face.stretch();
        let _ = face.is_italic();
        let _ = face.variation_axes();
        let _ = face.dump();
        for engine in [
            face.engine().with_font_size(24.0),
            face.engine().with_font_size(24.0).with_vertical_flow(),
        ] {
            if let Ok(run) = engine.shape("A\u{0301}あ") {
                for g in &run.glyphs {
                    assert!(g.x.is_finite() && g.y.is_finite());
                    assert!(g.glyph.metrics.advance_x.is_finite());
                    assert!(g.glyph.metrics.advance_y.is_finite());
                }
            }
            if let Ok(width) = engine.measure("A\u{0301}あ") {
                assert!(width.is_finite());
            }
            let _ = engine.render_svg("A");
        }
    }
}

#[test]
fn woff2_container_returns_unsupported_without_panic() {
    let mut header = vec![0u8; 48];
    header[0..4].copy_from_slice(b"wOF2");
    header[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    header[8..12].copy_from_slice(&48u32.to_be_bytes());
    let err = FontFile::from_buffer(&header).expect_err("invalid WOFF2 must return error");
    assert!(matches!(
        err.kind(),
        ErrorKind::InvalidData | ErrorKind::Unsupported
    ));

    assert!(FontFile::from_buffer(b"wOF2").is_err());
}

#[test]
fn decode_limits_reject_oversized_inputs_and_tables() {
    let small_input = DecodeLimits {
        max_input_bytes: 16,
        ..DecodeLimits::default()
    };
    let err = FontFile::from_buffer_with_limits(TTF_FIXTURE, small_input).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    assert!(err.to_string().contains("exceeds decode limit"), "{err}");

    let small_table = DecodeLimits {
        max_table_bytes: 16,
        ..DecodeLimits::default()
    };
    let err = FontFile::from_buffer_with_limits(TTF_FIXTURE, small_table).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    assert!(err.to_string().contains("exceeds decode limit"), "{err}");

    let small_total = DecodeLimits {
        max_total_decompressed_bytes: 64,
        ..DecodeLimits::default()
    };
    let err = FontFile::from_buffer_with_limits(TTF_FIXTURE, small_total).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    assert!(err.to_string().contains("exceeds decode limit"), "{err}");
}

#[test]
fn non_finite_and_invalid_options_never_panic() {
    let file = FontFile::from_buffer(TTF_FIXTURE).unwrap();
    let face = file.face(0).unwrap();

    for bad_size in [0.0_f32, -12.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let engine = face.engine().with_font_size(bad_size);
        assert!(engine.shape("A").is_err());
        assert!(engine.measure("A").is_err());
        assert!(engine.measure_layout("A").is_err());
        assert!(engine.render_svg("A").is_err());
    }

    for bad_stretch in [-1.0_f32, f32::NAN, f32::INFINITY] {
        let options = FontOptions::new(&face)
            .with_font_size(16.0)
            .with_font_stretch(FontStretch(bad_stretch));
        let engine = face.engine().with_options(options);
        assert!(engine.shape("A").is_err());
        assert!(engine.measure("A").is_err());
    }

    for bad_coord in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let engine = face
            .engine()
            .with_font_size(16.0)
            .with_variation("wght", bad_coord);
        let _ = engine.shape("A");
        let _ = engine.measure("A");
    }
}

#[test]
fn fuzz_random_and_header_prefixed_buffers_never_panic() {
    let mut rng = XorShift64(0x9E37_79B9_7F4A_7C15);
    let magics: [&[u8]; 6] = [b"", b"\x00\x01\x00\x00", b"OTTO", b"ttcf", b"wOFF", b"wOF2"];
    let limits = DecodeLimits {
        max_input_bytes: 1024 * 1024,
        max_table_bytes: 256 * 1024,
        max_total_decompressed_bytes: 1024 * 1024,
        max_metadata_bytes: 64 * 1024,
        max_svg_bytes: 64 * 1024,
    };

    for i in 0..420 {
        let magic = magics[i % magics.len()];
        let body_len = (rng.next_u64() as usize) % 512;
        let mut buf = magic.to_vec();
        buf.extend_from_slice(&rng.fill_bytes(body_len));
        if let Ok(file) = FontFile::from_buffer_with_limits(&buf, limits) {
            assert_run_finite(&file);
        }
    }
}

#[test]
fn fuzz_mutated_and_truncated_fixtures_never_panic_or_produce_non_finite() {
    let fixtures = [TTF_FIXTURE, TTC_FIXTURE, WOFF_FIXTURE, BIDI_FIXTURE];
    let limits = DecodeLimits {
        max_input_bytes: 2 * 1024 * 1024,
        max_table_bytes: 512 * 1024,
        max_total_decompressed_bytes: 2 * 1024 * 1024,
        max_metadata_bytes: 64 * 1024,
        max_svg_bytes: 64 * 1024,
    };
    let mut rng = XorShift64(0xD1B5_4A32_D192_ED03);

    for fixture in fixtures {
        for frac in [8usize, 3, 2, 1] {
            let cut = if frac == 1 {
                fixture.len().saturating_sub(1)
            } else {
                fixture.len() / frac
            };
            if let Ok(file) = FontFile::from_buffer_with_limits(&fixture[..cut], limits) {
                assert_run_finite(&file);
            }
        }

        for _ in 0..40 {
            let mut mutated = fixture.to_vec();
            let mutations = 1 + ((rng.next_u64() as usize) % 4);
            for _ in 0..mutations {
                let idx = (rng.next_u64() as usize) % mutated.len();
                let val = (rng.next_u64() & 0xFF) as u8;
                mutated[idx] = val;
            }
            if let Ok(file) = FontFile::from_buffer_with_limits(&mutated, limits) {
                assert_run_finite(&file);
            }
        }
    }
}

#[test]
fn bounded_table_reader_prevents_cross_table_overread() {
    // Construct a minimal SFNT where `name` declares string data past the `name` table end
    // into adjacent garbage bytes (`post`). Isolated `bounded_table_reader` must return an error
    // instead of reading adjacent table bytes.
    let mut name_table = Vec::new();
    name_table.extend_from_slice(&0u16.to_be_bytes()); // format = 0
    name_table.extend_from_slice(&1u16.to_be_bytes()); // count = 1
    name_table.extend_from_slice(&18u16.to_be_bytes()); // stringOffset = 18
                                                        // 1 NameRecord: claims length 40 starting at stringOffset+0 (extends far past name_table.len())
    name_table.extend_from_slice(&3u16.to_be_bytes());
    name_table.extend_from_slice(&1u16.to_be_bytes());
    name_table.extend_from_slice(&0x0409u16.to_be_bytes());
    name_table.extend_from_slice(&1u16.to_be_bytes());
    name_table.extend_from_slice(&40u16.to_be_bytes());
    name_table.extend_from_slice(&0u16.to_be_bytes());
    name_table.extend_from_slice(&[0u8; 4]); // total 22 bytes; 40 bytes string overflows!

    // SFNT with 2 tables: `name` (22 bytes) + `post` (64 bytes)
    let mut font = Vec::new();
    font.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    font.extend_from_slice(&2u16.to_be_bytes());
    font.extend_from_slice(&32u16.to_be_bytes());
    font.extend_from_slice(&1u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());

    let name_offset = 12 + 32;
    let post_offset = name_offset + 24;
    // name record
    font.extend_from_slice(b"name");
    font.extend_from_slice(&0u32.to_be_bytes());
    font.extend_from_slice(&(name_offset as u32).to_be_bytes());
    font.extend_from_slice(&(name_table.len() as u32).to_be_bytes());
    // post record
    font.extend_from_slice(b"post");
    font.extend_from_slice(&0u32.to_be_bytes());
    font.extend_from_slice(&(post_offset as u32).to_be_bytes());
    font.extend_from_slice(&64u32.to_be_bytes());

    font.extend_from_slice(&name_table);
    while font.len() < post_offset {
        font.push(0);
    }
    font.extend_from_slice(&[b'X'; 64]);

    assert!(FontFile::from_buffer(&font).is_err());
}
