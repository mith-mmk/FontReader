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
            kana_residual_mark: false,
            simple_mkmk_cluster: None,
            tibetan_upper_anchor: None,
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

// Deliberately exact: no selectors, joiners, repeated marks, or arbitrary Mn.
// Extended kana letters used by Ainu are included; spacing voiced marks are not.
pub(super) fn residual_mark(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let base = chars.next()?;
    let mark = chars.next()?;
    (matches!(base, '\u{3041}'..='\u{3096}' | '\u{309d}'..='\u{309f}'
        | '\u{30a1}'..='\u{30fa}' | '\u{30fd}'..='\u{30ff}' | '\u{31f0}'..='\u{31ff}')
        && matches!(mark, '\u{3099}' | '\u{309a}')
        && chars.next().is_none())
    .then_some(mark)
}

impl Font {
    pub(super) fn zero_residual_kana_advance(
        &self,
        glyph: &ResolvedGlyph,
        locale: Option<&str>,
        vertical: bool,
    ) -> bool {
        if !glyph.kana_residual_mark
            || !self.composition_glyph_is_renderable(glyph.glyph_id)
            || self
                .current_gdef()
                .is_some_and(|gdef| gdef.glyph_class_def.is_some())
        {
            return false;
        }
        // This fallback keeps nominal positions, rather than inventing anchors.
        // Faces without GPOS need a separate fallback-positioning policy.
        let Some(gpos) = self.current_gpos() else {
            return false;
        };
        // Even a zero-valued authored record wins over our fallback.
        gpos.lookup_single_adjustment(glyph.glyph_id as u16, vertical, locale, self.current_gdef())
            .is_none()
    }
}
