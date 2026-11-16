//! Font-aware canonical composition of complete, bounded modern Hangul EGCs.
use super::{Font, ResolvedGlyph};

// Unicode 17.0, section 3.12.3: algorithmic modern Hangul composition.
// This intentionally recognizes only complete L V [T] and LV T graphemes,
// not prefixes, compatibility jamo, fillers, or general normalization.
fn modern_hangul_syllable(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let first = chars.next()? as u32;
    let second = chars.next()? as u32;
    let syllable = if (0x1100..=0x1112).contains(&first) && (0x1161..=0x1175).contains(&second) {
        let lv = 0xac00 + ((first - 0x1100) * 21 + second - 0x1161) * 28;
        match chars.next() {
            None => lv,
            Some(trailing) if ('\u{11a8}'..='\u{11c2}').contains(&trailing) => {
                lv + trailing as u32 - 0x11a7
            }
            _ => return None,
        }
    } else if (0xac00..=0xd7a3).contains(&first)
        && (first - 0xac00).is_multiple_of(28)
        && (0x11a8..=0x11c2).contains(&second)
    {
        first + second - 0x11a7
    } else {
        return None;
    };
    // Do not compose an LV prefix of old/repeated jamo or any marked cluster.
    if chars.next().is_some() {
        return None;
    }
    char::from_u32(syllable)
}

impl Font {
    pub(super) fn is_composed_modern_hangul(glyph: &ResolvedGlyph) -> bool {
        glyph.source_end > glyph.source_start + 1
            && ('\u{ac00}'..='\u{d7a3}').contains(&glyph.shaping_ch)
            && glyph.ch != glyph.shaping_ch
    }

    pub(super) fn composed_modern_hangul_glyph(
        &self,
        text: &str,
        source_start: usize,
    ) -> Option<ResolvedGlyph> {
        let shaping_ch = modern_hangul_syllable(text)?;
        let glyph_id = self
            .resolve_glyph_id_with_uvs(shaping_ch, '\0', false)
            .ok()?;
        if !self.composition_glyph_is_renderable(glyph_id) {
            // Preserve the entire original cluster, not a supported LV prefix.
            return None;
        }
        Some(ResolvedGlyph {
            kana_residual_mark: false,
            simple_mkmk_cluster: None,
            tibetan_upper_anchor: None,
            ch: text.chars().next()?,
            shaping_ch,
            source_start,
            source_end: source_start + text.chars().count(),
            glyph_id,
            prefer_color: false,
            // Unicode composition is one nominal glyph, not a GSUB ligature.
            ligature_components: 1,
            tibetan_cluster: None,
        })
    }
}

#[cfg(test)]
#[path = "hangul_tests.rs"]
mod tests;

#[cfg(feature = "layout")]
fn jamo_mask(ch: char) -> Option<u8> {
    match ch as u32 {
        0x1100..=0x115e | 0xa960..=0xa97c => Some(1),
        0x1161..=0x11a7 | 0xd7b0..=0xd7c6 => Some(2),
        0x11a8..=0x11ff | 0xd7cb..=0xd7fb => Some(4),
        _ => None,
    }
}

#[cfg(feature = "layout")]
impl Font {
    pub(super) fn is_old_hangul_text(text: &str) -> bool {
        let chars: Vec<_> = text.chars().collect();
        matches!(chars.len(), 2 | 3)
            && jamo_mask(chars[0]) == Some(1)
            && jamo_mask(chars[1]) == Some(2)
            && (chars.len() == 2 || jamo_mask(chars[2]) == Some(4))
            && chars.iter().any(
                |ch| !matches!(*ch as u32, 0x1100..=0x1112 | 0x1161..=0x1175 | 0x11a8..=0x11c2),
            )
    }

    pub(super) fn try_old_hangul_run(
        &self,
        glyphs: &[ResolvedGlyph],
        locale: Option<&str>,
        orientation_tag: Option<[u8; 4]>,
    ) -> Option<Vec<ResolvedGlyph>> {
        if glyphs.is_empty() || glyphs.len() > 256 {
            return None;
        }
        let mut masks = Vec::with_capacity(glyphs.len());
        let mut start = 0;
        let mut old = false;
        while start < glyphs.len() {
            let cluster = glyphs[start].simple_mkmk_cluster?;
            let end = start
                + glyphs[start..]
                    .iter()
                    .take_while(|g| g.simple_mkmk_cluster == Some(cluster))
                    .count();
            let group = &glyphs[start..end];
            if !Self::is_old_hangul_text(&group.iter().map(|g| g.ch).collect::<String>()) {
                return None;
            }
            for (offset, g) in group.iter().enumerate() {
                let mask = jamo_mask(g.ch)?;
                if mask != [1, 2, 4][offset]
                    || g.ch != g.shaping_ch
                    || g.source_end != g.source_start + 1
                    || g.ligature_components != 1
                    || !self.composition_glyph_is_renderable(g.glyph_id)
                {
                    return None;
                }
                if start + offset > 0 && glyphs[start + offset - 1].source_end != g.source_start {
                    return None;
                }
                old |= !matches!(g.ch as u32, 0x1100..=0x1112 | 0x1161..=0x1175 | 0x11a8..=0x11c2);
                masks.push(mask);
            }
            start = end;
        }
        if !old {
            return None;
        }
        let input: Vec<_> = glyphs
            .iter()
            .enumerate()
            .map(
                |(source_index, g)| crate::opentype::extentions::gsub::GsubGlyph {
                    glyph_id: g.glyph_id,
                    source_index,
                    substituted: false,
                    mkmk_source_ambiguous: false,
                    ligature_components: 1,
                },
            )
            .collect();
        let shaped =
            self.current_gsub()?
                .bounded_hangul(&input, &masks, locale, orientation_tag)?;
        let mut result = glyphs.to_vec();
        for (i, (out, g)) in result.iter_mut().zip(shaped).enumerate() {
            if g.source_index != i
                || g.ligature_components != 1
                || !self.composition_glyph_is_renderable(g.glyph_id)
            {
                return None;
            }
            out.glyph_id = g.glyph_id;
            // One-to-one source ownership is proven, but mark-class preservation is not.
            out.simple_mkmk_cluster = None;
        }
        Some(result)
    }
}
