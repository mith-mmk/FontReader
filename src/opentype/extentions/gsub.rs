#![allow(dead_code)]

// GSUB -- Glyph Substitution Table

// only check lookup Format1.1 , 4, 5, 6.1, 7

use std::collections::HashSet;
use std::io::SeekFrom;

use crate::opentype::layouts::{
    feature::Feature,
    lookup::{Lookup, LookupResult},
    script::ParsedScript,
    *,
};
use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct JoiningForms {
    pub(crate) isolated: Option<usize>,
    pub(crate) initial: Option<usize>,
    pub(crate) medial: Option<usize>,
    pub(crate) final_form: Option<usize>,
}

impl JoiningForms {
    pub(crate) fn can_join_to_prev(self) -> bool {
        self.final_form.is_some() || self.medial.is_some()
    }

    pub(crate) fn can_join_to_next(self) -> bool {
        self.initial.is_some() || self.medial.is_some()
    }

    pub(crate) fn substitute(self, glyph_id: usize, join_prev: bool, join_next: bool) -> usize {
        if join_prev && join_next {
            if let Some(glyph_id) = self.medial {
                return glyph_id;
            }
        }
        if join_prev {
            if let Some(glyph_id) = self.final_form {
                return glyph_id;
            }
        }
        if join_next {
            if let Some(glyph_id) = self.initial {
                return glyph_id;
            }
        }

        self.isolated.unwrap_or(glyph_id)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GSUB {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) scripts: Box<ScriptList>,
    pub(crate) features: Box<FeatureList>,
    pub(crate) lookups: Box<LookupList>,
    // version 1.1
    pub(crate) feature_variations: Option<Box<FeatureVariationList>>,
}

impl GSUB {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, std::io::Error> {
        let offset = offset as u64;

        reader.seek(SeekFrom::Start(offset as u64))?;
        let major_version = reader.read_u16_be()?;
        let minor_version = reader.read_u16_be()?;
        let script_list_offset = reader.read_u16_be()?;
        let feature_list_offset = reader.read_u16_be()?;
        let lookup_list_offset = reader.read_u16_be()?;
        let feature_variations_offset = if major_version == 1 && minor_version == 1 {
            reader.read_u16_be()?
        } else {
            0
        };

        let scripts = Box::new(ScriptList::new(
            reader,
            offset + script_list_offset as u64,
            length,
        )?);
        let features = Box::new(FeatureList::new(
            reader,
            offset + feature_list_offset as u64,
            length,
        ));
        let lookups = Box::new(LookupList::new(
            reader,
            offset + lookup_list_offset as u64,
            length,
        )?);
        let feature_variations = if feature_variations_offset > 0 {
            Some(Box::new(FeatureVariationList::new(
                reader,
                offset + feature_variations_offset as u64,
                length,
            )))
        } else {
            None
        };
        Ok(Self {
            major_version,
            minor_version,
            scripts,
            features,
            lookups,
            feature_variations,
        })
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "GSUB\n".to_string();
        string += &format!("MajorVersion: {}\n", self.major_version);
        string += &format!("MinorVersion: {}\n", self.minor_version);
        string += &format!("Scripts:\n{}\n", self.scripts.to_string());
        string += &format!("Features:\n{}\n", self.features.to_string());
        string += &format!("Lookups:\n{}\n", self.lookups.to_string());
        string += &format!("FeatureVariations:\n{:?}\n", self.feature_variations);
        string
    }

    pub fn get_script(&self, tag: &[u8; 4]) -> Option<&ParsedScript> {
        self.scripts.get_script(tag)
    }

    pub fn get_features(&self, tag: &[u8; 4], script: &ParsedScript) -> Vec<&Feature> {
        let mut features = Vec::new();
        let language_system = &script.language_systems[0];
        for feature_index in language_system.language_system.feature_indexes.iter() {
            if self.features.features[*feature_index as usize].feature_tag
                == u32::from_be_bytes(*tag)
            {
                features.push(&self.features.features[*feature_index as usize]);
            }
        }
        features
    }

    pub fn get_lookups(&self, feature: &Feature) -> Vec<&Lookup> {
        let mut lookups = Vec::new();
        for lookup_index in feature.lookup_list_indices.iter() {
            lookups.push(&self.lookups.lookups[*lookup_index as usize]);
        }
        lookups
    }

    fn collect_feature_lookups<'a>(
        &'a self,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) -> Vec<&'a Lookup> {
        let mut lookups = Vec::new();
        let mut seen_lookup_indices = HashSet::new();

        for script in self.scripts.scripts.iter() {
            for language_system in self.get_language_systems(script, locale).iter() {
                for feature_index in language_system.language_system.feature_indexes.iter() {
                    let feature = &self.features.features[*feature_index as usize];
                    let feature_tag = feature.feature_tag;
                    if !feature_tags
                        .iter()
                        .any(|tag| feature_tag == u32::from_be_bytes(*tag))
                    {
                        continue;
                    }

                    for lookup_index in feature.lookup_list_indices.iter() {
                        if !seen_lookup_indices.insert(*lookup_index) {
                            continue;
                        }
                        lookups.push(&self.lookups.lookups[*lookup_index as usize]);
                    }
                }
            }
        }

        lookups
    }

