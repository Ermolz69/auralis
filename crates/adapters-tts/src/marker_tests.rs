use crate::{marker::build_preparation, marker_lock::release_owned};
use ports::speech_workflow::{SpeechCue, SpeechSourceSnapshot};
use serde_json::json;

fn source() -> SpeechSourceSnapshot {
    SpeechSourceSnapshot {
        kind: "subtitles".into(),
        source_id: None,
        revision: "r1".into(),
        sha256: "a".repeat(64),
        text: "12 км".into(),
        cues: vec![SpeechCue {
            id: "cue-1".into(),
            text: "12 км".into(),
            role: "narrator".into(),
            start_ms: 500,
            end_ms: 2500,
        }],
    }
}

#[test]
fn normalization_preserves_cue_identity_window_and_counts_only_real_changes() {
    let original = source();
    let result = json!({"blocks":[{"id":"cue-1","text":"двенадцать километров",
        "targets":[{"id":"n1","raw":"12"},{"id":"n2","raw":"км"}],
        "replacements":[{"id":"n1","text":"двенадцать"},{"id":"n2","text":"км"}],
        "unresolved":[{"id":"n2"}]}]});
    let prepared = build_preparation(original.clone(), &result, true).unwrap();
    assert_eq!(prepared.original, original);
    assert_eq!(prepared.changed_targets, 1);
    assert_eq!(prepared.unresolved_targets, 1);
    assert_eq!(prepared.prepared.cues[0].start_ms, 500);
    assert_eq!(prepared.prepared.cues[0].end_ms, 2500);
    assert_eq!(prepared.prepared.cues[0].role, "narrator");
    assert_eq!(prepared.prepared.text, "двенадцать километров");
    assert_ne!(prepared.prepared.sha256, original.sha256);
    let mut reordered = result;
    reordered["blocks"][0]["id"] = json!("foreign-cue");
    assert!(build_preparation(original.clone(), &reordered, true).is_err());
    assert!(build_preparation(original, &json!({"blocks":[]}), true).is_err());
}

#[tokio::test]
async fn exited_worker_cleanup_preserves_other_job_lock() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Normalizer");
    let lab = temporary.path().join("Algorithm Lab");
    tokio::fs::create_dir_all(&root).await.unwrap();
    tokio::fs::create_dir_all(&lab).await.unwrap();
    let lock = lab.join(".normalizer.lock");
    tokio::fs::write(&lock, "456").await.unwrap();
    release_owned(&root, Some(123)).await.unwrap();
    assert_eq!(tokio::fs::read_to_string(&lock).await.unwrap(), "456");
    release_owned(&root, Some(456)).await.unwrap();
    assert!(!lock.exists());
    release_owned(&root, Some(456)).await.unwrap();
}
