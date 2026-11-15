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

    pub(super) fn hangul_glyph_is_renderable(&self, glyph_id: usize) -> bool {
        // The generic coverage probe accepts any CFF ID. A newly selected cmap
        // entry must not index past CFF/CFF2 CharStrings during outline export.
        #[cfg(feature = "cff")]
        if let Some(cff) = self.current_cff() {
            if glyph_id >= cff.char_string.data.data.len() {
                return false;
            }
        }
        self.resolved_glyph_support(glyph_id).is_supported()
    }

    pub(super) fn restore_renderable_hangul_sources(
        &self,
        glyphs: &[ResolvedGlyph],
    ) -> Option<Vec<ResolvedGlyph>> {
        let mut restored = Vec::new();
        let mut changed = false;
        for glyph in glyphs {
            let source_len = glyph.source_end - glyph.source_start;
            let syllable = glyph.shaping_ch as u32;
            if !Self::is_composed_modern_hangul(glyph) {
                restored.push(*glyph);
                continue;
            }
            let offset = syllable - 0xac00;
            let mut chars = vec![glyph.ch];
            if ('\u{1100}'..='\u{1112}').contains(&glyph.ch) {
                chars.push(char::from_u32(0x1161 + (offset % 588) / 28).unwrap());
            }
            if !offset.is_multiple_of(28) {
                chars.push(char::from_u32(0x11a7 + offset % 28).unwrap());
            }
            if chars.len() != source_len {
                restored.push(*glyph);
                continue;
            }
            let original: Option<Vec<_>> = chars
                .into_iter()
                .enumerate()
                .map(|(index, ch)| {
                    let glyph_id = self.resolve_glyph_id_with_uvs(ch, '\0', false).ok()?;
                    self.hangul_glyph_is_renderable(glyph_id)
                        .then_some(ResolvedGlyph {
                            ch,
                            shaping_ch: ch,
                            source_start: glyph.source_start + index,
                            source_end: glyph.source_start + index + 1,
                            glyph_id,
                            ..*glyph
                        })
                })
                .collect();
            if let Some(original) = original {
                restored.extend(original);
                changed = true;
            } else {
                // Preserve composed-only coverage even when a neighboring
                // syllable must restore its usable original jamo.
                restored.push(*glyph);
            }
        }
        changed.then_some(restored)
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
        if !self.hangul_glyph_is_renderable(glyph_id) {
            // Preserve the entire original cluster, not a supported LV prefix.
            return None;
        }
        Some(ResolvedGlyph {
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
