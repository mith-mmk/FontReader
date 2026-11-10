#[derive(Debug, Clone)]
pub(crate) struct LanguageSystem {
    pub(crate) lookup_order_offset: u16, // 0
    pub(crate) required_feature_index: u16,
    pub(crate) feature_index_count: u16,
    pub(crate) feature_indexes: Vec<u16>,
}
impl LanguageSystem {
    fn dump_text(&self) -> String {
        let mut string = format!("lookup_order_offset {}\n", self.lookup_order_offset);
        string += &format!("required_feature_index {}\n", self.required_feature_index);
        string += &format!("feature_index_count {}\n", self.feature_index_count);
        string += &format!("feature_indexes {:?}\n", self.feature_indexes);

        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LanguageSystemRecord {
    pub(crate) language_system_tag: u32,
    pub(crate) language_system: LanguageSystem,
}

impl LanguageSystemRecord {
    fn dump_text(&self) -> String {
        let tag = if self.language_system_tag == 0 {
            "Default".to_string()
        } else {
            let bytes = self.language_system_tag.to_be_bytes();
            String::from_utf8_lossy(&bytes).to_string()
        };
        let mut string = format!(
            "LanguageSystem Tag: {} {:04x}\n",
            tag, self.language_system_tag
        );
        string += &format!("{}\n", self.language_system);

        string
    }
}

impl std::fmt::Display for LanguageSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

impl std::fmt::Display for LanguageSystemRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn display_preserves_nested_language_dump() {
        let record = LanguageSystemRecord {
            language_system_tag: 0,
            language_system: LanguageSystem {
                lookup_order_offset: 0,
                required_feature_index: 65535,
                feature_index_count: 2,
                feature_indexes: vec![1, 3],
            },
        };
        let expected = "LanguageSystem Tag: Default 0000\nlookup_order_offset 0\nrequired_feature_index 65535\nfeature_index_count 2\nfeature_indexes [1, 3]\n\n";
        assert_eq!(record.to_string(), expected);
        assert_eq!(format!("{record}"), expected);
    }
}
