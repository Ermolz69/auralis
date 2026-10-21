use adapters_tts::{MarkerClient, MarkerRuntimeConfig};
use ports::{
    marker::MarkerPort,
    speech_workflow::{SpeechCue, SpeechSourceSnapshot},
};
use sha2::{Digest, Sha256};

#[tokio::test]
#[ignore = "requires the verified local Normalizer and idle shared PUZA admission; no model loading"]
async fn installed_normalizer_prepares_text_and_subtitles_sequentially()
-> Result<(), Box<dyn std::error::Error>> {
    let config: MarkerRuntimeConfig = serde_json::from_slice(&std::fs::read(std::env::var(
        "AURALIS_TEST_MARKER_RUNTIME",
    )?)?)?;
    let client = MarkerClient {
        config: config.clone(),
    };
    let text = "Расстояние 12 км.";
    let original = SpeechSourceSnapshot {
        kind: "text".into(),
        source_id: None,
        revision: "fixture-text".into(),
        sha256: format!("{:x}", Sha256::digest(text.as_bytes())),
        text: text.into(),
        cues: vec![],
    };
    let prepared = client.prepare(original.clone()).await?;
    assert_eq!(prepared.original, original);
    assert_eq!(prepared.prepared.text, "Расстояние двенадцать км.");
    assert!(prepared.changed_targets > 0);
    assert_eq!(
        prepared.prepared.sha256,
        format!("{:x}", Sha256::digest(prepared.prepared.text.as_bytes()))
    );
    let mut subtitles = original;
    subtitles.kind = "subtitles".into();
    subtitles.cues = vec![
        SpeechCue {
            id: "first".into(),
            text: text.into(),
            role: "Alex".into(),
            start_ms: 100,
            end_ms: 1900,
        },
        SpeechCue {
            id: "second".into(),
            text: "Обычный русский текст.".into(),
            role: "Marina".into(),
            start_ms: 2200,
            end_ms: 4100,
        },
    ];
    subtitles.text = subtitles
        .cues
        .iter()
        .map(|cue| cue.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    subtitles.sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(&subtitles.cues)?));
    let cues = client.prepare(subtitles.clone()).await?;
    for (expected, actual) in subtitles.cues.iter().zip(&cues.prepared.cues) {
        assert_eq!(
            (
                &expected.id,
                &expected.role,
                expected.start_ms,
                expected.end_ms
            ),
            (&actual.id, &actual.role, actual.start_ms, actual.end_ms)
        );
    }
    assert_eq!(cues.prepared.cues[0].text, prepared.prepared.text);
    assert_eq!(cues.prepared.cues[1].text, subtitles.cues[1].text);
    let mut changed = config;
    changed.release_sha256 = "f".repeat(64);
    assert!(
        MarkerClient { config: changed }
            .prepare(subtitles)
            .await
            .is_err()
    );
    if let Ok(path) = std::env::var("AURALIS_TEST_MARKER_EVIDENCE") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({"text":prepared,"subtitles":cues}))?,
        )?;
    }
    Ok(())
}
