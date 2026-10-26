use super::*;
use application::usecases::translation::{
    ImportTranslationSourceRequest, ImportTranslationSourceUseCase,
};
use serde_json::{Value, json};

struct ChangedPlan {
    engine: Arc<Engine>,
    change: &'static str,
}

fn change_plan(plan: &mut Value, change: &str) -> Result<(), Box<dyn std::error::Error>> {
    match change {
        "text" => plan["utterances"][0]["text"] = json!("Unapproved replacement"),
        "timing" => plan["utterances"][0]["start_ms"] = json!(999),
        "order" => plan["utterances"]
            .as_array_mut()
            .ok_or("missing utterances")?
            .reverse(),
        "missing" => {
            plan["utterances"]
                .as_array_mut()
                .ok_or("missing utterances")?
                .pop();
        }
        "duplicate" => {
            let cue = plan["utterances"][0].clone();
            plan["utterances"]
                .as_array_mut()
                .ok_or("missing utterances")?
                .push(cue);
        }
        "revision" => plan["source"]["text_revision"] = json!("another-revision"),
        "decision" => plan["review"]["status"] = json!("rejected"),
        "voice" => plan["cast"][0]["voice_id"] = json!("another-voice"),
        "permission" => plan["review"]["technical_reference_permission"] = json!("added"),
        "profile" => plan["profile"]["profile_id"] = json!("another-profile"),
        _ => return Err("unknown fixture mutation".into()),
    }
    Ok(())
}

#[async_trait]
impl SpeechExecutionPort for ChangedPlan {
    async fn compose(
        &self,
        project: ProjectId,
        run: Uuid,
        profile: String,
        draft: String,
    ) -> Result<SpeechRunIntent, PortError> {
        let mut intent = self.engine.compose(project, run, profile, draft).await?;
        let invalid = |error: String| PortError::InvalidSource { message: error };
        let mut plan: Value =
            serde_json::from_str(&intent.plan_json).map_err(|e| invalid(e.to_string()))?;
        change_plan(&mut plan, self.change).map_err(|e| invalid(e.to_string()))?;
        intent.plan_json = plan.to_string();
        intent.plan_sha256 = hash(intent.plan_json.as_bytes());
        intent.expected_utterances = plan["utterances"]
            .as_array()
            .ok_or_else(|| invalid("missing utterances".into()))?
            .len() as u32;
        intent.expected_chunks = intent.expected_utterances;
        Ok(intent)
    }

    async fn validate(&self, intent: &SpeechRunIntent) -> Result<(), PortError> {
        self.engine.validate(intent).await
    }

    async fn execute(
        &self,
        intent: SpeechRunIntent,
        attempt: Uuid,
        observer: Arc<dyn SpeechProgressObserver>,
        cancel: CancellationToken,
    ) -> Result<SpeechExecutionResult, PortError> {
        self.engine.execute(intent, attempt, observer, cancel).await
    }
}

async fn subtitle_review(
    fixture: &HistoricalFixture,
    cases: &SpeechUseCases,
) -> Result<ports::speech_workflow::SpeechReview, Box<dyn std::error::Error>> {
    let artifact = ImportTranslationSourceUseCase::new(
        Arc::new(SqliteProjectRepository::new(fixture.pool.clone())),
        fixture.files.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(fixture.pool.clone())),
        Arc::new(adapters_translate::FormatInspector),
    )
    .execute(ImportTranslationSourceRequest {
        project_id: fixture.project_id.clone(),
        path: fixture.source_path.clone(),
        format: TranslationSourceFormat::Srt,
    })
    .await?;
    fixture.flush().await?;
    Ok(cases
        .review(
            fixture.project_id.clone(),
            SpeechSourceRequest {
                kind: "subtitles".into(),
                text: None,
                translation_id: None,
                artifact_id: Some(artifact.id),
                format: Some("srt".into()),
            },
            true,
        )
        .await?)
}

#[tokio::test]
async fn composed_plan_cannot_change_approved_cues_or_selected_voices()
-> Result<(), Box<dyn std::error::Error>> {
    for change in [
        "text",
        "timing",
        "order",
        "missing",
        "duplicate",
        "revision",
        "decision",
        "voice",
        "permission",
        "profile",
    ] {
        let (fixture, mut cases, engine, _) = fixture().await?;
        let review = subtitle_review(&fixture, &cases).await?;
        cases.engine = Some(Arc::new(ChangedPlan { engine, change }));
        assert!(
            cases
                .prepare(fixture.project_id.clone(), request(&review))
                .await
                .is_err(),
            "accepted composed plan with changed {change}"
        );
        assert!(cases.list(fixture.project_id).await?.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn start_rechecks_persisted_plan_against_the_exact_review()
-> Result<(), Box<dyn std::error::Error>> {
    for change in [
        "text",
        "timing",
        "order",
        "missing",
        "duplicate",
        "revision",
        "decision",
        "profile",
    ] {
        let (fixture, cases, _, _) = fixture().await?;
        let review = subtitle_review(&fixture, &cases).await?;
        let prepared = cases
            .prepare(fixture.project_id.clone(), request(&review))
            .await?;
        let intent = cases
            .store
            .get_run(prepared.run_id)
            .await?
            .ok_or("missing plan")?;
        let mut plan: Value = serde_json::from_str(&intent.plan_json)?;
        change_plan(&mut plan, change)?;
        let bytes = plan.to_string();
        sqlx::query("UPDATE speech_run_intents SET plan_json=?, plan_sha256=? WHERE run_id=?")
            .bind(&bytes)
            .bind(hash(bytes.as_bytes()))
            .bind(prepared.run_id.to_string())
            .execute(&fixture.pool)
            .await?;
        assert!(
            cases
                .start(fixture.project_id.clone(), prepared.run_id, false)
                .await
                .is_err(),
            "started persisted plan with changed {change}"
        );
        assert!(cases.store.latest_job(prepared.run_id).await?.is_none());
    }
    Ok(())
}
