#[derive(Debug, Clone)]
pub(crate) struct ConditionTable {
    pub(crate) format: u16,
    pub(crate) axis_index: u16,
    pub(crate) filter_range_min_value: f32,
    pub(crate) filter_range_max_value: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct ConditionSet {
    pub(crate) condition_count: u16,
    pub(crate) conditions: Box<Vec<ConditionTable>>,
}
