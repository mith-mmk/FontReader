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
            offset + length as u64,
        )?);
        let lookups = Box::new(LookupList::new(
            reader,
            offset + lookup_list_offset as u64,
            length,
        )?);
        let feature_variations = if feature_variations_offset > 0 {
            FeatureVariationList::new(reader, offset + feature_variations_offset as u64, length)
                .ok()
                .map(Box::new)
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
        for feature_index in
            Self::collect_language_system_feature_indices(&language_system.language_system)
        {
            if self.features.features[feature_index as usize].feature_tag
                == u32::from_be_bytes(*tag)
            {
                features.push(&self.features.features[feature_index as usize]);
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
        if locale.is_none() {
            return self.collect_feature_lookups_from_scripts(
                self.scripts.scripts.iter().collect(),
                locale,
                feature_tags,
            );
        }

        let (preferred_scripts, default_scripts, other_scripts) = self.partition_scripts(locale);
        let mut lookups = Vec::new();
        let mut seen_lookup_ptrs = HashSet::new();

        for feature_tag in feature_tags {
            let preferred = self.collect_feature_lookups_from_scripts(
                preferred_scripts.clone(),
                locale,
                std::slice::from_ref(feature_tag),
            );
            let script_lookups = if !preferred.is_empty() {
                preferred
            } else {
                let defaults = self.collect_feature_lookups_from_scripts(
                    default_scripts.clone(),
                    locale,
                    std::slice::from_ref(feature_tag),
                );
                if !defaults.is_empty() {
                    defaults
                } else {
                    self.collect_feature_lookups_from_scripts(
                        other_scripts.clone(),
                        locale,
                        std::slice::from_ref(feature_tag),
                    )
                }
            };

            for lookup in script_lookups {
                let ptr = lookup as *const Lookup as usize;
                if seen_lookup_ptrs.insert(ptr) {
                    lookups.push(lookup);
                }
            }
        }

        lookups
    }

    fn collect_feature_lookups_from_scripts<'a>(
        &'a self,
        scripts: Vec<&'a ParsedScript>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) -> Vec<&'a Lookup> {
        let mut lookups = Vec::new();
        let mut seen_lookup_indices = HashSet::new();

        for script in scripts {
            for language_system in self.get_language_systems(script, locale).iter() {
                for feature_index in
                    Self::collect_language_system_feature_indices(&language_system.language_system)
                {
                    let feature = &self.features.features[feature_index as usize];
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

    fn partition_scripts<'a>(
        &'a self,
        locale: Option<&str>,
    ) -> (
        Vec<&'a ParsedScript>,
        Vec<&'a ParsedScript>,
        Vec<&'a ParsedScript>,
    ) {
        if locale.is_none() {
            return (
                self.scripts.scripts.iter().collect(),
                Vec::new(),
                Vec::new(),
            );
        }

        let mut preferred = Vec::new();
        let mut defaults = Vec::new();
        let mut others = Vec::new();
        let preferred_tags = locale.map(Self::locale_to_script_tags).unwrap_or_default();

        if let Some(locale) = locale {
            for script_tag in Self::locale_to_script_tags(locale) {
                if let Some(script) = self
                    .scripts
                    .scripts
                    .iter()
                    .find(|script| script.script_tag == script_tag)
                {
                    preferred.push(script);
                }
            }
        }

        for script in self.scripts.scripts.iter() {
            if script.script_tag == u32::from_be_bytes(*b"DFLT") {
                defaults.push(script);
            } else if preferred_tags.contains(&script.script_tag) {
                if !preferred
                    .iter()
                    .any(|existing| existing.script_tag == script.script_tag)
                {
                    preferred.push(script);
                }
            } else {
                others.push(script);
            }
        }

        (preferred, defaults, others)
    }

    fn locale_subtags(locale: &str) -> Vec<String> {
        let locale = locale.trim();
        if locale.is_empty() {
            return Vec::new();
        }

        locale
            .split(|c| c == '-' || c == '_')
            .map(str::trim)
            .filter(|subtag| !subtag.is_empty())
            .map(|subtag| subtag.to_ascii_lowercase())
            .collect()
    }

    fn push_language_system_tag(tags: &mut Vec<u32>, tag: [u8; 4]) {
        let tag = u32::from_be_bytes(tag);
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }

    fn locale_to_language_system_tags(locale: &str) -> Vec<u32> {
        let subtags = Self::locale_subtags(locale);
        if subtags.is_empty() {
            return Vec::new();
        }

        let mut tags = Vec::new();
        match subtags[0].as_str() {
            "default" => Self::push_language_system_tag(&mut tags, [0, 0, 0, 0]),
            "ja" | "jp" | "jpn" => Self::push_language_system_tag(&mut tags, *b"JAN "),
            "ar" | "ara" => Self::push_language_system_tag(&mut tags, *b"ARA "),
            "fa" | "fas" | "per" => Self::push_language_system_tag(&mut tags, *b"FAR "),
            "ur" | "urd" => Self::push_language_system_tag(&mut tags, *b"URD "),
            "sd" | "snd" => Self::push_language_system_tag(&mut tags, *b"SND "),
            "he" | "heb" => {
                Self::push_language_system_tag(&mut tags, *b"IWR ");
                Self::push_language_system_tag(&mut tags, *b"HEB ");
            }
            "syr" => Self::push_language_system_tag(&mut tags, *b"SYR "),
            "syrj" => Self::push_language_system_tag(&mut tags, *b"SYRJ"),
            "syrn" => Self::push_language_system_tag(&mut tags, *b"SYRN"),
            _ => {}
        }

        for subtag in &subtags {
            match subtag.as_str() {
                "jp" => Self::push_language_system_tag(&mut tags, *b"JAN "),
                "arab" | "ar" | "ara" => Self::push_language_system_tag(&mut tags, *b"ARA "),
                "urd" | "ur" => Self::push_language_system_tag(&mut tags, *b"URD "),
                "far" | "fa" | "fas" | "per" => Self::push_language_system_tag(&mut tags, *b"FAR "),
                "snd" | "sd" => Self::push_language_system_tag(&mut tags, *b"SND "),
                "heb" | "he" => {
                    Self::push_language_system_tag(&mut tags, *b"IWR ");
                    Self::push_language_system_tag(&mut tags, *b"HEB ");
                }
                "syrc" | "syr" => Self::push_language_system_tag(&mut tags, *b"SYR "),
                "syrj" => Self::push_language_system_tag(&mut tags, *b"SYRJ"),
                "syrn" => Self::push_language_system_tag(&mut tags, *b"SYRN"),
                _ if (subtag.len() == 3 || subtag.len() == 4)
                    && subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) =>
                {
                    let mut tag = [b' '; 4];
                    for (i, ch) in subtag.chars().take(4).enumerate() {
                        tag[i] = ch.to_ascii_uppercase() as u8;
                    }
                    Self::push_language_system_tag(&mut tags, tag);
                }
                _ => {}
            }
        }

        tags
    }

    fn locale_to_script_tags(locale: &str) -> Vec<u32> {
        let subtags = Self::locale_subtags(locale);
        if subtags.is_empty() {
            return Vec::new();
        }

        let mut tags = Vec::new();
        let mut push_tag = |tag: [u8; 4]| {
            let tag = u32::from_be_bytes(tag);
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        };

        for subtag in &subtags {
            match subtag.as_str() {
                "arab" | "ar" | "ara" | "urd" | "fas" | "per" | "snd" => push_tag(*b"arab"),
                "hebr" | "he" | "heb" => push_tag(*b"hebr"),
                "syrc" | "syr" | "syc" | "syrj" | "syrn" => push_tag(*b"syrc"),
                "kana" | "ja" | "jp" | "jpn" => push_tag(*b"kana"),
                "hani" | "zh" | "zho" | "chi" => push_tag(*b"hani"),
                "hang" | "ko" | "kor" => push_tag(*b"hang"),
                _ if subtag.len() == 4 && subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) => {
                    let mut tag = [b' '; 4];
                    for (index, byte) in subtag.bytes().take(4).enumerate() {
                        tag[index] = byte;
                    }
                    push_tag(tag);
                }
                _ => {}
            }
        }

        tags
    }

    fn get_language_systems<'a>(
        &'a self,
        script: &'a ParsedScript,
        locale: Option<&str>,
    ) -> Vec<&'a crate::opentype::layouts::LanguageSystemRecord> {
        let mut systems = Vec::new();

        if let Some(locale) = locale {
            for tag in Self::locale_to_language_system_tags(locale) {
                if let Some(language_system) = script
                    .language_systems
                    .iter()
                    .find(|record| record.language_system_tag == tag)
                {
                    if !systems.iter().any(
                        |existing: &&crate::opentype::layouts::LanguageSystemRecord| {
                            existing.language_system_tag == language_system.language_system_tag
                        },
                    ) {
                        systems.push(language_system);
                    }
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

    fn collect_language_system_feature_indices(
        language_system: &crate::opentype::layouts::LanguageSystem,
    ) -> Vec<u16> {
        let mut feature_indices = Vec::new();

        if language_system.required_feature_index != 0xFFFF {
            feature_indices.push(language_system.required_feature_index);
        }

        for feature_index in &language_system.feature_indexes {
            if !feature_indices.contains(feature_index) {
                feature_indices.push(*feature_index);
            }
        }

        feature_indices
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
                            .eq(glyphs[index + 1..index + expected_len]
                                .iter()
                                .map(|item| item.0))
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
            crate::opentype::layouts::lookup::LookupSubstitution::ExtensionSubstitution(
                extension,
            ) => {
                return Self::apply_subtable_at(&extension.subtable, glyphs, index);
            }
            _ => {}
        }

        false
    }

    fn matches_input_coverages(
        coverages: &[crate::opentype::layouts::coverage::Coverage],
        glyphs: &[(usize, usize)],
        start: usize,
    ) -> bool {
        if start + coverages.len() > glyphs.len() {
            return false;
        }

        coverages
            .iter()
            .enumerate()
            .all(|(offset, coverage)| coverage.contains(glyphs[start + offset].0).is_some())
    }

    fn matches_backtrack_coverages(
        coverages: &[crate::opentype::layouts::coverage::Coverage],
        glyphs: &[(usize, usize)],
        start: usize,
    ) -> bool {
        if coverages.len() > start {
            return false;
        }

        coverages
            .iter()
            .enumerate()
            .all(|(offset, coverage)| coverage.contains(glyphs[start - 1 - offset].0).is_some())
    }

    fn matches_lookahead_coverages(
        coverages: &[crate::opentype::layouts::coverage::Coverage],
        glyphs: &[(usize, usize)],
        start: usize,
    ) -> bool {
        if start + coverages.len() > glyphs.len() {
            return false;
        }

        coverages
            .iter()
            .enumerate()
            .all(|(offset, coverage)| coverage.contains(glyphs[start + offset].0).is_some())
    }

    fn apply_lookup_index_at(
        &self,
        lookup_list_index: u16,
        glyphs: &mut Vec<(usize, usize)>,
        index: usize,
    ) -> bool {
        let Some(lookup) = self.lookups.lookups.get(lookup_list_index as usize) else {
            return false;
        };

        for subtable in &lookup.subtables {
            if self.apply_subtable_at_with_tables(subtable, glyphs, index) {
                return true;
            }
        }

        false
    }

    fn apply_sequence_lookup_records(
        &self,
        records: &crate::opentype::layouts::lookup::SequenceLookupRecords,
        glyphs: &mut Vec<(usize, usize)>,
        start_index: usize,
    ) -> bool {
        let mut changed = false;

        for record in &records.lookup_records {
            let target_index = start_index.saturating_add(record.sequence_index as usize);
            if target_index >= glyphs.len() {
                continue;
            }
            if self.apply_lookup_index_at(record.lookup_list_index, glyphs, target_index) {
                changed = true;
            }
        }

        changed
    }

    fn apply_subtable_at_with_tables(
        &self,
        subtable: &crate::opentype::layouts::lookup::LookupSubstitution,
        glyphs: &mut Vec<(usize, usize)>,
        index: usize,
    ) -> bool {
        let Some((glyph_id, _source_index)) = glyphs.get(index).copied() else {
            return false;
        };

        match subtable {
            crate::opentype::layouts::lookup::LookupSubstitution::ContextSubstitution(context) => {
                let Some(rule_set_index) = context.coverage.contains(glyph_id) else {
                    return false;
                };
                let Some(rule_set) = context.rule_sets.get(rule_set_index) else {
                    return false;
                };

                for rule in &rule_set.rules {
                    if index + rule.input_sequence.len() >= glyphs.len() + 1 {
                        continue;
                    }
                    let matches =
                        rule.input_sequence
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                glyphs[index + 1 + offset].0 == *expected as usize
                            });
                    if !matches {
                        continue;
                    }

                    let mut changed = false;
                    for lookup_index in &rule.lookup_indexes {
                        if self.apply_lookup_index_at(*lookup_index, glyphs, index) {
                            changed = true;
                        }
                    }
                    if changed {
                        return true;
                    }
                }
                false
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ContextSubstitution2(context) => {
                let Some(_) = context.coverage.contains(glyph_id) else {
                    return false;
                };
                let rule_set_index = context.class_def.get_class(glyph_id as u16) as usize;
                let Some(rule_set) = context.class_seq_rule_sets.get(rule_set_index) else {
                    return false;
                };

                for rule in &rule_set.class_seq_rules {
                    if index + rule.input_sequences.len() >= glyphs.len() + 1 {
                        continue;
                    }
                    let matches =
                        rule.input_sequences
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                context
                                    .class_def
                                    .get_class(glyphs[index + 1 + offset].0 as u16)
                                    == *expected
                            });
                    if !matches {
                        continue;
                    }
                    if self.apply_sequence_lookup_records(&rule.seq_lookup_records, glyphs, index) {
                        return true;
                    }
                }
                false
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ContextSubstitution3(context) => {
                if !Self::matches_input_coverages(&context.coverages, glyphs, index) {
                    return false;
                }
                self.apply_sequence_lookup_records(&context.seq_lookup_records, glyphs, index)
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ChainingContextSubstitution(
                chaining,
            ) => {
                let Some(rule_set_index) = chaining.coverage.contains(glyph_id) else {
                    return false;
                };
                let Some(rule_set) = chaining.chain_sub_rule_set.get(rule_set_index) else {
                    return false;
                };

                for rule in &rule_set.chain_sub_rule {
                    if rule.backtrack_glyph_ids.len() > index {
                        continue;
                    }
                    if index + rule.input_glyph_ids.len() + rule.lookahead_glyph_ids.len()
                        >= glyphs.len() + 1
                    {
                        continue;
                    }
                    let backtrack_matches =
                        rule.backtrack_glyph_ids
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                glyphs[index - 1 - offset].0 == *expected as usize
                            });
                    let input_matches =
                        rule.input_glyph_ids
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                glyphs[index + 1 + offset].0 == *expected as usize
                            });
                    let lookahead_matches =
                        rule.lookahead_glyph_ids
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                glyphs[index + 1 + rule.input_glyph_ids.len() + offset].0
                                    == *expected as usize
                            });
                    if !(backtrack_matches && input_matches && lookahead_matches) {
                        continue;
                    }

                    let mut changed = false;
                    for lookup_index in &rule.lookup_indexes {
                        if self.apply_lookup_index_at(*lookup_index, glyphs, index) {
                            changed = true;
                        }
                    }
                    if changed {
                        return true;
                    }
                }
                false
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ChainingContextSubstitution2(
                chaining,
            ) => {
                let Some(_) = chaining.coverage.contains(glyph_id) else {
                    return false;
                };
                let Some(input_class_def) = chaining.input_class_def.as_ref() else {
                    return false;
                };
                let current_class = input_class_def.get_class(glyph_id as u16) as usize;
                let Some(rule_set) = chaining.chain_sub_class_sets.get(current_class) else {
                    return false;
                };

                for rule in &rule_set.chained_class_seq_rules {
                    if rule.backtrack_sequences.len() > index {
                        continue;
                    }
                    if index + rule.input_sequences.len() + rule.lookahead_class_ids.len()
                        >= glyphs.len() + 1
                    {
                        continue;
                    }

                    let backtrack_matches = if let Some(backtrack_class_def) =
                        chaining.backtrack_class_def.as_ref()
                    {
                        rule.backtrack_sequences
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                backtrack_class_def.get_class(glyphs[index - 1 - offset].0 as u16)
                                    == *expected
                            })
                    } else {
                        rule.backtrack_sequences.is_empty()
                    };
                    let input_matches =
                        rule.input_sequences
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                input_class_def.get_class(glyphs[index + 1 + offset].0 as u16)
                                    == *expected
                            });
                    let lookahead_matches =
                        if let Some(lookahead_class_def) = chaining.lookahead_class_def.as_ref() {
                            rule.lookahead_class_ids
                                .iter()
                                .enumerate()
                                .all(|(offset, expected)| {
                                    lookahead_class_def.get_class(
                                        glyphs[index + 1 + rule.input_sequences.len() + offset].0
                                            as u16,
                                    ) == *expected
                                })
                        } else {
                            rule.lookahead_class_ids.is_empty()
                        };

                    if !(backtrack_matches && input_matches && lookahead_matches) {
                        continue;
                    }

                    if self.apply_sequence_lookup_records(&rule.seq_lookup_records, glyphs, index) {
                        return true;
                    }
                }
                false
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ChainingContextSubstitution3(
                chaining,
            ) => {
                if !Self::matches_backtrack_coverages(&chaining.backtrack_coverages, glyphs, index)
                {
                    return false;
                }
                if !Self::matches_input_coverages(&chaining.input_coverages, glyphs, index) {
                    return false;
                }
                if !Self::matches_lookahead_coverages(
                    &chaining.lookahead_coverages,
                    glyphs,
                    index + chaining.input_coverages.len(),
                ) {
                    return false;
                }
                self.apply_sequence_lookup_records(&chaining.seq_lookup_records, glyphs, index)
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ExtensionSubstitution(
                extension,
            ) => self.apply_subtable_at_with_tables(&extension.subtable, glyphs, index),
            _ => Self::apply_subtable_at(subtable, glyphs, index),
        }
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

    pub(crate) fn apply_lookup_once_with_tables(
        &self,
        lookup: &Lookup,
        glyphs: &mut Vec<(usize, usize)>,
    ) -> bool {
        let mut index = 0usize;
        while index < glyphs.len() {
            for subtable in &lookup.subtables {
                if self.apply_subtable_at_with_tables(subtable, glyphs, index) {
                    return true;
                }
            }
            index += 1;
        }
        false
    }

    pub(crate) fn apply_feature_sequence(
        &self,
        glyphs: &mut Vec<(usize, usize)>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) {
        let lookups = self.collect_feature_lookups(locale, feature_tags);
        if lookups.is_empty() || glyphs.is_empty() {
            return;
        }

        let mut iterations = 0usize;
        let max_iterations = lookups.len().saturating_mul(glyphs.len().max(1)).max(1) * 4;

        loop {
            let mut changed = false;
            for lookup in &lookups {
                if self.apply_lookup_once_with_tables(lookup, glyphs) {
                    changed = true;
                }
            }
            iterations += 1;
            if !changed || iterations >= max_iterations {
                break;
            }
        }
    }

    pub(crate) fn apply_ccmp_sequence(&self, glyphs: &mut Vec<(usize, usize)>) {
        self.apply_feature_sequence(glyphs, None, &[*b"ccmp"]);
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

    pub(crate) fn apply_rtl_contextual_sequence(
        &self,
        glyphs: &mut Vec<(usize, usize)>,
        locale: Option<&str>,
    ) {
        self.apply_feature_sequence(glyphs, locale, &[*b"rlig", *b"rclt", *b"calt", *b"clig"]);
    }

    pub(crate) fn apply_variant_sequence(
        &self,
        glyphs: &mut Vec<(usize, usize)>,
        locale: Option<&str>,
        font_variant: crate::commands::FontVariant,
    ) {
        let feature_tags = font_variant.gsub_feature_tags();
        if feature_tags.is_empty() {
            return;
        }
        self.apply_feature_sequence(glyphs, locale, feature_tags);
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

    pub(crate) fn lookup_standard_liga_sequence(&self, glyph_ids: &[usize]) -> Option<usize> {
        self.lookup_ligature_feature(glyph_ids, None, &[*b"liga"])
    }

    pub(crate) fn lookup_discretionary_liga_sequence(&self, glyph_ids: &[usize]) -> Option<usize> {
        self.lookup_ligature_feature(glyph_ids, None, &[*b"dlig"])
    }

    // rlig
    pub fn lookup_rlig_sequence(&self, glyph_ids: &[usize], locale: Option<&str>) -> Option<usize> {
        self.lookup_ligature_feature(glyph_ids, locale, &[*b"rlig"])
    }

    pub fn lookup_liga_sequence(&self, griph_ids: &[usize]) -> Option<usize> {
        self.lookup_standard_liga_sequence(griph_ids)
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
