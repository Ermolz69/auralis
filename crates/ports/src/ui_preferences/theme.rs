#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTheme {
    pub value: String,
    pub revision: u64,
}
