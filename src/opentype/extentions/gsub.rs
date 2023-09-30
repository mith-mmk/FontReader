// GSUB -- Glyph Substitution Table

// only check lookup Format1.1 , 4, 5, 6.1, 7

use std::io::SeekFrom;

use crate::opentype::layouts::{feature::Feature, lookup::Lookup, script::ParsedScript, *};
use bin_rs::reader::BinaryReader;

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

    pub fn get_script(&self, tag: &[u8;4]) -> Option<&ParsedScript> {
        self.scripts.get_script(tag)
    }

    pub fn get_features(&self,tag: &[u8;4], script: &ParsedScript) -> Vec<&Feature> {
        let mut features = Vec::new();
        let language_system = &script.language_systems[0];
        for feature_index in language_system.language_system.feature_indexes.iter() {
            if self.features.features[*feature_index as usize].feature_tag == u32::from_be_bytes(*tag) {
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

    // ccmp Glyph Composition / Decomposition
    pub fn lookup_ccmp(&self, griph_ids: usize, script: &ParsedScript) -> Option<usize> {
        let script = self.get_script(b"DFLT").unwrap();
        let features = self.get_features(&b"ccmp", script, );
        for feature in features.iter() {
            for lookup_index in feature.lookup_list_indices.iter() {
                let lookup = self.lookups.lookups[*lookup_index as usize].clone();
                for subtable in lookup.subtables.iter() {
                    let (coverage, _) = subtable.get_coverage();
                    if let Some(id) = coverage.contains(griph_ids) {
                        return Some(id);
                    }
                }
            }
        }
        None
    }

    // vert, vrt2, vrtr
    pub fn lookup_vertical(&self, _griph_ids: usize) -> usize {
        todo!("lookup_vertical")
    }

    // locl
    pub fn lookup_locale(&self, _griph_ids: usize, _locale: &String) -> usize {
        todo!("lookup_locale")
    }

    // liga
    pub fn lookup_liga(&self, _griph_ids: usize) -> usize {
        todo!("lookup_liga")
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
