use crate::translation_result_edit::TranslationResultEdit;
use uuid::Uuid;

pub struct TranslationBranchEdit {
    pub edit: TranslationResultEdit,
    pub expected_head_result_id: Uuid,
}
