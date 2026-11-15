//! Basic Arabic-style joining analysis, independent of font feature coverage.
//! Not the Syriac joining-group state machine or a bidirectional algorithm.
use super::joining_type_data::RANGES;

fn joining_type(ch: char) -> u8 {
    let scalar = ch as u32;
    RANGES
        .binary_search_by(|&(start, end, _)| {
            if end < scalar {
                std::cmp::Ordering::Less
            } else if start > scalar {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .map(|index| RANGES[index].2)
        .unwrap_or(b'U')
}

/// Analyze logical-order source scalars before ccmp changes the glyph stream.
/// Transparent characters have no form; join-causing characters connect their
/// neighbors but need no joining substitution themselves.
pub(crate) fn forms(chars: &[char]) -> Vec<Option<[u8; 4]>> {
    let mut forms = vec![None; chars.len()];
    let mut previous: Option<(usize, u8, bool)> = None;
    for (index, &ch) in chars.iter().enumerate() {
        let kind = joining_type(ch);
        if kind == b'T' {
            continue;
        }
        let joins_previous = previous.is_some_and(|(_, prev, _)| {
            matches!(prev, b'D' | b'L' | b'C') && matches!(kind, b'D' | b'R' | b'C')
        });
        if let Some((prev_index, prev_kind, prev_joined)) = previous {
            if matches!(prev_kind, b'D' | b'L' | b'R') {
                forms[prev_index] = Some(match (prev_joined, joins_previous) {
                    (true, true) => *b"medi",
                    (false, true) => *b"init",
                    (true, false) => *b"fina",
                    (false, false) => *b"isol",
                });
            }
        }
        if matches!(kind, b'D' | b'L' | b'R') {
            forms[index] = Some(if joins_previous { *b"fina" } else { *b"isol" });
        }
        previous = Some((index, kind, joins_previous));
    }
    forms
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn joining_boundaries_and_transparency() {
        for (text, expected) in [
            ("بب", vec![Some(*b"init"), Some(*b"fina")]),
            ("بَب", vec![Some(*b"init"), None, Some(*b"fina")]),
            ("بـب", vec![Some(*b"init"), None, Some(*b"fina")]),
            ("باب", vec![Some(*b"init"), Some(*b"fina"), Some(*b"isol")]),
            ("بدر", vec![Some(*b"init"), Some(*b"fina"), Some(*b"isol")]),
            ("بتب", vec![Some(*b"init"), Some(*b"medi"), Some(*b"fina")]),
            ("ب\u{200c}ب", vec![Some(*b"isol"), None, Some(*b"isol")]),
            ("ب\u{200d}ب", vec![Some(*b"init"), None, Some(*b"fina")]),
            ("ب ب", vec![Some(*b"isol"), None, Some(*b"isol")]),
            ("ب\nب", vec![Some(*b"isol"), None, Some(*b"isol")]),
            ("\u{200d}ب", vec![None, Some(*b"fina")]),
            ("ب\u{200d}", vec![Some(*b"init"), None]),
        ] {
            assert_eq!(forms(&text.chars().collect::<Vec<_>>()), expected, "{text}");
        }
    }

    #[test]
    fn transparent_edges_and_control_boundaries() {
        for text in ["َ", "ََ"] {
            assert!(forms(&text.chars().collect::<Vec<_>>())
                .iter()
                .all(Option::is_none));
        }
        assert_eq!(
            forms(&"َبَ".chars().collect::<Vec<_>>()),
            [None, Some(*b"isol"), None]
        );
        assert_eq!(
            forms(&"ب\tب".chars().collect::<Vec<_>>()),
            [Some(*b"isol"), None, Some(*b"isol")]
        );
    }

    #[test]
    fn joining_types_match_pinned_source_for_every_scalar() {
        let mut expected = vec![b'U'; 0x110000];
        for line in include_str!("../../data/unicode/17.0.0/DerivedJoiningType.txt").lines() {
            let data = line.split('#').next().unwrap().trim();
            if data.is_empty() {
                continue;
            }
            let (range, value) = data.split_once(';').unwrap();
            let bounds: Vec<_> = range.trim().split("..").collect();
            let start = u32::from_str_radix(bounds[0], 16).unwrap();
            let end = u32::from_str_radix(bounds[bounds.len() - 1], 16).unwrap();
            for cp in start..=end {
                expected[cp as usize] = value.trim().as_bytes()[0];
            }
        }
        for (cp, kind) in expected.into_iter().enumerate() {
            if let Some(ch) = char::from_u32(cp as u32) {
                assert_eq!(joining_type(ch), kind, "U+{cp:04X}");
            }
        }
    }
}
