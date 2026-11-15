use super::{combining_class, decomposition};

#[test]
fn every_tibetan_property_matches_the_independent_unicode_17_source() {
    let source = include_str!("../../data/unicode/17.0.0/UnicodeData.txt");
    let mut expected = std::collections::BTreeMap::new();
    for line in source.lines() {
        let fields: Vec<_> = line.split(';').collect();
        let cp = u32::from_str_radix(fields[0], 16).unwrap();
        if !(0xf00..=0xfff).contains(&cp) {
            continue;
        }
        let mapping = if fields[5].is_empty() || fields[5].starts_with('<') {
            None
        } else {
            Some(
                fields[5]
                    .split_whitespace()
                    .map(|value| char::from_u32(u32::from_str_radix(value, 16).unwrap()).unwrap())
                    .collect::<Vec<_>>(),
            )
        };
        assert!(expected
            .insert(
                char::from_u32(cp).unwrap(),
                (fields[3].parse::<u8>().unwrap(), mapping)
            )
            .is_none());
    }
    assert_eq!(expected.len(), 211);
    let mut mappings = 0;
    let mut classes = 0;
    for cp in 0xf00..=0xfff {
        let ch = char::from_u32(cp).unwrap();
        let (class, mapping) = expected.get(&ch).cloned().unwrap_or_default();
        assert_eq!(combining_class(ch), class, "U+{cp:04X}");
        assert_eq!(decomposition(ch).map(|d| d.to_vec()), mapping, "U+{cp:04X}");
        mappings += usize::from(mapping.is_some());
        classes += usize::from(class != 0);
        // The pinned Tibetan mappings have no recursive decomposition.
        if let Some(parts) = decomposition(ch) {
            assert!(parts.iter().all(|part| decomposition(*part).is_none()));
        }
    }
    assert_eq!((mappings, classes), (17, 19));
    for ch in ['\u{f77}', '\u{f79}', '\u{f0c}', 'é', '\u{fb01}'] {
        assert!(decomposition(ch).is_none());
    }
    assert_eq!(combining_class('\u{f39}'), 216);
    assert_eq!(combining_class('\u{fc6}'), 220);
    assert_eq!(combining_class('\u{fb7}'), 0);
}