    fn locale_to_language_system_tag(locale: &str) -> Option<u32> {
        let locale = locale.trim();
        if locale.is_empty() {
            return None;
        }

        let primary = locale
            .split(|c| c == '-' || c == '_')
            .next()
            .unwrap_or(locale)
            .trim();
        if primary.is_empty() {
            return None;
        }

        let lower = primary.to_ascii_lowercase();
        let tag = match lower.as_str() {
            "default" => [0, 0, 0, 0],
            "ja" | "jp" | "jpn" => *b"JAN ",
            _ => {
                let mut tag = [b' '; 4];
                for (i, ch) in primary.chars().take(4).enumerate() {
                    tag[i] = ch.to_ascii_uppercase() as u8;
                }
                tag
            }
        };

        Some(u32::from_be_bytes(tag))
    }

    fn get_language_systems<'a>(
        &'a self,
        script: &'a ParsedScript,
        locale: Option<&str>,
    ) -> Vec<&'a crate::opentype::layouts::LanguageSystemRecord> {
        let mut systems = Vec::new();

        if let Some(locale) = locale {
            if let Some(tag) = Self::locale_to_language_system_tag(locale) {
                if let Some(language_system) = script
                    .language_systems
                    .iter()
                    .find(|record| record.language_system_tag == tag)
                {
                    systems.push(language_system);
                }
            }

            if systems.is_empty() {
                if let Some(default_system) = script
                    .language_systems
                    .iter()
                    .find(|record| record.language_system_tag == 0)
                {
                    systems.push(default_system);
                }
            }

            if systems.is_empty() {
                if let Some(first) = script.language_systems.first() {
                    systems.push(first);
                }
            }
        } else {
            if let Some(default_system) = script
                .language_systems
                .iter()
                .find(|record| record.language_system_tag == 0)
            {
                systems.push(default_system);
            }

            for system in script.language_systems.iter() {
                if system.language_system_tag != 0 {
                    systems.push(system);
                }
            }
        }

