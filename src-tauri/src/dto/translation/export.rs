use application::usecases::translation::ExportTranslationResultReceipt;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationExportReceiptDto {
    pub result_id: String,
    pub result_sha256: String,
    pub size_bytes: u64,
    pub destination: String,
}

impl From<ExportTranslationResultReceipt> for TranslationExportReceiptDto {
    fn from(value: ExportTranslationResultReceipt) -> Self {
        Self {
            result_id: value.result_id.to_string(),
            result_sha256: value.result_sha256,
            size_bytes: value.size_bytes,
            destination: value.destination.to_string_lossy().into_owned(),
        }
    }
}
