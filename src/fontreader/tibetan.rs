//! Bounded Tibetan canonical normalization and transactional stack shaping.
use super::{Font, ResolvedGlyph, TextRunMode};
use crate::commands::FontVariant;
use crate::opentype::extentions::gsub::GsubGlyph;
use crate::unicode::tibetan_normalization::{combining_class, decomposition};

impl Font {
    pub(super) fn is_tibetan_normalization_cluster(text: &str) -> bool {
        // Selectors, join controls, and non-Tibetan combining marks cannot enter
        // normalization. Original grapheme boundaries remain authoritative.
        !text.is_empty() && text.chars().all(|ch| ('\u{f00}'..='\u{fff}').contains(&ch))
    }

    fn is_bounded_tibetan_stack(glyphs: &[ResolvedGlyph]) -> bool {
        let mut chars = glyphs.iter().map(|g| g.shaping_ch);
        let Some(base) = chars.next() else {
            return false;
        };
        if !matches!(base, '\u{f40}'..='\u{f47}' | '\u{f49}'..='\u{f6c}')
            || decomposition(base).is_some()
        {
            return false;
        }
        let mut subjoined = false;
        let mut vowel = None;
        for ch in chars {
            if let Some(previous) = vowel {
                // Only AA+U is newly admitted. AA+above marks and upper vowels
                // on ligated stacks need attachment handling not provided here.
                if previous == '\u{f71}' && ch == '\u{f74}' {
                    vowel = Some(ch);
                    continue;
                }
                return false;
            }
            if matches!(ch, '\u{f90}'..='\u{f97}' | '\u{f99}'..='\u{fbc}')
                && decomposition(ch).is_none()
            {
                subjoined = true;
            } else if matches!(ch, '\u{f71}' | '\u{f74}')
                || (!subjoined && matches!(ch, '\u{f72}' | '\u{f7a}'..='\u{f7d}' | '\u{f80}'))
            {
                vowel = Some(ch);
            } else {
                return false;
            }
        }
        true
    }

    fn normalized_tibetan_glyphs(&self, original: &[ResolvedGlyph]) -> Option<Vec<ResolvedGlyph>> {
        if original.is_empty() || original.iter().any(|g| g.tibetan_cluster.is_none()) {
            return None;
        }
        let mut normalized = Vec::new();
        let mut start = 0;
        while start < original.len() {
            let mut end = start + 1;
            while end < original.len()
                && original[end].tibetan_cluster == original[start].tibetan_cluster
            {
                end += 1;
            }
            let cluster_start = normalized.len();
            for glyph in &original[start..end] {
                if let Some(parts) = decomposition(glyph.shaping_ch) {
                    // All 17 pinned mappings are terminal pairs. Never replace
                    // an available compound by missing/non-renderable components.
                    let ids = parts.map(|ch| self.resolve_glyph_id_with_uvs(ch, '\0', false).ok());
                    if let [Some(first), Some(second)] = ids {
                        if [first, second]
                            .iter()
                            .all(|&id| self.resolved_glyph_support(id).is_supported())
                        {
                            for (ch, id) in parts.iter().zip([first, second]) {
                                normalized.push(ResolvedGlyph {
                                    kana_residual_mark: false,
                                    simple_mkmk_cluster: None,
                                    shaping_ch: *ch,
                                    glyph_id: id,
                                    ..*glyph
                                });
                            }
                            continue;
                        }
                    }
                }
                normalized.push(*glyph);
            }
            // The admitted grammar has at most two nonzero-CCC marks. Reject
            // longer untrusted mark sequences before the bounded insertion sort.
            if normalized[cluster_start..]
                .iter()
                .filter(|g| combining_class(g.shaping_ch) != 0)
                .take(3)
                .count()
                > 2
            {
                return None;
            }
            // Stable canonical ordering stops at CCC 0, including subjoined
            // letters. Move the complete glyph/provenance record with each mark.
            for index in cluster_start + 1..normalized.len() {
                let class = combining_class(normalized[index].shaping_ch);
                if class == 0 {
                    continue;
                }
                let mut cursor = index;
                while cursor > cluster_start {
                    let previous = combining_class(normalized[cursor - 1].shaping_ch);
                    if previous == 0 || previous <= class {
                        break;
                    }
                    normalized.swap(cursor - 1, cursor);
                    cursor -= 1;
                }
            }
            if !Self::is_bounded_tibetan_stack(&normalized[cluster_start..]) {
                return None;
            }
            start = end;
        }
        normalized
            .iter()
            .all(|g| g.glyph_id != 0)
            .then_some(normalized)
    }