        systems
    }

    fn lookup_single_feature(
        &self,
        glyph_id: usize,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) -> Option<usize> {
        for lookup in self.collect_feature_lookups(locale, feature_tags) {
            for subtable in lookup.subtables.iter() {
                match subtable.get_lookup(glyph_id) {
                    LookupResult::Single(result) => return Some(result as usize),
                    LookupResult::Multiple(results) => {
                        if let Some(result) = results.first() {
                            return Some(*result as usize);
                        }
                    }
                    LookupResult::Ligature(results) => {
                        if let Some(result) = results.first() {
                            return Some(result.ligature_glyph as usize);
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    fn lookup_ligature_feature(
        &self,
        glyph_ids: &[usize],
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) -> Option<usize> {
        let first_glyph = *glyph_ids.first()?;
        for lookup in self.collect_feature_lookups(locale, feature_tags) {
            for subtable in lookup.subtables.iter() {
                if let LookupResult::Ligature(records) = subtable.get_lookup(first_glyph) {
                    for record in records.iter() {
                        let expected_len = record.component_count as usize;
                        if expected_len != glyph_ids.len() {
                            continue;
                        }
                        if record
                            .component_glyph_ids
                            .iter()
                            .map(|glyph_id| *glyph_id as usize)
                            .eq(glyph_ids.iter().copied().skip(1))
                        {
                            return Some(record.ligature_glyph as usize);
                        }
                    }
                }
            }
        }

        None
    }

    fn apply_subtable_at(
        subtable: &crate::opentype::layouts::lookup::LookupSubstitution,
        glyphs: &mut Vec<(usize, usize)>,
        index: usize,
    ) -> bool {
        let Some((glyph_id, source_index)) = glyphs.get(index).copied() else {
            return false;
        };

        match subtable {
            crate::opentype::layouts::lookup::LookupSubstitution::Single(single) => {
                if let Some(replacement) = subtable.get_single_glyph_id(glyph_id as u16) {
                    glyphs[index].0 = replacement as usize;
                    return true;
                }
                let _ = single;
            }
            crate::opentype::layouts::lookup::LookupSubstitution::Single2(_) => {
                if let Some(replacement) = subtable.get_single_glyph_id(glyph_id as u16) {
                    glyphs[index].0 = replacement as usize;
                    return true;
                }
            }
            crate::opentype::layouts::lookup::LookupSubstitution::Multiple(multiple) => {
                if let Some(coverage_index) = multiple.coverage.contains(glyph_id) {
                    let replacement = multiple.sequence_tables[coverage_index]
                        .substitute_glyph_ids
                        .iter()
                        .map(|glyph_id| (*glyph_id as usize, source_index))
                        .collect::<Vec<_>>();
                    glyphs.splice(index..index + 1, replacement);
                    return true;
                }
            }
            crate::opentype::layouts::lookup::LookupSubstitution::Ligature(ligature) => {
                if let Some(coverage_index) = ligature.coverage.contains(glyph_id) {
                    let ligature_set = &ligature.ligature_set[coverage_index];
                    for record in &ligature_set.ligature_table {
                        let expected_len = record.component_count as usize;
                        if index + expected_len > glyphs.len() {
                            continue;
                        }
                        if record
                            .component_glyph_ids
                            .iter()
                            .map(|glyph_id| *glyph_id as usize)
                            .eq(glyphs[index + 1..index + expected_len].iter().map(|item| item.0))
                        {
                            glyphs.splice(
                                index..index + expected_len,
                                [(record.ligature_glyph as usize, source_index)],
                            );
                            return true;
                        }
                    }
                }
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ExtensionSubstitution(extension) => {
                return Self::apply_subtable_at(&extension.subtable, glyphs, index);
            }
            _ => {}
        }

        false
    }

    pub(crate) fn apply_lookup_once(lookup: &Lookup, glyphs: &mut Vec<(usize, usize)>) -> bool {
        let mut index = 0usize;
        while index < glyphs.len() {
            for subtable in &lookup.subtables {
                if Self::apply_subtable_at(subtable, glyphs, index) {
                    return true;
                }
            }
            index += 1;
        }
        false
    }

    pub(crate) fn apply_ccmp_sequence(&self, glyphs: &mut Vec<(usize, usize)>) {
        let lookups = self.collect_feature_lookups(None, &[*b"ccmp"]);
        if lookups.is_empty() || glyphs.is_empty() {
            return;
        }

        let mut iterations = 0usize;
        let max_iterations = lookups.len().saturating_mul(glyphs.len().max(1)).max(1) * 4;

        loop {
            let mut changed = false;
            for lookup in &lookups {
                if Self::apply_lookup_once(lookup, glyphs) {
                    changed = true;
                }
            }
            iterations += 1;
            if !changed || iterations >= max_iterations {
                break;
            }
        }
    }

    pub(crate) fn lookup_joining_forms(
        &self,
        glyph_id: usize,
        locale: Option<&str>,
    ) -> JoiningForms {
        JoiningForms {
            isolated: self.lookup_single_feature(glyph_id, locale, &[*b"isol"]),
            initial: self.lookup_single_feature(glyph_id, locale, &[*b"init"]),
            medial: self.lookup_single_feature(glyph_id, locale, &[*b"medi"]),
            final_form: self.lookup_single_feature(glyph_id, locale, &[*b"fina"]),
        }
    }

    pub(crate) fn apply_joining_sequence(
        &self,
        glyphs: &mut Vec<(usize, usize)>,
        locale: Option<&str>,
    ) {
        if glyphs.is_empty() {
            return;
        }

        let forms: Vec<JoiningForms> = glyphs
            .iter()
            .map(|(glyph_id, _)| self.lookup_joining_forms(*glyph_id, locale))
            .collect();

        for index in 0..glyphs.len() {
            let join_prev = if index > 0 {
                forms[index - 1].can_join_to_next() && forms[index].can_join_to_prev()
            } else {
                false
            };
            let join_next = if index + 1 < glyphs.len() {
                forms[index].can_join_to_next() && forms[index + 1].can_join_to_prev()
            } else {
                false
            };
            glyphs[index].0 = forms[index].substitute(glyphs[index].0, join_prev, join_next);
        }
    }

    // ccmp Glyph Composition / Decomposition
    pub fn lookup_ccmp(&self, _glyph_id: usize) -> Option<Vec<u16>> {
        let script = self.get_script(b"DFLT")?;
        let features = self.get_features(&b"ccmp", script);
        for feature in features.iter() {
            for lookup_index in feature.lookup_list_indices.iter() {
                let lookup = self.lookups.lookups[*lookup_index as usize].clone();
                for _subtable in lookup.subtables.iter() {
                    // get glyph_ids from subtable
                }
            }
        }
        None
    }

    // vert, vrt2, vrtr
    pub fn lookup_vertical(&self, glyph_id: u16) -> Option<u16> {
        self.lookup_single_feature(glyph_id as usize, None, &[*b"vert", *b"vrt2", *b"vrtr"])
            .map(|glyph_id| glyph_id as u16)
    }

    // locl
    pub fn lookup_locale(&self, griph_ids: usize, locale: &str) -> usize {
        self.lookup_single_feature(griph_ids, Some(locale), &[*b"locl"])
            .unwrap_or(griph_ids)
    }

    // liga
    pub fn lookup_liga(&self, griph_ids: usize) -> usize {
        self.lookup_liga_sequence(&[griph_ids]).unwrap_or(griph_ids)
    }

    pub fn lookup_liga_sequence(&self, griph_ids: &[usize]) -> Option<usize> {
        self.lookup_ligature_feature(griph_ids, None, &[*b"liga", *b"dlig"])
    }

    // hwid, fwid, qwid, twid, pkna
    pub fn lookup_width(&self, _griph_ids: usize, _tag: u32) -> usize {
        todo!("lookup_width")
    }

    // dnom, numr, frac, subs, sups, zero
    pub fn lookup_number(&self, _griph_ids: Vec<usize>) -> usize {
        todo!("lookup_number")
    }
}
