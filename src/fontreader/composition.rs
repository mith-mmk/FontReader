//! Shared coverage and one bounded rollback for Hangul and kana composition.
use super::{Font, ResolvedGlyph};

impl Font {
    pub(super) fn is_bounded_composition(glyph: &ResolvedGlyph) -> bool {
        Self::is_composed_modern_hangul(glyph) || Self::is_composed_kana(glyph)
    }

    pub(super) fn composition_glyph_is_renderable(&self, glyph_id: usize) -> bool {
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

    pub(super) fn restore_renderable_composition_sources(
        &self,
        glyphs: &[ResolvedGlyph],
    ) -> Option<Vec<ResolvedGlyph>> {
        let mut restored = Vec::new();
        let mut changed = false;
        for glyph in glyphs {
            let source_len = glyph.source_end - glyph.source_start;
            let syllable = glyph.shaping_ch as u32;
            if !Self::is_bounded_composition(glyph) {
                restored.push(*glyph);
                continue;
            }
            let chars = if Self::is_composed_modern_hangul(glyph) {
                let offset = syllable - 0xac00;
                let mut chars = vec![glyph.ch];
                if ('\u{1100}'..='\u{1112}').contains(&glyph.ch) {
                    chars.push(char::from_u32(0x1161 + (offset % 588) / 28).unwrap());
                }
                if !offset.is_multiple_of(28) {
                    chars.push(char::from_u32(0x11a7 + offset % 28).unwrap());
                }
                chars
            } else {
                super::kana::original_pair(glyph.shaping_ch)
                    .unwrap()
                    .to_vec()
            };
            if chars.len() != source_len {
                restored.push(*glyph);
                continue;
            }
            let original: Option<Vec<_>> = chars
                .into_iter()
                .enumerate()
                .map(|(index, ch)| {
                    let glyph_id = self.resolve_glyph_id_with_uvs(ch, '\0', false).ok()?;
                    self.composition_glyph_is_renderable(glyph_id)
                        .then_some(ResolvedGlyph {
                            simple_mkmk_cluster: None,
                            tibetan_upper_anchor: None,
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
                // cluster must restore its usable original scalars.
                restored.push(*glyph);
            }
        }
        changed.then_some(restored)
    }
}
