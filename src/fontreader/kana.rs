//! Exact canonical kana pairs from Unicode 17.0.0, never prefix normalization.
use super::{Font, ResolvedGlyph};
use crate::unicode::kana_composition_data::DECOMPOSITIONS;

pub(super) fn original_pair(ch: char) -> Option<[char; 2]> {
    DECOMPOSITIONS
        .iter()
        .find(|(target, _)| *target == ch)
        .map(|(_, pair)| *pair)
}

fn composed_kana(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let pair = [chars.next()?, chars.next()?];
    if chars.next().is_some() {
        return None;
    }
    DECOMPOSITIONS
        .iter()
        .find(|(_, source)| *source == pair)
        .map(|(target, _)| *target)
}

impl Font {
    pub(super) fn is_composed_kana(glyph: &ResolvedGlyph) -> bool {
        glyph.source_end == glyph.source_start + 2
            && original_pair(glyph.shaping_ch).is_some_and(|pair| pair[0] == glyph.ch)
    }

    pub(super) fn composed_kana_glyph(
        &self,
        text: &str,
        source_start: usize,
    ) -> Option<ResolvedGlyph> {
        let shaping_ch = composed_kana(text)?;
        let glyph_id = self
            .resolve_glyph_id_with_uvs(shaping_ch, '\0', false)
            .ok()?;
        if !self.composition_glyph_is_renderable(glyph_id) {
            return None;
        }
        Some(ResolvedGlyph {
            ch: text.chars().next()?,
            shaping_ch,
            source_start,
            source_end: source_start + 2,
            glyph_id,
            prefer_color: false,
            ligature_components: 1,
            tibetan_cluster: None,
        })
    }
}

#[cfg(test)]
#[path = "kana_tests.rs"]
mod tests;
