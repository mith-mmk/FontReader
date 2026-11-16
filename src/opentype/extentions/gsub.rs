#![allow(dead_code)]

// GSUB -- Glyph Substitution Table

// only check lookup Format1.1 , 4, 5, 6.1, 7

use std::collections::{HashMap, HashSet};
use std::io::SeekFrom;

use crate::opentype::extentions::gdef::{Gdef, GlyphClass};
use crate::opentype::layouts::{
    feature::Feature,
    lookup::{Lookup, LookupResult, LookupType},
    script::ParsedScript,
    *,
};
use bin_rs::reader::BinaryReader;

const MAX_GSUB_GLYPHS: usize = 1_000_000;
// Bound nested execution per input position without truncating ordinary scans.
const MAX_GSUB_LOOKUP_APPLICATIONS: usize = 4096;

const MAX_GSUB_NESTING: usize = 32;

mod filtered_context;

/// Metadata carried through every sequence substitution, before GPOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GsubGlyph {
    pub(crate) glyph_id: usize,
    pub(crate) source_index: usize,
    // A sequence substitution may depend on normalized context, not only its owner.
    pub(crate) substituted: bool,
    // Monotonic: only class-preserving table-driven single substitutions keep
    // original simple mark ownership. This is not ligature component metadata.
    pub(crate) mkmk_source_ambiguous: bool,
    pub(crate) ligature_components: u16,
}

struct ApplyState<'a> {
    gdef: Option<&'a Gdef>,
    // Only the bounded Tibetan plan can prove exact no-skip filtering.
    exact_glyph_counts: Option<HashMap<usize, usize>>,
    // Cache for the current set of present IDs; invalidate on 0-count transitions.
    exact_filter_proofs: HashMap<*const Lookup, bool>,
    ambiguous_sources: bool,
    // Base, ligature, and mark counts for the current run. Updating just the
    // replaced span keeps repeated flag checks constant-time, including after
    // nested substitutions introduce or remove a filtered glyph class.
    class_counts: [usize; 3],
    remaining: usize,
    depth: usize,
    next_index: usize,
    filtered: filtered_context::FilteredState,
}
impl<'a> ApplyState<'a> {
    fn new(gdef: Option<&'a Gdef>, glyphs: &[GsubGlyph]) -> Self {
        let mut state = Self {
            gdef: gdef.filter(|gdef| gdef.glyph_class_def.is_some()),
            exact_glyph_counts: None,
            exact_filter_proofs: HashMap::new(),
            ambiguous_sources: false,
            class_counts: [0; 3],
            remaining: MAX_GSUB_LOOKUP_APPLICATIONS,
            depth: 0,
            next_index: 0,
            filtered: filtered_context::FilteredState::default(),
        };
        state.replace_classes(&[], glyphs);
        state
    }

    fn class_index(&self, glyph_id: usize) -> Option<usize> {
        match self.gdef?.glyph_class(glyph_id as u16) {
            Some(GlyphClass::Base) => Some(0),
            Some(GlyphClass::Ligature) => Some(1),
            Some(GlyphClass::Mark) => Some(2),
            _ => None,
        }
    }

    fn preserves_simple_class(&self, original: usize, replacement: usize) -> bool {
        matches!(self.class_index(original), Some(0 | 2))
            && self.class_index(original) == self.class_index(replacement)
    }

    fn replace_classes(&mut self, removed: &[GsubGlyph], added: &[GsubGlyph]) {
        if let Some(counts) = &mut self.exact_glyph_counts {
            for glyph in removed {
                if let Some(count) = counts.get_mut(&glyph.glyph_id) {
                    *count -= 1;
                    if *count == 0 {
                        counts.remove(&glyph.glyph_id);
                        self.exact_filter_proofs.clear();
                    }
                }
            }
            for glyph in added {
                let count = counts.entry(glyph.glyph_id).or_default();
                if *count == 0 {
                    self.exact_filter_proofs.clear();
                }
                *count += 1;
            }
        }
        if self.gdef.is_none() {
            return;
        }
        for glyph in removed {
            if let Some(index) = self.class_index(glyph.glyph_id) {
                self.class_counts[index] -= 1;
            }
        }
        for glyph in added {
            if let Some(index) = self.class_index(glyph.glyph_id) {
                self.class_counts[index] += 1;
            }
        }
    }

    fn allows_lookup(&mut self, lookup: &Lookup) -> bool {
        let flag = lookup.lookup_flag;
        if flag & 0xFF1E == 0 {
            return true;
        }
        if self.gdef.is_none() {
            return false;
        }
        if let Some(counts) = &self.exact_glyph_counts {
            let pointer = lookup as *const Lookup;
            if let Some(proven) = self.exact_filter_proofs.get(&pointer) {
                return *proven;
            }
            let gdef = self.gdef.unwrap();
            let proven = counts
                .keys()
                .all(|glyph_id| match gdef.glyph_class(*glyph_id as u16) {
                    Some(GlyphClass::Base) => flag & 0x0002 == 0,
                    Some(GlyphClass::Ligature) => flag & 0x0004 == 0,
                    Some(GlyphClass::Component) => true,
                    Some(GlyphClass::Mark) => {
                        if flag & 0x0008 != 0 {
                            return false;
                        }
                        if flag & 0x0010 != 0 {
                            return lookup
                                .mark_filtering_set
                                .and_then(|index| {
                                    gdef.mark_glyph_sets_def
                                        .as_ref()?
                                        .coverages
                                        .get(index as usize)
                                })
                                .is_some_and(|coverage| coverage.contains(*glyph_id).is_some());
                        }
                        flag >> 8 == 0
                            || gdef.mark_attachment_class(*glyph_id as u16) == Some(flag >> 8)
                    }
                    None => false,
                });
            self.exact_filter_proofs.insert(pointer, proven);
            return proven;
        }
        // This engine does not yet skip filtered glyphs when matching. A flagged
        // lookup is safe only when declared GDEF classes prove no glyph would
        // be filtered. Attachment/filter-set flags remain conservative for marks.
        (flag & 0x0002 == 0 || self.class_counts[0] == 0)
            && (flag & 0x0004 == 0 || self.class_counts[1] == 0)
            && (flag & 0xFF18 == 0 || self.class_counts[2] == 0)
    }

