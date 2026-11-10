use fontcore::{
    vertical_orientation, VerticalOrientation as Vo, VERTICAL_ORIENTATION_UNICODE_VERSION,
};

const SOURCE: &str = include_str!("../data/unicode/18.0.0/VerticalOrientation.txt");

#[derive(Debug)]
struct PropertyRange {
    start: u32,
    end: u32,
    value: Vo,
}

// Parse the original UCD independently of the Python generator and Rust lookup.
fn parse_range(line: &str) -> PropertyRange {
    let (range, value) = line.split_once(';').unwrap();
    let value = match value.trim() {
        "U" => Vo::Upright,
        "R" => Vo::Rotated,
        "Tu" => Vo::TransformWithUprightFallback,
        "Tr" => Vo::TransformWithRotatedFallback,
        other => panic!("unexpected property value {other}"),
    };
    let range = range.trim();
    let (start, end) = range.split_once("..").unwrap_or((range, range));
    let start = u32::from_str_radix(start, 16).unwrap();
    let end = u32::from_str_radix(end, 16).unwrap();
    assert!(start <= end && end <= 0x10FFFF);
    PropertyRange { start, end, value }
}

fn official_data() -> (Vec<Vo>, Vec<PropertyRange>) {
    assert!(SOURCE.starts_with("# VerticalOrientation-18.0.0.txt\n"));
    let mut default = None;
    let mut ranges: Vec<PropertyRange> = Vec::new();
    for line in SOURCE.lines() {
        if let Some(missing) = line.strip_prefix("# @missing:") {
            assert!(default.is_none(), "multiple @missing directives");
            let missing = parse_range(missing);
            assert_eq!((missing.start, missing.end), (0, 0x10FFFF));
            assert_eq!(missing.value, Vo::Rotated);
            default = Some(missing.value);
            continue;
        }
        let data = line.split('#').next().unwrap().trim();
        if data.is_empty() {
            continue;
        }
        let range = parse_range(data);
        if let Some(previous) = ranges.last() {
            assert!(previous.end < range.start, "overlap or unsorted UCD data");
        }
        ranges.push(range);
    }
    assert_eq!(ranges.len(), 2499);
    let mut expected = vec![default.expect("missing default directive"); 0x110000];
    for range in &ranges {
        expected[range.start as usize..=range.end as usize].fill(range.value);
    }
    (expected, ranges)
}

#[test]
fn exposes_exact_four_property_values_and_pinned_version() {
    assert_eq!(VERTICAL_ORIENTATION_UNICODE_VERSION, "18.0.0");
    assert_eq!(
        fontcore::unicode::VERTICAL_ORIENTATION_UNICODE_VERSION,
        "18.0.0"
    );
    for (character, expected) in [
        ('漢', Vo::Upright),
        ('A', Vo::Rotated),
        ('、', Vo::TransformWithUprightFallback),
        ('ー', Vo::TransformWithRotatedFallback),
    ] {
        assert_eq!(vertical_orientation(character), expected);
        assert_eq!(fontcore::unicode::vertical_orientation(character), expected);
    }
    let _: fontcore::unicode::VerticalOrientation = Vo::Upright;
}

#[test]
fn every_scalar_matches_independently_expanded_official_data() {
    let (expected, _) = official_data();
    let mut counts = [0; 4];
    let mut surrogate_count = 0;
    for (scalar, &value) in expected.iter().enumerate() {
        let Some(character) = char::from_u32(scalar as u32) else {
            assert!((0xD800..=0xDFFF).contains(&scalar));
            assert_eq!(value, Vo::Rotated);
            surrogate_count += 1;
            continue;
        };
        assert_eq!(vertical_orientation(character), value, "U+{scalar:04X}");
        counts[match value {
            Vo::Upright => 0,
            Vo::Rotated => 1,
            Vo::TransformWithUprightFallback => 2,
            Vo::TransformWithRotatedFallback => 3,
        }] += 1;
    }
    assert_eq!(counts, [332_511, 779_339, 163, 51]);
    assert_eq!(counts.iter().sum::<usize>(), 1_112_064);
    assert_eq!(surrogate_count, 2048);
}

#[test]
fn every_source_boundary_and_its_neighbors_match() {
    let (expected, ranges) = official_data();
    for range in ranges {
        for boundary in [range.start, range.end] {
            for scalar in [
                boundary.checked_sub(1),
                Some(boundary),
                boundary.checked_add(1),
            ]
            .into_iter()
            .flatten()
            {
                if let Some(character) = char::from_u32(scalar) {
                    assert_eq!(
                        vertical_orientation(character),
                        expected[scalar as usize],
                        "boundary neighbor U+{scalar:04X} for {range:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn preserves_defaults_unassigned_private_use_and_non_bmp_values() {
    let (expected, ranges) = official_data();
    // An unlisted unassigned scalar uses @missing: R, not a block heuristic.
    assert!(!ranges
        .iter()
        .any(|range| range.start <= 0x0378 && 0x0378 <= range.end));
    for scalar in [
        0x0000, 0x0378, 0xD7FF, 0xE000, 0xF8FF, 0xFDD0, 0xFFFD, 0xFFFF, 0x10000, 0x1F600, 0x20000,
        0x3FFFD, 0x3FFFE, 0xF0000, 0xFFFFD, 0xFFFFE, 0x100000, 0x10FFFD, 0x10FFFE, 0x10FFFF,
    ] {
        assert_eq!(
            vertical_orientation(char::from_u32(scalar).unwrap()),
            expected[scalar as usize]
        );
    }
    for scalar in [0x2065, 0x2BFF, 0xE000, 0xF0000, 0x100000] {
        assert_eq!(
            vertical_orientation(char::from_u32(scalar).unwrap()),
            Vo::Upright
        );
    }
    assert_eq!(vertical_orientation('\u{0378}'), Vo::Rotated);
    assert!(char::from_u32(0xD800).is_none());
    assert!(char::from_u32(0xDFFF).is_none());
    assert!(char::from_u32(0x110000).is_none());
}

#[test]
fn includes_unicode_18_values_and_newly_assigned_characters() {
    // Newly assigned Tangut components retain the unassigned range's U value
    // from Unicode 17. Jurchen and the 1D250..1D281 range changed from R to U.
    for scalar in [
        0x18D80, 0x18DF2, 0x18DFF, 0x18E00, 0x19191, 0x191DF, 0x1D250, 0x1D281,
    ] {
        assert_eq!(
            vertical_orientation(char::from_u32(scalar).unwrap()),
            Vo::Upright
        );
    }
    // Unicode 18 also changes previously U scalar values to Tu or R.
    assert_eq!(
        vertical_orientation('\u{1B168}'),
        Vo::TransformWithUprightFallback
    );
    for scalar in [0x1CEDD, 0x1CEDE, 0x1CEDF, 0x1CEF1, 0x1CEFD] {
        assert_eq!(
            vertical_orientation(char::from_u32(scalar).unwrap()),
            Vo::Rotated
        );
    }
    assert_eq!(vertical_orientation('\u{191E0}'), Vo::Rotated);
}

#[test]
fn reports_scalar_properties_without_resolving_clusters() {
    // A caller cannot use these individual values as an emoji/combining-cluster
    // orientation decision: the function deliberately returns only the UCD value.
    for character in ['\u{0301}', '\u{200D}', '\u{FE0F}', '\u{E0100}'] {
        assert_eq!(vertical_orientation(character), Vo::Rotated);
    }
    assert_eq!(vertical_orientation('漢'), Vo::Upright);
    assert_eq!(vertical_orientation('😀'), Vo::Upright);
}
