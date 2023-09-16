// GSUB -- Glyph Substitution Table

// only check lookup Format1.1 , 4, 6.1, 7

use std::io::SeekFrom;

use crate::opentype::layouts::*;
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
    pub(crate) fn new<R: BinaryReader>(reader: &mut R, offset: u32, length: u32) -> Self {
        let offset = offset as u64;

        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
        let major_version = reader.read_u16_be().unwrap();
        let minor_version = reader.read_u16_be().unwrap();
        let script_list_offset = reader.read_u16_be().unwrap();
        let feature_list_offset = reader.read_u16_be().unwrap();
        let lookup_list_offset = reader.read_u16_be().unwrap();
        let feature_variations_offset = if major_version == 1 && minor_version == 1 {
            reader.read_u16_be().unwrap()
        } else {
            0
        };

        let scripts = Box::new(ScriptList::new(
            reader,
            offset + script_list_offset as u64,
            length,
        ));
        let features = Box::new(FeatureList::new(
            reader,
            offset + feature_list_offset as u64,
            length,
        ));
        let lookups = Box::new(LookupList::new(
            reader,
            offset + lookup_list_offset as u64,
            length,
        ));
        let feature_variations = if feature_variations_offset > 0 {
            Some(Box::new(FeatureVariationList::new(
                reader,
                offset + feature_variations_offset as u64,
                length,
            )))
        } else {
            None
        };
        Self {
            major_version,
            minor_version,
            scripts,
            features,
            lookups,
            feature_variations,
        }
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

    // ccmp
    pub fn lookup_ccmp(&self, griph_ids: usize) -> Option<usize> {
        let features = self.features.get_features(b"ccmp");
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
    pub fn lookup_vertical(&self, griph_ids: usize) -> usize {
        todo!("lookup_vertical")
    }

    // locl
    pub fn lookup_locale(&self, griph_ids: usize, locale: &String) -> usize {
        todo!("lookup_locale")
    }

    // liga
    pub fn lookup_liga(&self, griph_ids: usize) -> usize {
        todo!("lookup_liga")
    }

    // hwid, fwid, qwid, twid, pkna
    pub fn lookup_width(&self, griph_ids: usize, tag: u32) -> usize {
        todo!("lookup_width")
    }

    // dnom, numr, frac, subs, sups, zero
    pub fn lookup_number(&self, griph_ids: Vec<usize>) -> usize {
        todo!("lookup_number")
    }
}