    fn enter(&mut self) -> bool {
        if self.remaining == 0 {
            self.filtered.fail_if_enabled();
            return false;
        }
        self.remaining -= 1;
        true
    }
}

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
        self.selected_substitution(join_prev, join_next)
            .unwrap_or(glyph_id)
    }

    fn selected_substitution(self, join_prev: bool, join_next: bool) -> Option<usize> {
        if join_prev && join_next && self.medial.is_some() {
            return self.medial;
        }
        if join_prev && self.final_form.is_some() {
            return self.final_form;
        }
        if join_next && self.initial.is_some() {
            return self.initial;
        }
        self.isolated
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Gsub {
    pub(crate) major_version: u16,
    pub(crate) minor_version: u16,
    pub(crate) scripts: Box<ScriptList>,
    pub(crate) features: Box<FeatureList>,
    pub(crate) lookups: Box<LookupList>,
    // version 1.1
    pub(crate) feature_variations: Option<Box<FeatureVariationList>>,
}

impl Gsub {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, std::io::Error> {
        let offset = offset as u64;

        reader.seek(SeekFrom::Start(offset))?;
        let major_version = reader.read_u16_be()?;
        let minor_version = reader.read_u16_be()?;
        let script_list_offset = reader.read_u16_be()?;
        let feature_list_offset = reader.read_u16_be()?;
        let lookup_list_offset = reader.read_u16_be()?;
        let feature_variations_offset = if major_version == 1 && minor_version == 1 {
            reader.read_u32_be()?
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

    fn dump_text(&self) -> String {
        let mut string = "GSUB\n".to_string();
        string += &format!("MajorVersion: {}\n", self.major_version);
        string += &format!("MinorVersion: {}\n", self.minor_version);
        string += &format!("Scripts:\n{}\n", self.scripts);
        string += &format!("Features:\n{}\n", self.features);
        string += &format!("Lookups:\n{}\n", self.lookups);
        string += &format!("FeatureVariations:\n{:?}\n", self.feature_variations);
        string
    }

    pub fn get_script(&self, tag: &[u8; 4]) -> Option<&ParsedScript> {
        self.scripts.get_script(tag)
    }

    pub fn get_features(&self, tag: &[u8; 4], script: &ParsedScript) -> Vec<&Feature> {
        let mut features = Vec::new();
        let Some(language_system) = script.language_systems.first() else {
            return features;
        };
        for feature_index in
            Self::collect_language_system_feature_indices(&language_system.language_system)
        {
            if let Some(feature) = self.features.features.get(feature_index as usize) {
                if feature.feature_tag == u32::from_be_bytes(*tag) {
                    features.push(feature);
                }
            }
        }
        features
    }

    pub fn get_lookups(&self, feature: &Feature) -> Vec<&Lookup> {
        let mut lookups = Vec::new();
        for lookup_index in feature.lookup_list_indices.iter() {
            if let Some(lookup) = self.lookups.lookups.get(*lookup_index as usize) {
                lookups.push(lookup);
            }
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
                    let Some(feature) = self.features.features.get(feature_index as usize) else {
                        continue;
                    };
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
                        if let Some(lookup) = self.lookups.lookups.get(*lookup_index as usize) {
                            lookups.push(lookup);
                        }
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
            .split(['-', '_'])
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
                        // This caller has exactly one input glyph. A ligature's
                        // component count includes that first input; never invent
                        // its remaining components in the scalar locl/joining path.
                        if let Some(result) = results.iter().find(|result| {
                            result.component_count == 1 && result.component_glyph_ids.is_empty()
                        }) {
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
        glyphs: &mut Vec<GsubGlyph>,
        index: usize,
        state: &mut ApplyState,
    ) -> bool {
        let Some(original) = glyphs.get(index).copied() else {
            return false;
        };

        let glyph_id = original.glyph_id;
        match subtable {
            crate::opentype::layouts::lookup::LookupSubstitution::Single(single) => {
                if let Some(replacement) = subtable.get_single_glyph_id(glyph_id as u16) {
                    state.replace_classes(
                        &[original],
                        &[GsubGlyph {
                            glyph_id: replacement as usize,
                            ..original
                        }],
                    );
                    glyphs[index].mkmk_source_ambiguous |=
                        !state.preserves_simple_class(glyph_id, replacement as usize);
                    glyphs[index].glyph_id = replacement as usize;
                    glyphs[index].substituted = true;
                    state.next_index = index + 1;
                    return true;
                }
                let _ = single;
            }
            crate::opentype::layouts::lookup::LookupSubstitution::Single2(_) => {
                if let Some(replacement) = subtable.get_single_glyph_id(glyph_id as u16) {
                    state.replace_classes(
                        &[original],
                        &[GsubGlyph {
                            glyph_id: replacement as usize,
                            ..original
                        }],
                    );
                    glyphs[index].mkmk_source_ambiguous |=
                        !state.preserves_simple_class(glyph_id, replacement as usize);
                    glyphs[index].glyph_id = replacement as usize;
                    glyphs[index].substituted = true;
                    state.next_index = index + 1;
                    return true;
                }
            }
            crate::opentype::layouts::lookup::LookupSubstitution::Multiple(multiple) => {
                if let Some(coverage_index) = multiple.coverage.contains(glyph_id) {
                    let Some(sequence_table) = multiple.sequence_tables.get(coverage_index) else {
                        return false;
                    };
                    // No exact source-set metadata is carried through expansion
                    // or deletion. The bounded Tibetan caller must roll back.
                    if sequence_table.substitute_glyph_ids.len() != 1 {
                        state.ambiguous_sources = true;
                    }
                    let new_length = glyphs.len().checked_sub(1).and_then(|length| {
                        length.checked_add(sequence_table.substitute_glyph_ids.len())
                    });
                    if new_length.is_none_or(|length| length > MAX_GSUB_GLYPHS) {
                        return false;
                    }
                    let replacement = sequence_table
                        .substitute_glyph_ids
                        .iter()
                        .map(|glyph_id| GsubGlyph {
                            glyph_id: *glyph_id as usize,
                            substituted: true,
                            mkmk_source_ambiguous: true,
                            ..original
                        })
                        .collect::<Vec<_>>();
                    state.next_index = index + replacement.len();
                    state.replace_classes(&glyphs[index..index + 1], &replacement);
                    glyphs.splice(index..index + 1, replacement);
                    return true;
                }
            }
            crate::opentype::layouts::lookup::LookupSubstitution::Ligature(ligature) => {
                if let Some(coverage_index) = ligature.coverage.contains(glyph_id) {
                    let Some(ligature_set) = ligature.ligature_set.get(coverage_index) else {
                        return false;
                    };
                    for record in &ligature_set.ligature_table {
                        let expected_len = record.component_count as usize;
                        let Some(end) = index.checked_add(expected_len) else {
                            continue;
                        };
                        if expected_len == 0 || end > glyphs.len() {
                            continue;
                        }
                        if record
                            .component_glyph_ids
                            .iter()
                            .map(|glyph_id| *glyph_id as usize)
                            .eq(glyphs[index + 1..index + expected_len]
                                .iter()
                                .map(|item| item.glyph_id))
                        {
                            state.next_index = index + 1;
                            let replacement = GsubGlyph {
                                glyph_id: record.ligature_glyph as usize,
                                substituted: true,
                                mkmk_source_ambiguous: true,
                                ligature_components: glyphs[index..end]
                                    .iter()
                                    .fold(0u16, |count, glyph| {
                                        count.saturating_add(glyph.ligature_components)
                                    }),
                                ..original
                            };
                            state.replace_classes(&glyphs[index..end], &[replacement]);
                            glyphs.splice(index..end, [replacement]);
                            return true;
                        }
                    }
                }
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ExtensionSubstitution(
                extension,
            ) => {
                if extension.subst_format != 1 || extension.extension_lookup_type == 7 {
                    return false;
                }
                return Self::apply_subtable_at(&extension.subtable, glyphs, index, state);
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ReverseChainSingle(reverse) => {
                let Some(coverage_index) = reverse.coverage.contains(glyph_id) else {
                    return false;
                };
                if reverse.backtrack_coverages.len() > index
                    || index
                        .saturating_add(1)
                        .saturating_add(reverse.lookahead_coverages.len())
                        > glyphs.len()
                {
                    return false;
                }
                let backtrack_matches =
                    reverse
                        .backtrack_coverages
                        .iter()
                        .enumerate()
                        .all(|(offset, coverage)| {
                            coverage
                                .contains(glyphs[index - 1 - offset].glyph_id)
                                .is_some()
                        });
                let lookahead_matches =
                    reverse
                        .lookahead_coverages
                        .iter()
                        .enumerate()
                        .all(|(offset, coverage)| {
                            coverage
                                .contains(glyphs[index.saturating_add(1) + offset].glyph_id)
                                .is_some()
                        });
                if !(backtrack_matches && lookahead_matches) {
                    return false;
                }
                let Some(&replacement) = reverse.substitute_glyph_ids.get(coverage_index) else {
                    return false;
                };
                state.replace_classes(
                    &[original],
                    &[GsubGlyph {
                        glyph_id: replacement as usize,
                        ..original
                    }],
                );
                glyphs[index].mkmk_source_ambiguous |=
                    !state.preserves_simple_class(glyph_id, replacement as usize);
                glyphs[index].glyph_id = replacement as usize;
                glyphs[index].substituted = true;
                state.next_index = index + 1;
                return true;
            }
            _ => {}
        }

        false
    }

    fn matches_input_coverages(
        coverages: &[crate::opentype::layouts::coverage::Coverage],
        glyphs: &[GsubGlyph],
        start: usize,
    ) -> bool {
        let Some(end) = start.checked_add(coverages.len()) else {
            return false;
        };
        if end > glyphs.len() {
            return false;
        }

        coverages
            .iter()
            .enumerate()
            .all(|(offset, coverage)| coverage.contains(glyphs[start + offset].glyph_id).is_some())
    }

    fn matches_backtrack_coverages(
        coverages: &[crate::opentype::layouts::coverage::Coverage],
        glyphs: &[GsubGlyph],
        start: usize,
    ) -> bool {
        if coverages.len() > start {
            return false;
        }

        coverages.iter().enumerate().all(|(offset, coverage)| {
            coverage
                .contains(glyphs[start - 1 - offset].glyph_id)
                .is_some()
        })
    }

    fn matches_lookahead_coverages(
        coverages: &[crate::opentype::layouts::coverage::Coverage],
        glyphs: &[GsubGlyph],
        start: usize,
    ) -> bool {
        let Some(end) = start.checked_add(coverages.len()) else {
            return false;
        };
        if end > glyphs.len() {
            return false;
        }

        coverages
            .iter()
            .enumerate()
            .all(|(offset, coverage)| coverage.contains(glyphs[start + offset].glyph_id).is_some())
    }

    fn apply_lookup_index_at(
        &self,
        lookup_list_index: u16,
        glyphs: &mut Vec<GsubGlyph>,
        index: usize,
        state: &mut ApplyState,
    ) -> bool {
        if state.depth >= MAX_GSUB_NESTING || !state.enter() {
            state.filtered.fail_if_enabled();
            return false;
        }
        let Some(lookup) = self.lookups.lookups.get(lookup_list_index as usize) else {
            state.filtered.fail_if_enabled();
            return false;
        };
        state.depth += 1;
        let applied = self.dispatch_lookup_at(lookup, glyphs, index, state);
        state.depth -= 1;
        applied
    }

    fn dispatch_lookup_at(
        &self,
        lookup: &Lookup,
        glyphs: &mut Vec<GsubGlyph>,
        index: usize,
        state: &mut ApplyState,
    ) -> bool {
        if state.filtered.failed {
            return false;
        }
        let proven = state.allows_lookup(lookup);
        if state.filtered.enabled && (!proven || state.filtered.input_bound.is_some()) {
            return self.apply_filtered_lookup_at(lookup, glyphs, index, state);
        }
        if !proven {
            return false;
        }
        for subtable in &lookup.subtables {
            let applied = self.apply_subtable_at_with_tables(subtable, glyphs, index, state);
            if applied || state.filtered.failed {
                return applied;
            }
        }
        false
    }

    fn apply_sequence_lookup_records(
        &self,
        records: &crate::opentype::layouts::lookup::SequenceLookupRecords,
        glyphs: &mut Vec<GsubGlyph>,
        start_index: usize,
        input_count: usize,
        state: &mut ApplyState,
    ) -> bool {
        if input_count == 0 {
            return false;
        }
        let mut end = start_index.saturating_add(input_count).min(glyphs.len());
        // SequenceIndex addresses the sequence after earlier records' changes.
        for record in &records.lookup_records {
            if state.remaining == 0 {
                state.filtered.fail_if_enabled();
                break;
            }
            let target = start_index.saturating_add(record.sequence_index as usize);
            if target >= end || target >= glyphs.len() {
                continue;
            }
            let old_len = glyphs.len();
            self.apply_lookup_index_at(record.lookup_list_index, glyphs, target, state);
            if glyphs.len() >= old_len {
                end = end.saturating_add(glyphs.len() - old_len);
            } else {
                end = end.saturating_sub(old_len - glyphs.len());
            }
        }
        state.next_index = end.max(start_index).min(glyphs.len());
        // A matching context (including ignore/no-op rules) blocks later rules.
        true
    }

    fn apply_subtable_at_with_tables(
        &self,
        subtable: &crate::opentype::layouts::lookup::LookupSubstitution,
        glyphs: &mut Vec<GsubGlyph>,
        index: usize,
        state: &mut ApplyState,
    ) -> bool {
        let Some(original) = glyphs.get(index).copied() else {
            return false;
        };

        let glyph_id = original.glyph_id;
        match subtable {
            crate::opentype::layouts::lookup::LookupSubstitution::ContextSubstitution(context) => {
                let Some(rule_set_index) = context.coverage.contains(glyph_id) else {
                    return false;
                };
                let Some(rule_set) = context.rule_sets.get(rule_set_index) else {
                    return false;
                };

                for rule in &rule_set.rules {
                    if rule.input_sequence.len()
                        > glyphs.len().saturating_sub(index.saturating_add(1))
                    {
                        continue;
                    }
                    let matches =
                        rule.input_sequence
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                glyphs[index + 1 + offset].glyph_id == *expected as usize
                            });
                    if !matches {
                        continue;
                    }

                    return self.apply_sequence_lookup_records(
                        &rule.seq_lookup_records,
                        glyphs,
                        index,
                        rule.input_sequence.len() + 1,
                        state,
                    );
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
                    if rule.input_sequences.len()
                        > glyphs.len().saturating_sub(index.saturating_add(1))
                    {
                        continue;
                    }
                    let matches =
                        rule.input_sequences
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                context
                                    .class_def
                                    .get_class(glyphs[index + 1 + offset].glyph_id as u16)
                                    == *expected
                            });
                    if !matches {
                        continue;
                    }
                    return self.apply_sequence_lookup_records(
                        &rule.seq_lookup_records,
                        glyphs,
                        index,
                        rule.input_sequences.len() + 1,
                        state,
                    );
                }
                false
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ContextSubstitution3(context) => {
                if !Self::matches_input_coverages(&context.coverages, glyphs, index) {
                    return false;
                }
                self.apply_sequence_lookup_records(
                    &context.seq_lookup_records,
                    glyphs,
                    index,
                    context.coverages.len(),
                    state,
                )
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
                    let required_following = rule
                        .input_glyph_ids
                        .len()
                        .checked_add(rule.lookahead_glyph_ids.len());
                    if required_following
                        .map(|required| {
                            required > glyphs.len().saturating_sub(index.saturating_add(1))
                        })
                        .unwrap_or(true)
                    {
                        continue;
                    }
                    let backtrack_matches =
                        rule.backtrack_glyph_ids
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                glyphs[index - 1 - offset].glyph_id == *expected as usize
                            });
                    let input_matches =
                        rule.input_glyph_ids
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                glyphs[index + 1 + offset].glyph_id == *expected as usize
                            });
                    let lookahead_matches =
                        rule.lookahead_glyph_ids
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                glyphs[index + 1 + rule.input_glyph_ids.len() + offset].glyph_id
                                    == *expected as usize
                            });
                    if !(backtrack_matches && input_matches && lookahead_matches) {
                        continue;
                    }

                    return self.apply_sequence_lookup_records(
                        &rule.seq_lookup_records,
                        glyphs,
                        index,
                        rule.input_glyph_ids.len() + 1,
                        state,
                    );
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
                    let required_following = rule
                        .input_sequences
                        .len()
                        .checked_add(rule.lookahead_class_ids.len());
                    if required_following
                        .map(|required| {
                            required > glyphs.len().saturating_sub(index.saturating_add(1))
                        })
                        .unwrap_or(true)
                    {
                        continue;
                    }

                    let backtrack_matches =
                        if let Some(backtrack_class_def) = chaining.backtrack_class_def.as_ref() {
                            rule.backtrack_sequences
                                .iter()
                                .enumerate()
                                .all(|(offset, expected)| {
                                    backtrack_class_def
                                        .get_class(glyphs[index - 1 - offset].glyph_id as u16)
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
                                input_class_def
                                    .get_class(glyphs[index + 1 + offset].glyph_id as u16)
                                    == *expected
                            });
                    let lookahead_matches = if let Some(lookahead_class_def) =
                        chaining.lookahead_class_def.as_ref()
                    {
                        rule.lookahead_class_ids
                            .iter()
                            .enumerate()
                            .all(|(offset, expected)| {
                                lookahead_class_def.get_class(
                                    glyphs[index + 1 + rule.input_sequences.len() + offset].glyph_id
                                        as u16,
                                ) == *expected
                            })
                    } else {
                        rule.lookahead_class_ids.is_empty()
                    };

                    if !(backtrack_matches && input_matches && lookahead_matches) {
                        continue;
                    }

                    return self.apply_sequence_lookup_records(
                        &rule.seq_lookup_records,
                        glyphs,
                        index,
                        rule.input_sequences.len() + 1,
                        state,
                    );
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
                self.apply_sequence_lookup_records(
                    &chaining.seq_lookup_records,
                    glyphs,
                    index,
                    chaining.input_coverages.len(),
                    state,
                )
            }
            crate::opentype::layouts::lookup::LookupSubstitution::ExtensionSubstitution(
                extension,
            ) => {
                if extension.subst_format != 1 || extension.extension_lookup_type == 7 {
                    return false;
                }
                self.apply_subtable_at_with_tables(&extension.subtable, glyphs, index, state)
            }
            _ => Self::apply_subtable_at(subtable, glyphs, index, state),
        }
    }

    pub(crate) fn apply_lookup_once(lookup: &Lookup, glyphs: &mut Vec<(usize, usize)>) -> bool {
        let mut working = Self::with_metadata(glyphs);
        let mut state = ApplyState::new(None, &working);
        let mut applied = false;
        if lookup.lookup_flag & 0xFF1E == 0 {
            for index in 0..working.len() {
                if lookup.subtables.iter().any(|subtable| {
                    Self::apply_subtable_at(subtable, &mut working, index, &mut state)
                }) {
                    applied = true;
                    break;
                }
            }
        }
        *glyphs = working
            .into_iter()
            .map(|glyph| (glyph.glyph_id, glyph.source_index))
            .collect();
        applied
    }

    pub(crate) fn apply_lookup_once_with_tables(
        &self,
        lookup: &Lookup,
        glyphs: &mut Vec<(usize, usize)>,
    ) -> bool {
        let mut working = Self::with_metadata(glyphs);
        let mut state = ApplyState::new(None, &working);
        let mut applied = false;
        if lookup.lookup_flag & 0xFF1E == 0 {
            for index in 0..working.len() {
                if lookup.subtables.iter().any(|subtable| {
                    self.apply_subtable_at_with_tables(subtable, &mut working, index, &mut state)
                }) {
                    applied = true;
                    break;
                }
            }
        }
        *glyphs = working
            .into_iter()
            .map(|glyph| (glyph.glyph_id, glyph.source_index))
            .collect();
        applied
    }

    fn apply_lookup_all_with_tables(
        &self,
        lookup: &Lookup,
        glyphs: &mut Vec<GsubGlyph>,
        state: &mut ApplyState,
    ) {
        if state.filtered.enabled {
            self.apply_filtered_transaction_lookup(lookup, glyphs, state);
            return;
        }
        if !state.allows_lookup(lookup) {
            return;
        }
        let reverse = lookup.lookup_type == LookupType::ReverseChainingContextualSingleSubstitution as u16
            || (lookup.lookup_type == LookupType::ExtensionSubstitution as u16 && lookup.subtables.iter().any(|subtable| matches!(subtable,
                crate::opentype::layouts::lookup::LookupSubstitution::ExtensionSubstitution(extension)
                    if extension.extension_lookup_type == LookupType::ReverseChainingContextualSingleSubstitution as u16)));
        if reverse {
            for index in (0..glyphs.len()).rev() {
                if !state.allows_lookup(lookup) {
                    break;
                }
                state.remaining = MAX_GSUB_LOOKUP_APPLICATIONS;
                for subtable in &lookup.subtables {
                    if self.apply_subtable_at_with_tables(subtable, glyphs, index, state) {
                        break;
                    }
                }
            }
            return;
        }
        let mut index = 0;
        while index < glyphs.len() {
            if !state.allows_lookup(lookup) {
                break;
            }
            state.remaining = MAX_GSUB_LOOKUP_APPLICATIONS;
            let old_len = glyphs.len();
            let old_index = index;
            let applied = lookup
                .subtables
                .iter()
                .any(|subtable| self.apply_subtable_at_with_tables(subtable, glyphs, index, state));
            if applied {
                index = state.next_index;
            } else {
                index += 1;
            }
            // A matched empty context must not repeatedly match the same position.
            if applied && index <= old_index && glyphs.len() >= old_len {
                index = old_index + 1;
            }
        }
    }

    fn with_metadata(glyphs: &[(usize, usize)]) -> Vec<GsubGlyph> {
        glyphs
            .iter()
            .map(|&(glyph_id, source_index)| GsubGlyph {
                glyph_id,
                source_index,
                substituted: false,
                mkmk_source_ambiguous: false,
                ligature_components: 1,
            })
            .collect()
    }

    pub(crate) fn apply_feature_sequence(
        &self,
        glyphs: &mut Vec<(usize, usize)>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) {
        let mut working = Self::with_metadata(glyphs);
        self.apply_feature_glyphs(&mut working, locale, feature_tags);
        *glyphs = working
            .into_iter()
            .map(|glyph| (glyph.glyph_id, glyph.source_index))
            .collect();
    }

    pub(crate) fn apply_feature_glyphs(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
    ) {
        self.apply_selected_glyphs(
            glyphs,
            self.collect_feature_lookups(locale, feature_tags),
            None,
        );
    }

    /// Ordinary features use locale selection; orientation retains the legacy
    /// all-script selection while sharing one ordered, deduplicated pass.
    pub(crate) fn apply_feature_glyphs_with_orientation(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
        orientation_tag: Option<[u8; 4]>,
    ) {
        let mut selected = self.collect_feature_lookups(locale, feature_tags);
        if let Some(tag) = orientation_tag {
            selected.extend(self.collect_feature_lookups(None, &[tag]));
        }
        self.apply_selected_glyphs(glyphs, selected, None);
    }

    pub(crate) fn apply_feature_glyphs_with_gdef(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
        gdef: Option<&Gdef>,
    ) {
        self.apply_selected_glyphs(
            glyphs,
            self.collect_feature_lookups(locale, feature_tags),
            gdef,
        );
    }

    pub(crate) fn apply_feature_glyphs_with_orientation_and_gdef(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        locale: Option<&str>,
        feature_tags: &[[u8; 4]],
        orientation_tag: Option<[u8; 4]>,
        gdef: Option<&Gdef>,
    ) {
        let mut selected = self.collect_feature_lookups(locale, feature_tags);
        if let Some(tag) = orientation_tag {
            selected.extend(self.collect_feature_lookups(None, &[tag]));
        }
        self.apply_selected_glyphs(glyphs, selected, gdef);
    }

    fn collect_tibetan_stack_lookups(
        &self,
        locale: Option<&str>,
        tags: &[[u8; 4]],
    ) -> Vec<&Lookup> {
        self.collect_feature_lookups_from_scripts(
            self.scripts
                .scripts
                .iter()
                .filter(|script| script.script_tag == u32::from_be_bytes(*b"tibt"))
                .collect(),
            locale,
            tags,
        )
    }

    pub(crate) fn has_tibetan_stack_features(&self, locale: Option<&str>) -> bool {
        !self
            .collect_tibetan_stack_lookups(locale, &[*b"abvs", *b"blws"])
            .is_empty()
    }

    pub(crate) fn preprocess_tibetan_stack(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        locale: Option<&str>,
        gdef: Option<&Gdef>,
    ) -> bool {
        self.apply_tibetan_selected_glyphs(
            glyphs,
            self.collect_tibetan_stack_lookups(locale, &[*b"locl", *b"ccmp"]),
            &[],
            gdef,
        )
        .0
    }

    /// Preprocessing is cluster-scoped; presentation uses the entire bounded
    /// run, in font LookupList order, including contextual neighbors.
    pub(crate) fn apply_tibetan_stack_presentation(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        locale: Option<&str>,
        variant: crate::commands::FontVariant,
        orientation_tag: Option<[u8; 4]>,
        gdef: Option<&Gdef>,
    ) -> bool {
        self.apply_tibetan_stack_presentation_with_filtering(
            glyphs,
            locale,
            variant,
            orientation_tag,
            gdef,
            false,
        )
    }

    /// Opt-in prerequisite only. Existing callers remain on the no-skip proof.
    pub(crate) fn apply_tibetan_stack_presentation_with_filtering(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        locale: Option<&str>,
        variant: crate::commands::FontVariant,
        orientation_tag: Option<[u8; 4]>,
        gdef: Option<&Gdef>,
        filtered_context: bool,
    ) -> bool {
        let tracked = self.collect_tibetan_stack_lookups(locale, &[*b"abvs", *b"blws"]);
        let mut tags = vec![
            *b"abvs", *b"blws", *b"rlig", *b"rclt", *b"calt", *b"clig", *b"liga",
        ];
        tags.extend_from_slice(variant.gsub_feature_tags());
        let mut selected = self.collect_tibetan_stack_lookups(locale, &tags);
        if let Some(tag) = orientation_tag {
            selected.extend(self.collect_feature_lookups(None, &[tag]));
        }
        let (safe, changed) = self.apply_tibetan_selected_glyphs_with_filtering(
            glyphs,
            selected,
            &tracked,
            gdef,
            filtered_context,
        );
        safe && changed
    }

    fn apply_tibetan_selected_glyphs(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        selected: Vec<&Lookup>,
        tracked: &[&Lookup],
        gdef: Option<&Gdef>,
    ) -> (bool, bool) {
        self.apply_tibetan_selected_glyphs_with_filtering(glyphs, selected, tracked, gdef, false)
    }

    fn apply_tibetan_selected_glyphs_with_filtering(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        selected: Vec<&Lookup>,
        tracked: &[&Lookup],
        gdef: Option<&Gdef>,
        filtered_context: bool,
    ) -> (bool, bool) {
        let original = filtered_context.then(|| glyphs.clone());
        let selected: HashSet<*const Lookup> = selected
            .into_iter()
            .map(|lookup| lookup as *const Lookup)
            .collect();
        let tracked: HashSet<*const Lookup> = tracked
            .iter()
            .map(|lookup| *lookup as *const Lookup)
            .collect();
        let mut state = ApplyState::new(gdef, glyphs);
        state.filtered.enabled = filtered_context;
        // Maintain only distinct IDs, rather than rescanning the run at every
        // cursor. Counts are updated by every nested replacement as well.
        let mut counts = HashMap::new();
        for glyph in glyphs.iter() {
            *counts.entry(glyph.glyph_id).or_default() += 1;
        }
        state.exact_glyph_counts = Some(counts);
        let mut changed = false;
        for lookup in &self.lookups.lookups {
            if glyphs.is_empty() {
                break;
            }
            let pointer = lookup as *const Lookup;
            if selected.contains(&pointer) {
                let before = tracked.contains(&pointer).then(|| glyphs.clone());
                self.apply_lookup_all_with_tables(lookup, glyphs, &mut state);
                changed |= before.is_some_and(|before| {
                    before.len() != glyphs.len()
                        || before.iter().zip(glyphs.iter()).any(|(a, b)| {
                            (a.glyph_id, a.source_index, a.ligature_components)
                                != (b.glyph_id, b.source_index, b.ligature_components)
                        })
                });
                if state.ambiguous_sources || state.filtered.failed {
                    if let Some(original) = original {
                        *glyphs = original;
                        return (false, false);
                    }
                    return (false, changed);
                }
            }
        }
        (true, changed)
    }

    pub(crate) fn selected_vertical_tag(&self, mixed_upright: bool) -> [u8; 4] {
        if !mixed_upright && !self.collect_feature_lookups(None, &[*b"vrt2"]).is_empty() {
            *b"vrt2"
        } else {
            *b"vert"
        }
    }

    pub(crate) fn apply_orientation_glyphs(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        feature_tag: [u8; 4],
    ) {
        let mut selected = self.collect_feature_lookups(None, &[feature_tag]);
        if feature_tag == *b"vrt2" && selected.is_empty() {
            selected = self.collect_feature_lookups(None, &[*b"vert"]);
        }
        self.apply_selected_glyphs(glyphs, selected, None);
    }

    fn apply_selected_glyphs(
        &self,
        glyphs: &mut Vec<GsubGlyph>,
        selected: Vec<&Lookup>,
        gdef: Option<&Gdef>,
    ) {
        let selected: HashSet<*const Lookup> = selected
            .into_iter()
            .map(|lookup| lookup as *const Lookup)
            .collect();
        let mut state = ApplyState::new(gdef, glyphs);
        for lookup in &self.lookups.lookups {
            if glyphs.is_empty() {
                break;
            }
            if selected.contains(&(lookup as *const Lookup)) {
                self.apply_lookup_all_with_tables(lookup, glyphs, &mut state);
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

    pub(crate) fn apply_joining_glyphs(
        &self,
        glyphs: &mut [GsubGlyph],
        locale: Option<&str>,
        source_forms: &[Option<[u8; 4]>],
    ) {
        for glyph in glyphs {
            if let Some(Some(tag)) = source_forms.get(glyph.source_index) {
                if let Some(replacement) =
                    self.lookup_single_feature(glyph.glyph_id, locale, &[*tag])
                {
                    glyph.glyph_id = replacement;
                    glyph.substituted = true;
                    glyph.mkmk_source_ambiguous = true;
                }
            }
        }
    }

    pub(crate) fn apply_joining_sequence(
        &self,
        glyphs: &mut [(usize, usize)],
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
        let features = self.get_features(b"ccmp", script);
        for feature in features.iter() {
            for lookup_index in feature.lookup_list_indices.iter() {
                let Some(lookup) = self.lookups.lookups.get(*lookup_index as usize).cloned() else {
                    continue;
                };
                for _subtable in lookup.subtables.iter() {
                    // get glyph_ids from subtable
                }
            }
        }
        None
    }

    /// Select one legacy vertical feature: available `vrt2`, otherwise `vert`.
    /// Availability means at least one valid lookup referenced by the existing
    /// all-script / all-LangSys collector, not coverage of this particular glyph.
    /// Never use `vrtr`: it requires graphical rotation by the renderer.
    /// Apply direct / extension Single substitutions once per selected lookup,
    /// in LookupList order. Other lookup types require a sequence-aware API and
    /// are skipped, without falling back to a different vertical feature.
    /// GDEF-dependent filtering flags are unsupported here and skip the lookup.
    pub fn lookup_vertical(&self, glyph_id: u16) -> Option<u16> {
        let mut selected = self.collect_feature_lookups(None, &[*b"vrt2"]);
        if selected.is_empty() {
            selected = self.collect_feature_lookups(None, &[*b"vert"]);
        }
        self.apply_single_lookup_chain(glyph_id, selected)
    }

    /// Select only `vrtr` for a caller that graphically rotates a horizontal run.
    pub(crate) fn lookup_sideways(&self, glyph_id: u16) -> Option<u16> {
        self.apply_single_lookup_chain(glyph_id, self.collect_feature_lookups(None, &[*b"vrtr"]))
    }

    /// Select only `vert` for preclassified upright mixed-orientation groups.
    pub(crate) fn lookup_mixed_upright(&self, glyph_id: u16) -> Option<u16> {
        self.apply_single_lookup_chain(glyph_id, self.collect_feature_lookups(None, &[*b"vert"]))
    }

    fn apply_single_lookup_chain(&self, glyph_id: u16, selected: Vec<&Lookup>) -> Option<u16> {
        use crate::opentype::layouts::lookup::LookupSubstitution;

        // Preserve the collector's scope, but not its feature-reference order.
        // Pointer identity is only used for membership, never for ordering.
        let selected: HashSet<*const Lookup> = selected
            .into_iter()
            .map(|lookup| lookup as *const Lookup)
            .collect();
        let mut current = glyph_id;
        let mut applied = false;
        for lookup in self.lookups.lookups.iter() {
            if !selected.contains(&(lookup as *const Lookup)) {
                continue;
            }
            // This single-glyph API has no GDEF class/mark-filtering context.
            // Do not apply a potentially excluded substitution. RIGHT_TO_LEFT
            // (0x0001) is ignored for GSUB; reserved bits do not select classes.
            if lookup.lookup_flag & 0xFF1E != 0 {
                continue;
            }
            for subtable in &lookup.subtables {
                let replacement = match subtable {
                    LookupSubstitution::Single(_) | LookupSubstitution::Single2(_)
                        if lookup.lookup_type == 1 =>
                    {
                        subtable.get_single_glyph_id(current)
                    }
                    LookupSubstitution::ExtensionSubstitution(extension)
                        if lookup.lookup_type == 7
                            && extension.subst_format == 1
                            && extension.extension_lookup_type == 1 =>
                    {
                        extension.subtable.get_single_glyph_id(current)
                    }
                    _ => None,
                };
                if let Some(replacement) = replacement {
                    current = replacement;
                    applied = true;
                    break;
                }
            }
        }
        applied.then_some(current)
    }

    pub(crate) fn apply_locale_glyph(&self, glyph: &mut GsubGlyph, locale: &str) {
        if let Some(glyph_id) =
            self.lookup_single_feature(glyph.glyph_id, Some(locale), &[*b"locl"])
        {
            glyph.glyph_id = glyph_id;
            glyph.substituted = true;
            glyph.mkmk_source_ambiguous = true;
        }
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

impl std::fmt::Display for Gsub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

#[cfg(test)]
mod vertical_feature_tests {
    #[test]
    fn vertical_lookup_chains_and_preserves_applied_identity() {
        let font = crate::fontreader::Font::get_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/vertical-lookup-chain.ttc"
        ))
        .unwrap();
        let expected = [
            Some(3),
            Some(3),
            Some(3),
            Some(2),
            Some(3),
            Some(1),
            Some(1),
            Some(2),
            None,
            None,
            None,
            None,
            None,
            Some(3),
            None,
            None,
            Some(3),
            None,
            Some(3),
            Some(3),
        ];
        for (index, replacement) in expected.into_iter().enumerate() {
            let selected = if index == 0 {
                &font
            } else {
                &font.more_fonts[index - 1]
            };
            let gsub = selected.gsub.as_ref().unwrap();
            assert_eq!(gsub.lookup_vertical(1), replacement, "face {index}");
            assert_eq!(gsub.lookup_vertical(65535), None, "face {index}");
        }
        for index in 20..42 {
            let gsub = font.more_fonts[index - 1].gsub.as_ref().unwrap();
            let expected = match index {
                40 => Some(2),
                41 => Some(3),
                _ if index % 2 == 0 => None,
                _ => Some(2),
            };
            assert_eq!(gsub.lookup_vertical(1), expected, "flag face {index}");
        }
    }

    #[test]
    fn vertical_lookup_skips_malformed_single_and_extension_subtables() {
        use crate::opentype::layouts::lookup::LookupSubstitution;
        let font = crate::fontreader::Font::get_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/vertical-lookup-chain.ttc"
        ))
        .unwrap();
        let mut direct = font.more_fonts[17].gsub.as_ref().unwrap().clone();
        if let LookupSubstitution::Single2(single) = &mut direct.lookups.lookups[0].subtables[0] {
            single.substitute_glyph_ids.clear();
        } else {
            panic!("fixture must use Single format 2");
        }
        assert_eq!(direct.lookup_vertical(1), None);
        for mode in 0..4 {
            let mut gsub = font.more_fonts[1].gsub.as_ref().unwrap().clone();
            if let LookupSubstitution::ExtensionSubstitution(extension) =
                &mut gsub.lookups.lookups[0].subtables[0]
            {
                match mode {
                    0 => extension.subst_format = 2,
                    1 => extension.extension_lookup_type = 2,
                    2 => *extension.subtable = LookupSubstitution::Unknown,
                    _ => {
                        *extension.subtable =
                            LookupSubstitution::ExtensionSubstitution(extension.clone())
                    }
                }
            } else {
                panic!("fixture must use Extension substitution");
            }
            assert_eq!(gsub.lookup_vertical(1), None, "mode {mode}");
        }
    }

    #[test]
    fn vertical_lookup_selects_one_available_feature_before_coverage() {
        let font = crate::fontreader::Font::get_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/vertical-feature-selection.ttc"
        ))
        .unwrap();
        let expected = [
            Some(3),
            Some(3),
            Some(2),
            None,
            None,
            Some(3),
            Some(2),
            Some(2),
            Some(2),
            Some(3),
            Some(3),
            None,
        ];
        for (index, replacement) in expected.into_iter().enumerate() {
            let selected = if index == 0 {
                &font
            } else {
                &font.more_fonts[index - 1]
            };
            assert_eq!(
                selected.gsub.as_ref().unwrap().lookup_vertical(1),
                replacement,
                "face {index}"
            );
        }
    }
}

