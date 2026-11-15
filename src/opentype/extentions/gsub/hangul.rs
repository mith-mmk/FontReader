//! Transactional, one-to-one Old Hangul GSUB. Not a general masked interpreter.
use super::{Gsub, GsubGlyph};
use crate::opentype::layouts::lookup::{Lookup, LookupSubstitution as Sub};

impl Gsub {
    pub(crate) fn bounded_hangul(
        &self,
        input: &[GsubGlyph],
        masks: &[u8],
        locale: Option<&str>,
    ) -> Option<Vec<GsubGlyph>> {
        if self.feature_variations.is_some() {
            return None;
        }
        if input.is_empty() || input.len() > 256 || input.len() != masks.len() {
            return None;
        }
        let script = self
            .scripts
            .scripts
            .iter()
            .find(|s| s.script_tag == u32::from_be_bytes(*b"hang"))?;
        let requested = locale
            .map(Self::locale_to_language_system_tags)
            .unwrap_or_default();
        let system = requested
            .iter()
            .find_map(|tag| {
                script
                    .language_systems
                    .iter()
                    .find(|s| s.language_system_tag == *tag)
            })
            .or_else(|| {
                script
                    .language_systems
                    .iter()
                    .find(|s| s.language_system_tag == 0)
            })?;
        // Unknown required-feature semantics are outside this bounded plan.
        if system.language_system.required_feature_index != 0xffff {
            return None;
        }
        let mut selected = vec![0u8; self.lookups.lookups.len()];
        let mut has_jmo = false;
        for fi in &system.language_system.feature_indexes {
            let feature = self.features.features.get(*fi as usize)?;
            if [
                *b"rvrn", *b"ltra", *b"ltrm", *b"abvm", *b"blwm", *b"mark", *b"mkmk", *b"curs",
                *b"dist", *b"kern",
            ]
            .contains(&feature.feature_tag.to_be_bytes())
            {
                return None;
            }
            let mask = match feature.feature_tag.to_be_bytes() {
                [b'l', b'j', b'm', b'o'] => {
                    has_jmo = true;
                    1
                }
                [b'v', b'j', b'm', b'o'] => {
                    has_jmo = true;
                    2
                }
                [b't', b'j', b'm', b'o'] => {
                    has_jmo = true;
                    4
                }
                tag if [*b"locl", *b"ccmp", *b"rlig", *b"rclt", *b"clig", *b"liga"]
                    .contains(&tag) =>
                {
                    7
                }
                // HarfBuzz disables calt on Jamo, not on unrelated text.
                _ => continue,
            };
            for li in &feature.lookup_list_indices {
                *selected.get_mut(*li as usize)? |= mask;
            }
        }
        if !has_jmo {
            return None;
        }
        let mut result = input.to_vec();
        let mut budget = 16384usize;
        for (lookup, mask) in self.lookups.lookups.iter().zip(selected) {
            if mask == 0 {
                continue;
            }
            for (i, source_mask) in masks.iter().enumerate() {
                if source_mask & mask == 0 {
                    continue;
                }
                self.hangul_lookup(lookup, &mut result, i, &mut budget, false)
                    .ok()?;
            }
        }
        Some(result)
    }

    fn hangul_lookup(
        &self,
        lookup: &Lookup,
        glyphs: &mut [GsubGlyph],
        i: usize,
        budget: &mut usize,
        nested: bool,
    ) -> Result<bool, ()> {
        *budget = budget.checked_sub(1).ok_or(())?;
        // No filtering, skipping, reverse traversal or unresolved flag semantics.
        if lookup.lookup_flag != 0 || lookup.mark_filtering_set.is_some() {
            return Err(());
        }
        for sub in &lookup.subtables {
            if self.hangul_subtable(sub, glyphs, i, budget, nested, false)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn hangul_subtable(
        &self,
        sub: &Sub,
        glyphs: &mut [GsubGlyph],
        i: usize,
        budget: &mut usize,
        nested: bool,
        extension: bool,
    ) -> Result<bool, ()> {
        *budget = budget.checked_sub(1).ok_or(())?;
        let gid = glyphs[i].glyph_id;
        match sub {
            Sub::Single(_) | Sub::Single2(_) => {
                if sub.get_coverage().0.contains(gid).is_none() {
                    return Ok(false);
                }
                let replacement = sub
                    .get_single_glyph_id(u16::try_from(gid).map_err(|_| ())?)
                    .ok_or(())?;
                glyphs[i].glyph_id = replacement as usize;
                glyphs[i].substituted = true;
                glyphs[i].mkmk_source_ambiguous = true;
                Ok(true)
            }
            Sub::ExtensionSubstitution(e) => {
                if extension || e.subst_format != 1 || e.extension_lookup_type != 1 {
                    return Err(());
                }
                if !matches!(e.subtable.as_ref(), Sub::Single(_) | Sub::Single2(_)) {
                    return Err(());
                }
                self.hangul_subtable(&e.subtable, glyphs, i, budget, nested, true)
            }
            Sub::ChainingContextSubstitution3(c) if !nested => {
                // Lookaround is context only, never a target/category requirement.
                if !Self::matches_backtrack_coverages(&c.backtrack_coverages, glyphs, i)
                    || !Self::matches_input_coverages(&c.input_coverages, glyphs, i)
                    || !Self::matches_lookahead_coverages(
                        &c.lookahead_coverages,
                        glyphs,
                        i + c.input_coverages.len(),
                    )
                {
                    return Ok(false);
                }
                if c.input_coverages.len() != 1 || c.seq_lookup_records.lookup_records.len() != 1 {
                    return Err(());
                }
                let record = &c.seq_lookup_records.lookup_records[0];
                if record.sequence_index != 0 {
                    return Err(());
                }
                let lookup = self
                    .lookups
                    .lookups
                    .get(record.lookup_list_index as usize)
                    .ok_or(())?;
                self.hangul_lookup(lookup, glyphs, i, budget, true)?;
                Ok(true)
            }
            // ccmp ligatures must remain on the legacy path when they match.
            Sub::Ligature(l) if !nested => {
                let Some(ci) = l.coverage.contains(gid) else {
                    return Ok(false);
                };
                let set = l.ligature_set.get(ci).ok_or(())?;
                for record in &set.ligature_table {
                    let count = record.component_count as usize;
                    if count == 0 || record.component_glyph_ids.len() + 1 != count {
                        return Err(());
                    }
                    if let Some(following) = glyphs.get(i + 1..i.saturating_add(count)) {
                        if following
                            .iter()
                            .map(|g| g.glyph_id)
                            .eq(record.component_glyph_ids.iter().map(|g| *g as usize))
                        {
                            return Err(());
                        }
                    }
                }
                Ok(false)
            }
            Sub::Unknown => Err(()),
            // Only prove no-match for unsupported forms with an unambiguous first-input coverage.
            Sub::ChainingContextSubstitution3(c) => {
                if c.input_coverages
                    .first()
                    .is_some_and(|coverage| coverage.contains(gid).is_none())
                {
                    Ok(false)
                } else {
                    Err(())
                }
            }
            _ => {
                if sub.get_coverage().0.contains(gid).is_none() {
                    Ok(false)
                } else {
                    Err(())
                }
            }
        }
    }
}
