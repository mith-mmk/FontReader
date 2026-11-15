//! Conservative, transactional prerequisite for bounded Tibetan upper stacks.
//! This is not general skipped-input GSUB: original indices and glyph storage
//! are retained, and every newly admitted substitution consumes contiguous input.
use super::*;
use crate::opentype::layouts::{coverage::Coverage, lookup::LookupSubstitution as Subtable};

// Shared by the entire opt-in transaction, never reset at a cursor or recursion.
// Charge subtable/record attempts and glyph-property probes, including misses.
const MAX_FILTERED_WORK: usize = 65_536;

#[derive(Debug)]
pub(super) struct FilteredState {
    pub(super) enabled: bool,
    pub(super) failed: bool,
    pub(super) input_bound: Option<(usize, usize)>,
    work_left: usize,
}

impl Default for FilteredState {
    fn default() -> Self {
        Self {
            enabled: false,
            failed: false,
            input_bound: None,
            work_left: MAX_FILTERED_WORK,
        }
    }
}

impl FilteredState {
    pub(super) fn fail_if_enabled(&mut self) {
        self.failed |= self.enabled;
    }
    fn charge(&mut self) -> Result<(), ()> {
        if self.failed {
            return Err(());
        }
        self.work_left = self.work_left.checked_sub(1).ok_or(())?;
        Ok(())
    }
    fn permits_consumption(&self, start: usize, end: usize) -> Result<(), ()> {
        if self
            .input_bound
            .is_some_and(|(lo, hi)| start < lo || end > hi)
        {
            Err(())
        } else {
            Ok(())
        }
    }
}

impl ApplyState<'_> {
    // Missing class/filter metadata is unknown, never equivalent to inclusion.
    fn filtered_includes(&mut self, lookup: &Lookup, glyph_id: usize) -> Result<bool, ()> {
        self.filtered.charge()?;
        let flag = lookup.lookup_flag;
        if flag & 0xFF1E == 0 {
            return Ok(true);
        }
        let gdef = self.gdef.ok_or(())?;
        let set = if flag & 0x0010 != 0 && flag & 0x0008 == 0 {
            Some(
                gdef.mark_glyph_sets_def
                    .as_ref()
                    .ok_or(())?
                    .coverages
                    .get(lookup.mark_filtering_set.ok_or(())? as usize)
                    .ok_or(())?,
            )
        } else {
            None
        };
        Ok(
            match gdef
                .glyph_class(u16::try_from(glyph_id).map_err(|_| ())?)
                .ok_or(())?
            {
                GlyphClass::Base => flag & 2 == 0,
                GlyphClass::Ligature => flag & 4 == 0,
                GlyphClass::Component => true,
                GlyphClass::Mark => {
                    if flag & 8 != 0 {
                        false
                    } else if let Some(set) = set {
                        set.contains(glyph_id).is_some()
                    } else if flag >> 8 != 0 {
                        gdef.mark_attach_class_def
                            .as_ref()
                            .ok_or(())?
                            .get_class(glyph_id as u16)
                            == flag >> 8
                    } else {
                        true
                    }
                }
            },
        )
    }
}

// Unlike get_coverage(), this is strictly the CURRENT input coverage, never
// backtrack coverage, and does not clone the context's coverage vectors.
fn first_coverage(subtable: &Subtable) -> Option<&Coverage> {
    match subtable {
        Subtable::Single(s) => Some(&s.coverage),
        Subtable::Single2(s) => Some(&s.coverage),
        Subtable::Multiple(s) => Some(&s.coverage),
        Subtable::Alternate(s) => Some(&s.coverage),
        Subtable::Ligature(s) => Some(&s.coverage),
        Subtable::ContextSubstitution(s) => Some(&s.coverage),
        Subtable::ContextSubstitution2(s) => Some(&s.coverage),
        Subtable::ContextSubstitution3(s) => s.coverages.first(),
        Subtable::ChainingContextSubstitution(s) => Some(&s.coverage),
        Subtable::ChainingContextSubstitution2(s) => Some(&s.coverage),
        Subtable::ChainingContextSubstitution3(s) => s.input_coverages.first(),
        Subtable::ReverseChainSingle(s) => Some(&s.coverage),
        _ => None,
    }
}