#[cfg(test)]
mod scalar_feature_tests {
    use super::GsubGlyph;

    #[test]
    fn scalar_features_require_complete_ligature_input() {
        let font = crate::fontreader::Font::get_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/scalar-locl.ttc"
        ))
        .unwrap();
        // Direct and Extension: multi-input, one-input, ordered, identity,
        // next lookup, Single, Alternate, legacy Multiple, deletion, liga.
        let expected = [
            None,
            Some(3),
            Some(4),
            Some(1),
            Some(4),
            Some(3),
            Some(3),
            Some(3),
            None,
            None,
        ];
        for extension in 0..2 {
            for (mode, replacement) in expected.into_iter().enumerate() {
                let gsub = font.more_fonts[extension * 10 + mode]
                    .gsub
                    .as_ref()
                    .unwrap();
                for tag in [*b"locl", *b"isol", *b"init", *b"medi", *b"fina"] {
                    assert_eq!(
                        gsub.lookup_single_feature(1, Some("san"), &[tag]),
                        replacement,
                        "extension {extension} mode {mode} tag {tag:?}"
                    );
                }
                assert_eq!(gsub.lookup_locale(1, "san"), replacement.unwrap_or(1));
                let original = GsubGlyph {
                    glyph_id: 1,
                    source_index: 7,
                    substituted: false,
                    mkmk_source_ambiguous: false,
                    ligature_components: 0,
                };
                let mut glyph = original;
                gsub.apply_locale_glyph(&mut glyph, "san");
                let mut wanted = original;
                if let Some(id) = replacement {
                    wanted.glyph_id = id;
                    wanted.substituted = true;
                    wanted.mkmk_source_ambiguous = true;
                }
                assert_eq!(glyph, wanted, "provenance mode {mode}");
                if mode == 9 {
                    assert_eq!(gsub.lookup_standard_liga_sequence(&[1, 2]), Some(4));
                }
            }
        }
    }
    #[test]
    fn scalar_ligature_rejects_inconsistent_component_metadata() {
        use crate::opentype::layouts::lookup::LookupSubstitution;
        let font = crate::fontreader::Font::get_font_from_buffer(include_bytes!(
            "../../../tests/fixtures/scalar-locl.ttc"
        ))
        .unwrap();
        for extension in 0..2 {
            for malformed in 0..3 {
                let mut gsub = font.more_fonts[extension * 10 + 1]
                    .gsub
                    .as_ref()
                    .unwrap()
                    .clone();
                for lookup in &mut gsub.lookups.lookups {
                    for subtable in &mut lookup.subtables {
                        let inner = match subtable {
                            LookupSubstitution::ExtensionSubstitution(ext) => &mut *ext.subtable,
                            other => other,
                        };
                        if let LookupSubstitution::Ligature(lig) = inner {
                            let record = &mut lig.ligature_set[0].ligature_table[0];
                            match malformed {
                                0 => record.component_count = 0,
                                1 => record.component_count = 2,
                                _ => record.component_glyph_ids.push(2),
                            }
                        }
                    }
                }
                assert_eq!(
                    gsub.lookup_single_feature(1, Some("san"), &[*b"locl"]),
                    None
                );
            }
        }
    }
}
