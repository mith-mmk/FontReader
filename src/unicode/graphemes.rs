//! Internal Unicode 17.0.0 extended grapheme boundaries (UAX #29).
//!
//! This is independent of the Unicode 18.0.0 vertical-orientation property.
//! Layout controls and orphan variation selectors are handled by the caller.

use unicode_segmentation::{Graphemes, UnicodeSegmentation};

pub(crate) fn extended_graphemes(text: &str) -> Graphemes<'_> {
    text.graphemes(true)
}

#[cfg(test)]
mod tests {
    use super::extended_graphemes;

    const SOURCE: &str = include_str!("../../data/unicode/17.0.0/GraphemeBreakTest.txt");

    #[test]
    fn every_official_unicode_17_extended_grapheme_test_matches() {
        assert_eq!(unicode_segmentation::UNICODE_VERSION, (17, 0, 0));
        assert!(SOURCE.starts_with("# GraphemeBreakTest-17.0.0.txt\n"));
        let mut cases = 0;
        for (line_index, line) in SOURCE.lines().enumerate() {
            let data = line.split('#').next().unwrap().trim();
            if data.is_empty() {
                continue;
            }
            let fields: Vec<_> = data.split_whitespace().collect();
            assert_eq!(fields.len() % 2, 1);
            assert_eq!(fields.first(), Some(&"÷"));
            assert_eq!(fields.last(), Some(&"÷"));
            let mut text = String::new();
            let mut boundaries = Vec::new();
            for (index, field) in fields.iter().enumerate() {
                if index % 2 == 0 {
                    match *field {
                        "÷" => boundaries.push(text.len()),
                        "×" => (),
                        _ => panic!("invalid boundary at line {}", line_index + 1),
                    }
                } else {
                    let scalar = u32::from_str_radix(field, 16).unwrap();
                    text.push(char::from_u32(scalar).expect("official test scalar"));
                }
            }
            let expected: Vec<_> = boundaries
                .windows(2)
                .map(|pair| &text[pair[0]..pair[1]])
                .collect();
            let actual: Vec<_> = extended_graphemes(&text).collect();
            assert_eq!(actual, expected, "official line {}: {line}", line_index + 1);
            let reverse: Vec<_> = extended_graphemes(&text).rev().collect();
            assert_eq!(reverse, expected.iter().rev().copied().collect::<Vec<_>>());
            assert_eq!(actual.concat(), text);
            cases += 1;
        }
        assert_eq!(cases, 766, "do not silently truncate the official fixture");
        assert_eq!(extended_graphemes("").count(), 0);
    }
}
