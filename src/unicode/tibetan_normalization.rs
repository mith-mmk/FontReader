//! Canonical-only Tibetan properties, pinned to Unicode 17.0.0.
//! Not a general normalizer, compatibility mapping, or HarfBuzz modified CCC.
use super::tibetan_normalization_data::{COMBINING_CLASSES, DECOMPOSITIONS};

pub(crate) fn decomposition(ch: char) -> Option<&'static [char; 2]> {
    DECOMPOSITIONS
        .binary_search_by_key(&ch, |&(cp, _)| cp)
        .ok()
        .map(|index| &DECOMPOSITIONS[index].1)
}

pub(crate) fn combining_class(ch: char) -> u8 {
    COMBINING_CLASSES
        .binary_search_by_key(&ch, |&(cp, _)| cp)
        .map(|index| COMBINING_CLASSES[index].1)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "tibetan_normalization_tests.rs"]
mod tests;
