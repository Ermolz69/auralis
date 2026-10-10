use std::path::PathBuf;
use uuid::Uuid;

pub struct ExportTranslationResultReceipt {
    pub result_id: Uuid,
    pub result_sha256: String,
    pub size_bytes: u64,
    pub destination: PathBuf,
}
