use uuid::Uuid;

pub struct TranslationResultEdit {
    pub base_result_id: Uuid,
    pub result_id: Uuid,
    pub segment_id: u32,
    pub lines: Vec<String>,
}
