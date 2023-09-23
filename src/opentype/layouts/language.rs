#[derive(Debug, Clone)]
pub(crate) struct LanguageSystem {
    pub(crate) lookup_order_offset: u16,
    pub(crate) required_feature_index: u16,
    pub(crate) feature_index_count: u16,
    pub(crate) feature_indexes: Vec<u16>,
}
impl LanguageSystem {
    pub(crate) fn to_string(&self) -> String {
        let mut string = format!("LanguageSystem: {}\n", self.lookup_order_offset);
        string += &format!("{}\n", self.required_feature_index);
        string += &format!("{}\n", self.feature_index_count);
        for feature_index in self.feature_indexes.iter() {
            string += &format!("{}\n", feature_index);
        }
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
        let mut string = format!("LanguageSystem: {}\n", self.language_system_tag);
        string += &format!("{}\n", self.language_system.to_string());
        string
    }
}