    pub(super) fn try_tibetan_stack_run(
        &self,
        original: &[ResolvedGlyph],
        locale: Option<&str>,
        variant: FontVariant,
        orientation: Option<TextRunMode>,
    ) -> Option<Vec<ResolvedGlyph>> {
        let gsub = self.current_gsub()?;
        if !gsub.has_tibetan_stack_features(locale) {
            return None;
        }
        let glyphs = self.normalized_tibetan_glyphs(original)?;
        let normalization_changed = glyphs.len() != original.len()
            || glyphs
                .iter()
                .zip(original)
                .any(|(a, b)| a.shaping_ch != b.shaping_ch || a.source_start != b.source_start);
        // AA+U is new even when the source is already canonically ordered.
        let new_lower_vowel_tail = glyphs.windows(2).any(|pair| {
            pair[0].tibetan_cluster == pair[1].tibetan_cluster
                && pair[0].shaping_ch == '\u{f71}'
                && pair[1].shaping_ch == '\u{f74}'
        });
        if !glyphs
            .windows(2)
            .any(|p| p[0].tibetan_cluster == p[1].tibetan_cluster)
        {
            return None;
        }
        let working: Vec<_> = glyphs
            .iter()
            .enumerate()
            .map(|(source_index, glyph)| GsubGlyph {
                glyph_id: glyph.glyph_id,
                source_index,
                substituted: false,
                ligature_components: 1,
            })
            .collect();
        let mut candidate = Vec::new();
        let mut start = 0;
        while start < glyphs.len() {
            let mut end = start + 1;
            while end < glyphs.len() && glyphs[end].tibetan_cluster == glyphs[start].tibetan_cluster
            {
                end += 1;
            }
            let mut cluster = working[start..end].to_vec();
            if !gsub.preprocess_tibetan_stack(&mut cluster, locale, self.current_gdef()) {
                return None;
            }
            candidate.extend(cluster);
            start = end;
        }
        let orientation_tag = orientation.map(|mode| match mode {
            TextRunMode::SidewaysLtr => *b"vrtr",
            TextRunMode::MixedUpright => gsub.selected_vertical_tag(true),
            TextRunMode::Standard => gsub.selected_vertical_tag(false),
        });
        if !gsub.apply_tibetan_stack_presentation(
            &mut candidate,
            locale,
            variant,
            orientation_tag,
            self.current_gdef(),
        ) {
            return None;
        }
        // Validate exact normalized component ownership, then exact original
        // scalar ownership. A decomposed scalar may not be split across outputs.
        let mut next_normalized = 0;
        let mut next_original = 0;
        let mut result = Vec::new();
        for item in candidate {
            // Preserve an available original if normalization would expose a
            // missing or unrenderable substituted component. Unchanged PR39
            // candidates retain their established fallback behavior.
            if (normalization_changed || new_lower_vowel_tail)
                && !self.resolved_glyph_support(item.glyph_id).is_supported()
            {
                return None;
            }
            if item.source_index != next_normalized {
                return None;
            }
            let end = next_normalized.checked_add(item.ligature_components as usize)?;
            let sources = glyphs.get(next_normalized..end)?;
            let first = sources.first()?;
            if sources
                .iter()
                .any(|g| g.tibetan_cluster != first.tibetan_cluster)
                || matches!(
                    first.shaping_ch,
                    '\u{f71}' | '\u{f74}' | '\u{f90}'..='\u{fbc}'
                )
            {
                return None;
            }
            let mut owners: Vec<_> = sources.iter().map(|g| g.source_start).collect();
            owners.sort_unstable();
            owners.dedup();
            if owners
                .iter()
                .copied()
                .ne(next_original..next_original + owners.len())
            {
                return None;
            }
            let original_end = next_original + owners.len();
            if sources.iter().any(|g| g.source_end != g.source_start + 1) {
                return None;
            }
            result.push(ResolvedGlyph {
                kana_residual_mark: false,
                simple_mkmk_cluster: None,
                ch: original.get(next_original)?.ch,
                shaping_ch: first.shaping_ch,
                glyph_id: item.glyph_id,
                prefer_color: first.prefer_color,
                ligature_components: item.ligature_components,
                tibetan_cluster: first.tibetan_cluster,
                source_start: next_original,
                source_end: original_end,
            });
            next_original = original_end;
            next_normalized = end;
        }
        if next_normalized != glyphs.len() || next_original != original.len() {
            return None;
        }
        Some(result)
    }
}

#[cfg(test)]
#[path = "tibetan_tests.rs"]
mod tests;
