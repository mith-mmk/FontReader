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
