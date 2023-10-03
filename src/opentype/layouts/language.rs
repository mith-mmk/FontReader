#[derive(Debug, Clone)]
pub(crate) struct LanguageSystem {
    pub(crate) lookup_order_offset: u16, // 0
    pub(crate) required_feature_index: u16,
    pub(crate) feature_index_count: u16,
    pub(crate) feature_indexes: Vec<u16>,
}
impl LanguageSystem {
    pub(crate) fn to_string(&self) -> String {
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
    pub(crate) fn to_string(&self) -> String {
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
        string += &format!("{}\n", self.language_system.to_string());

        string
    }
}
