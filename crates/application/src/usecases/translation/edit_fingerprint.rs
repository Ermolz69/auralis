use ports::translation_edit_intents::TranslationEditIntent;
use sha2::{Digest, Sha256};

const REQUEST_DOMAIN: &str = "auralis-historical-edit-v1";

pub(super) fn digest(intent: &TranslationEditIntent, segment_id: u32, lines: &[String]) -> String {
    let mut hash = Sha256::new();
    for value in [
        REQUEST_DOMAIN.to_owned(),
        intent.project_id.to_string(),
        intent.translation_id.to_string(),
        intent.result_id.to_string(),
        intent.base_result_id.to_string(),
        intent.run_id.to_string(),
        intent.expected_head_result_id.to_string(),
    ] {
        append(&mut hash, value.as_bytes());
    }
    hash.update(intent.expected_link_revision.to_be_bytes());
    hash.update(segment_id.to_be_bytes());
    hash.update((lines.len() as u64).to_be_bytes());
    for line in lines {
        append(&mut hash, line.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn append(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}
