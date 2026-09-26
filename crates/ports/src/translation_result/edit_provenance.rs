use uuid::Uuid;

pub struct TranslationEditProvenance {
    pub base_result_id: Uuid,
    pub observed_head_result_id: Uuid,
    pub segment_id: u32,
}
