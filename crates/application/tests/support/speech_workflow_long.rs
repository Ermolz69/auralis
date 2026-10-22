use super::*;
use application::usecases::translation::{
    ImportTranslationSourceRequest, ImportTranslationSourceUseCase,
};

fn timestamp(second: usize) -> String {
    format!(
        "{:02}:{:02}:{:02},000",
        second / 3600,
        second / 60 % 60,
        second % 60
    )
}

#[tokio::test]
async fn six_hundred_cues_keep_exact_order_assignments_and_review_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, mut cases, _, _) = fixture().await?;
    let text: String = (1..=600)
        .map(|index| {
            format!(
                "{index}\n{} --> {}\nРусская реплика {index}.\n\n",
                timestamp(index * 2),
                timestamp(index * 2 + 1)
            )
        })
        .collect();
    let original = fixture.directory.path().join("long-original.srt");
    tokio::fs::write(&original, text.as_bytes()).await?;
    let artifact = ImportTranslationSourceUseCase::new(
        Arc::new(SqliteProjectRepository::new(fixture.pool.clone())),
        fixture.files.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(fixture.pool.clone())),
        Arc::new(adapters_translate::FormatInspector),
    )
    .execute(ImportTranslationSourceRequest {
        project_id: fixture.project_id.clone(),
        path: original.clone(),
        format: TranslationSourceFormat::Srt,
    })
    .await?;
    fixture.flush().await?;
    let source = SpeechSourceRequest {
        kind: "subtitles".into(),
        text: None,
        translation_id: None,
        artifact_id: Some(artifact.id),
        format: Some("srt".into()),
    };
    let review = cases
        .review(fixture.project_id.clone(), source.clone(), true)
        .await?;
    assert_eq!(review.source.cues.len(), 600);
    let mut prepare = request(&review);
    prepare.assignments = review
        .source
        .cues
        .iter()
        .enumerate()
        .map(|(index, cue)| SpeechAssignment {
            scope: "cue".into(),
            target: cue.id.clone(),
            voice_id: if index % 2 == 0 {
                "marina_elevenlabs_ru"
            } else {
                "alex_analyst_elevenlabs_ru"
            }
            .into(),
            reference_id: if index % 3 == 0 { "joy" } else { "neutral" }.into(),
        })
        .collect();
    let prepared = cases.prepare(fixture.project_id.clone(), prepare).await?;
    assert_eq!(prepared.utterance_count, 600);
    let intent = cases
        .store
        .get_run(prepared.run_id)
        .await?
        .ok_or("missing long plan")?;
    let plan: serde_json::Value = serde_json::from_str(&intent.plan_json)?;
    assert_eq!(
        plan["review"]["decision_id"],
        review.decision_id.to_string()
    );
    assert_eq!(plan["source"]["source_sha256"], review.source.sha256);
    let utterances = plan["utterances"].as_array().ok_or("missing utterances")?;
    assert_eq!(utterances.len(), 600);
    let cast: std::collections::HashMap<_, _> = plan["cast"]
        .as_array()
        .ok_or("missing cast")?
        .iter()
        .map(|entry| {
            (
                entry["role_id"].as_str().unwrap_or_default(),
                entry["voice_id"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    for (index, utterance) in utterances.iter().enumerate() {
        assert_eq!(utterance["cue_id"], format!("cue-{}", index + 1));
        assert_eq!(utterance["text"], format!("Русская реплика {}.", index + 1));
        assert_eq!(utterance["start_ms"], ((index + 1) * 2000) as u64);
        let role = utterance["role_id"].as_str().ok_or("missing role")?;
        assert_eq!(
            cast.get(role).copied(),
            Some(if index % 2 == 0 {
                "marina_elevenlabs_ru"
            } else {
                "alex_analyst_elevenlabs_ru"
            })
        );
        assert_eq!(
            utterance["reference_id"],
            if index % 3 == 0 { "joy" } else { "neutral" }
        );
    }
    assert_eq!(tokio::fs::read(&original).await?, text.as_bytes());
    cases.limits = Ok(application::usecases::speech::SpeechLimits::from_json(
        br#"{"max_utterances":599}"#,
    )?);
    let rejection = cases
        .source(&fixture.project_id, source.clone())
        .await
        .err()
        .ok_or("expected configured cap")?;
    assert!(rejection.to_string().contains("599 cues"));
    assert!(
        cases
            .prepare(fixture.project_id.clone(), request(&review))
            .await
            .is_err()
    );
    assert_eq!(cases.store.get_run(prepared.run_id).await?, Some(intent));
    cases.limits = Err("invalid configuration fixture".into());
    assert!(
        cases
            .source(&fixture.project_id, source)
            .await
            .err()
            .ok_or("expected invalid configuration")?
            .to_string()
            .contains("correct the file and restart")
    );
    Ok(())
}