impl Gsub {
    pub(super) fn apply_filtered_lookup_at(
        &self,
        lookup: &Lookup,
        glyphs: &mut Vec<GsubGlyph>,
        index: usize,
        state: &mut ApplyState,
    ) -> bool {
        let result = (|| {
            let Some(glyph) = glyphs.get(index) else {
                return Ok(false);
            };
            if !state.filtered_includes(lookup, glyph.glyph_id)? {
                return Ok(false);
            }
            for subtable in &lookup.subtables {
                if self.filtered_subtable_at(lookup, subtable, glyphs, index, state, 0)? {
                    return Ok(true);
                }
            }
            Ok(false)
        })();
        match result {
            Ok(applied) => applied,
            Err(()) => {
                state.filtered.failed = true;
                false
            }
        }
    }

    fn filtered_subtable_at(
        &self,
        lookup: &Lookup,
        subtable: &Subtable,
        glyphs: &mut Vec<GsubGlyph>,
        index: usize,
        state: &mut ApplyState,
        extension_depth: usize,
    ) -> Result<bool, ()> {
        state.filtered.charge()?;
        if let Subtable::ExtensionSubstitution(extension) = subtable {
            if extension_depth >= MAX_GSUB_NESTING
                || extension.subst_format != 1
                || extension.extension_lookup_type == 7
            {
                return Err(());
            }
            return self.filtered_subtable_at(
                lookup,
                &extension.subtable,
                glyphs,
                index,
                state,
                extension_depth + 1,
            );
        }
        let glyph_id = glyphs.get(index).ok_or(())?.glyph_id;
        let coverage = first_coverage(subtable).ok_or(())?;
        let Some(coverage_index) = coverage.contains(glyph_id) else {
            return Ok(false);
        };
        match subtable {
            Subtable::Single(_) | Subtable::Single2(_) => {
                state
                    .filtered
                    .permits_consumption(index, index.checked_add(1).ok_or(())?)?;
                subtable.get_single_glyph_id(glyph_id as u16).ok_or(())?;
                Ok(Self::apply_subtable_at(subtable, glyphs, index, state))
            }
            Subtable::Ligature(ligature) => {
                let set = ligature.ligature_set.get(coverage_index).ok_or(())?;
                for record in &set.ligature_table {
                    state.filtered.charge()?;
                    let count = record.component_count as usize;
                    if count == 0 || count != record.component_glyph_ids.len() + 1 {
                        return Err(());
                    }
                    let end = index.checked_add(count).ok_or(())?;
                    // A hole is unsupported even if a later raw glyph mismatches:
                    // do not guess that a filtered earlier ligature would fail.
                    let mut matches = true;
                    for (offset, expected) in record.component_glyph_ids.iter().enumerate() {
                        let Some(glyph) = glyphs.get(index + 1 + offset) else {
                            matches = false;
                            break;
                        };
                        if !state.filtered_includes(lookup, glyph.glyph_id)? {
                            return Err(());
                        }
                        if glyph.glyph_id != *expected as usize {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        state.filtered.permits_consumption(index, end)?;
                        return Ok(Self::apply_subtable_at(subtable, glyphs, index, state));
                    }
                }
                Ok(false)
            }
            Subtable::ChainingContextSubstitution3(chaining) => {
                let count = chaining.input_coverages.len();
                if count == 0 {
                    return Err(());
                }
                // Read-only filtered matching may prove an earlier rule is a
                // genuine miss. A successful noncontiguous input match is still
                // unsupported: never execute its records or fall through.
                let mut cursor = index.checked_add(1).ok_or(())?;
                let mut contiguous = true;
                for coverage in chaining.input_coverages.iter().skip(1) {
                    loop {
                        let Some(glyph) = glyphs.get(cursor) else {
                            return Ok(false);
                        };
                        cursor = cursor.checked_add(1).ok_or(())?;
                        if !state.filtered_includes(lookup, glyph.glyph_id)? {
                            contiguous = false;
                            continue;
                        }
                        if coverage.contains(glyph.glyph_id).is_none() {
                            return Ok(false);
                        }
                        break;
                    }
                }
                if !contiguous {
                    return Err(());
                }
                let end = cursor;
                if !Self::filtered_context_matches(
                    lookup,
                    &chaining.backtrack_coverages,
                    glyphs,
                    index,
                    true,
                    state,
                )? || !Self::filtered_context_matches(
                    lookup,
                    &chaining.lookahead_coverages,
                    glyphs,
                    end,
                    false,
                    state,
                )? {
                    return Ok(false);
                }
                let records = &chaining.seq_lookup_records.lookup_records;
                if records.len() > 1 {
                    return Err(());
                }
                let old_len = glyphs.len();
                if let Some(record) = records.first() {
                    if record.sequence_index as usize >= count {
                        return Err(());
                    }
                    let previous = state.filtered.input_bound;
                    state.filtered.input_bound = Some(
                        previous.map_or((index, end), |(lo, hi)| (lo.max(index), hi.min(end))),
                    );
                    self.apply_lookup_index_at(
                        record.lookup_list_index,
                        glyphs,
                        index + record.sequence_index as usize,
                        state,
                    );
                    state.filtered.input_bound = previous;
                    if state.filtered.failed {
                        return Err(());
                    }
                }
                // Only singles/contiguous ligatures can mutate this path. A
                // single child removes solely within the proven parent span.
                let removed = old_len.checked_sub(glyphs.len()).ok_or(())?;
                state.next_index = end.checked_sub(removed).ok_or(())?;
                Ok(true) // Includes zero-record ignore rules and child no-ops.
            }
            // Coverage hit cannot prove a filtered unsupported rule is a miss.
            _ => Err(()),
        }
    }

    fn filtered_context_matches(
        lookup: &Lookup,
        coverages: &[Coverage],
        glyphs: &[GsubGlyph],
        mut cursor: usize,
        backwards: bool,
        state: &mut ApplyState,
    ) -> Result<bool, ()> {
        for coverage in coverages {
            loop {
                let position = if backwards {
                    let Some(previous) = cursor.checked_sub(1) else {
                        return Ok(false);
                    };
                    cursor = previous;
                    cursor
                } else {
                    let position = cursor;
                    cursor = cursor.checked_add(1).ok_or(())?;
                    position
                };
                let Some(glyph) = glyphs.get(position) else {
                    return Ok(false);
                };
                if !state.filtered_includes(lookup, glyph.glyph_id)? {
                    continue;
                }
                if coverage.contains(glyph.glyph_id).is_none() {
                    return Ok(false);
                }
                break;
            }
        }
        Ok(true)
    }

    pub(super) fn apply_filtered_transaction_lookup(
        &self,
        lookup: &Lookup,
        glyphs: &mut Vec<GsubGlyph>,
        state: &mut ApplyState,
    ) {
        let reverse = lookup.lookup_type == LookupType::ReverseChainingContextualSingleSubstitution as u16
            || (lookup.lookup_type == LookupType::ExtensionSubstitution as u16 && lookup.subtables.iter().any(|subtable| matches!(subtable,
                Subtable::ExtensionSubstitution(extension) if extension.extension_lookup_type == LookupType::ReverseChainingContextualSingleSubstitution as u16)));
        if reverse {
            for index in (0..glyphs.len()).rev() {
                state.remaining = MAX_GSUB_LOOKUP_APPLICATIONS;
                self.dispatch_lookup_at(lookup, glyphs, index, state);
                if state.filtered.failed {
                    break;
                }
            }
            return;
        }
        let mut index = 0;
        while index < glyphs.len() && !state.filtered.failed {
            state.remaining = MAX_GSUB_LOOKUP_APPLICATIONS;
            let old_len = glyphs.len();
            let old_index = index;
            let applied = self.dispatch_lookup_at(lookup, glyphs, index, state);
            index = if applied { state.next_index } else { index + 1 };
            if applied && index <= old_index && glyphs.len() >= old_len {
                index = old_index + 1;
            }
        }
    }
}

#[cfg(test)]
mod tests;
