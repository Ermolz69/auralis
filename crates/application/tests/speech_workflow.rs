#[allow(dead_code)]
#[path = "support/historical_control.rs"]
mod historical_control;
#[allow(dead_code)]
#[path = "support/historical_fixture.rs"]
mod historical_fixture;
#[allow(dead_code)]
#[path = "support/historical_provider.rs"]
mod historical_provider;
#[allow(dead_code)]
#[path = "support/historical_seed.rs"]
mod historical_seed;

#[path = "support/speech_workflow_long.rs"]
mod long_sources;

#[path = "support/speech_layout_publication.rs"]
mod layout_publication;

use adapters_storage::sqlite::{
    SqliteJobRepository, SqliteProjectRepository, SqliteSpeechRunStore, SqliteStorageUnitOfWork,
    SqliteTranslationReviewStore,
};
use application::usecases::{
    speech::{PrepareSpeechRequest, SpeechAssignment, SpeechSourceRequest, SpeechUseCases},
    translation::TranslationReviewUseCase,
};
use async_trait::async_trait;
use domain::project::ProjectId;
use historical_fixture::HistoricalFixture;
use ports::{
    cancellation::CancellationToken,
    error::PortError,
    job_scheduler::JobSchedulerPort,
    speech_catalog::{
        SpeechCatalogPort, SpeechCatalogSnapshot, SpeechProfileOption, SpeechReferenceOption,
        SpeechVoiceOption,
    },
    speech_run::{SpeechCheckpoint, SpeechRunIntent},
    speech_workflow::{
        SpeechExecutionPort, SpeechExecutionResult, SpeechProducedFile, SpeechProgressObserver,
    },
    translation_inspection::TranslationSourceFormat,
};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::Notify;
use uuid::Uuid;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
struct Catalog {
    qualified: bool,
}
#[async_trait]
impl SpeechCatalogPort for Catalog {
    async fn snapshot(&self, _: &ProjectId) -> Result<SpeechCatalogSnapshot, PortError> {
        let mut snapshot = SpeechCatalogSnapshot {
            profiles: vec![SpeechProfileOption {
                profile_id: "native-plan-voxcpm2".into(),
                profile_sha256: "a".repeat(64),
                engine_id: "voxcpm2".into(),
            }],
            voices: vec![SpeechVoiceOption {
                voice_id: "marina_elevenlabs_ru".into(),
                display_name: "Marina".into(),
                description: "Fixture".into(),
                manifest_sha256: "b".repeat(64),
                references: vec![
                    SpeechReferenceOption {
                        reference_id: "neutral".into(),
                        engine_id: "voxcpm2".into(),
                        emotion_id: "neutral".into(),
                        transcript: "Fixture".into(),
                        audio_sha256: "c".repeat(64),
                        qualification_status: if self.qualified {
                            "qualified"
                        } else {
                            "unverified"
                        }
                        .into(),
                    },
                    SpeechReferenceOption {
                        reference_id: "joy".into(),
                        engine_id: "voxcpm2".into(),
                        emotion_id: "joy".into(),
                        transcript: "Joy fixture".into(),
                        audio_sha256: "d".repeat(64),
                        qualification_status: if self.qualified {
                            "qualified"
                        } else {
                            "unverified"
                        }
                        .into(),
                    },
                ],
            }],
        };
        let mut alex_analyst_elevenlabs_ru = snapshot.voices[0].clone();
        alex_analyst_elevenlabs_ru.voice_id = "alex_analyst_elevenlabs_ru".into();
        alex_analyst_elevenlabs_ru.display_name = "Alex".into();
        snapshot.voices.push(alex_analyst_elevenlabs_ru);
        Ok(snapshot)
    }
}
struct Engine {
    root: PathBuf,
    hold: AtomicBool,
    checkpoint: Notify,
    invalid: AtomicBool,
    include_layout: AtomicBool,
}
#[async_trait]
impl SpeechExecutionPort for Engine {
    async fn compose(
        &self,
        project: ProjectId,
        run_id: Uuid,
        profile_id: String,
        draft: String,
    ) -> Result<SpeechRunIntent, PortError> {
        let mut value: serde_json::Value =
            serde_json::from_str(&draft).map_err(|e| PortError::InvalidSource {
                message: e.to_string(),
            })?;
        value["plan_digest"] = serde_json::json!(hash(draft.as_bytes()));
        let count = value["utterances"]
            .as_array()
            .ok_or_else(|| PortError::InvalidSource {
                message: "Missing fixture utterances".into(),
            })?
            .len() as u32;
        let plan_json = value.to_string();
        Ok(SpeechRunIntent {
            run_id,
            project_id: project,
            plan_sha256: hash(plan_json.as_bytes()),
            plan_json,
            profile_id,
            profile_sha256: "a".repeat(64),
            expected_utterances: count,
            expected_chunks: count,
        })
    }
    async fn validate(&self, _: &SpeechRunIntent) -> Result<(), PortError> {
        if self.invalid.load(Ordering::SeqCst) {
            Err(PortError::InvalidSource {
                message: "Changed profile/reference/hash".into(),
            })
        } else {
            Ok(())
        }
    }
    async fn execute(
        &self,
        intent: SpeechRunIntent,
        attempt_id: Uuid,
        observer: Arc<dyn SpeechProgressObserver>,
        cancel: CancellationToken,
    ) -> Result<SpeechExecutionResult, PortError> {
        observer
            .checkpoint(SpeechCheckpoint {
                run_id: intent.run_id,
                attempt_id,
                plan_sha256: intent.plan_sha256,
                sequence: 1,
                utterance_id: "utterance-0001".into(),
                cue_id: None,
                chunk_id: "utterance-0001-chunk-0001".into(),
                cache_key: "c".repeat(64),
                audio_sha256: "d".repeat(64),
                duration_ms: 100,
                origin: if self.hold.load(Ordering::SeqCst) {
                    "generated"
                } else {
                    "cached"
                }
                .into(),
            })
            .await?;
        self.checkpoint.notify_one();
        if self.hold.load(Ordering::SeqCst) {
            cancel.cancelled().await;
            return Ok(SpeechExecutionResult::Cancelled);
        }
        let bytes = b"model-free fixture bytes";
        let file = self.root.join("result.wav");
        tokio::fs::write(&file, bytes)
            .await
            .map_err(|e| PortError::Io {
                message: e.to_string(),
            })?;
        let mut kinds = vec!["wav", "manifest", "segment"];
        if self.include_layout.load(Ordering::SeqCst) {
            kinds.push("audio_layout");
        }
        Ok(SpeechExecutionResult::Completed {
            duration_ms: 100,
            files: kinds
                .into_iter()
                .map(|kind| SpeechProducedFile {
                    kind: kind.into(),
                    path: file.clone(),
                    sha256: hash(bytes),
                })
                .collect(),
        })
    }
}

async fn fixture() -> Result<
    (
        HistoricalFixture,
        SpeechUseCases,
        Arc<Engine>,
        Arc<jobs::manager::JobManager>,
    ),
    Box<dyn std::error::Error>,
> {
    let fixture = HistoricalFixture::new(TranslationSourceFormat::Srt).await?;
    let manager = Arc::new(jobs::manager::JobManager::new(
        Arc::new(SqliteJobRepository::new(fixture.pool.clone())),
        Arc::new(SqliteStorageUnitOfWork::new(fixture.pool.clone())),
        None,
    ));
    let engine = Arc::new(Engine {
        root: fixture.directory.path().into(),
        hold: AtomicBool::new(true),
        checkpoint: Notify::new(),
        invalid: AtomicBool::new(false),
        include_layout: AtomicBool::new(false),
    });
    let cases = SpeechUseCases {
        limits: Ok(Default::default()),
        marker: None,
        marker_error: None,
        contract: None,
        projects: Arc::new(SqliteProjectRepository::new(fixture.pool.clone())),
        store: Arc::new(SqliteSpeechRunStore::new(fixture.pool.clone())),
        engine: Some(engine.clone()),
        catalog: Some(Arc::new(Catalog { qualified: true })),
        runtime_error: None,
        runtime: manager.clone(),
        artifacts: fixture.files.clone(),
        index: fixture.artifacts.clone(),
        translation_review: Arc::new(TranslationReviewUseCase::new(
            fixture.verifier(),
            Arc::new(SqliteTranslationReviewStore::new(fixture.pool.clone())),
        )),
        subtitles: fixture.inspector(),
        emitter: Arc::new(|_| {}),
    };
    Ok((fixture, cases, engine, manager))
}

fn source() -> SpeechSourceRequest {
    SpeechSourceRequest {
        kind: "text".into(),
        text: Some("Привет, мир. [voice: literal data] 🦊".into()),
        translation_id: None,
        artifact_id: None,
        format: None,
    }
}
fn request(review: &ports::speech_workflow::SpeechReview) -> PrepareSpeechRequest {
    PrepareSpeechRequest {
        decision_id: review.decision_id,
        expected_source_sha256: review.source.sha256.clone(),
        profile_id: "native-plan-voxcpm2".into(),
        voice_id: "marina_elevenlabs_ru".into(),
        reference_id: "neutral".into(),
        assignments: vec![],
    }
}

#[tokio::test]
async fn qualified_emotional_reference_keeps_its_emotion_in_the_canonical_plan()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, cases, _, _) = fixture().await?;
    let review = cases
        .review(fixture.project_id.clone(), source(), true)
        .await?;
    let mut emotional = request(&review);
    emotional.reference_id = "joy".into();
    let prepared = cases.prepare(fixture.project_id.clone(), emotional).await?;
    let intent = cases
        .store
        .get_run(prepared.run_id)
        .await?
        .ok_or("Saved plan missing")?;
    let plan: serde_json::Value = serde_json::from_str(&intent.plan_json)?;
    assert_eq!(plan["utterances"][0]["reference_id"], "joy");
    assert_eq!(plan["utterances"][0]["emotion_id"], "joy");
    Ok(())
}

#[tokio::test]
async fn approved_srt_voice_overrides_preserve_cues_and_reject_changed_selection()
-> Result<(), Box<dyn std::error::Error>> {
    approved_translation_assignments(TranslationSourceFormat::Srt).await
}

#[tokio::test]
async fn approved_vtt_voice_overrides_preserve_cues_and_reject_changed_selection()
-> Result<(), Box<dyn std::error::Error>> {
    approved_translation_assignments(TranslationSourceFormat::Vtt).await
}

async fn approved_translation_assignments(
    format: TranslationSourceFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    use application::usecases::translation::{
        GetTranslationReviewRequest, RecordTranslationReviewRequest,
    };
    use ports::translation_links::{SelectTranslationHistoryResult, TranslationLinkStore};
    use ports::translation_reviews::ReviewDecisionKind;
    let (fixture, mut cases, _, _) = fixture().await?;
    let alternate;
    let fixture = if format == TranslationSourceFormat::Srt {
        &fixture
    } else {
        alternate = HistoricalFixture::new(format).await?;
        cases.projects = Arc::new(SqliteProjectRepository::new(alternate.pool.clone()));
        cases.store = Arc::new(SqliteSpeechRunStore::new(alternate.pool.clone()));
        cases.translation_review = Arc::new(TranslationReviewUseCase::new(
            alternate.verifier(),
            Arc::new(SqliteTranslationReviewStore::new(alternate.pool.clone())),
        ));
        &alternate
    };
    let source = SpeechSourceRequest {
        kind: "translation".into(),
        text: None,
        translation_id: Some(fixture.translation_id),
        artifact_id: None,
        format: None,
    };
    assert!(
        cases
            .review(fixture.project_id.clone(), source.clone(), true)
            .await
            .is_err()
    );
    let selected = cases
        .translation_review
        .get(GetTranslationReviewRequest {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.head_id,
        })
        .await?;
    let approved = cases
        .translation_review
        .record(RecordTranslationReviewRequest {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.head_id,
            expected_result_sha256: selected.result_sha256,
            expected_link_revision: selected.link_revision,
            decision: ReviewDecisionKind::Approved,
            reviewer_label: "Fixture reviewer".into(),
            note: "Model-free acceptance".into(),
        })
        .await?;
    let review = cases
        .review(fixture.project_id.clone(), source, false)
        .await?;
    let mut prepared_request = request(&review);
    let role = SpeechAssignment {
        scope: "role".into(),
        target: "Narrator".into(),
        voice_id: "alex_analyst_elevenlabs_ru".into(),
        reference_id: "neutral".into(),
    };
    let cue = SpeechAssignment {
        scope: "cue".into(),
        target: review.source.cues[0].id.clone(),
        voice_id: "marina_elevenlabs_ru".into(),
        reference_id: "neutral".into(),
    };
    let mut repeated = request(&review);
    repeated.assignments = vec![cue.clone(), cue.clone()];
    assert!(
        cases
            .prepare(fixture.project_id.clone(), repeated)
            .await
            .is_err()
    );
    let mut unknown = request(&review);
    unknown.assignments = vec![SpeechAssignment {
        voice_id: "missing".into(),
        ..role.clone()
    }];
    assert!(
        cases
            .prepare(fixture.project_id.clone(), unknown)
            .await
            .is_err()
    );
    prepared_request.assignments = vec![role.clone(), cue.clone()];
    let prepared = cases
        .prepare(fixture.project_id.clone(), prepared_request)
        .await?;
    let intent = cases
        .store
        .get_run(prepared.run_id)
        .await?
        .ok_or("Missing speech plan")?;
    let plan: serde_json::Value = serde_json::from_str(&intent.plan_json)?;
    assert_eq!(prepared.utterance_count, 2);
    assert_eq!(plan["cast"][0]["voice_id"], "marina_elevenlabs_ru");
    assert_eq!(plan["cast"][1]["voice_id"], "alex_analyst_elevenlabs_ru");
    for (q, original) in review.source.cues.iter().enumerate() {
        assert_eq!(plan["utterances"][q]["text"], original.text);
        assert_eq!(plan["utterances"][q]["cue_id"], original.id);
        assert_eq!(plan["utterances"][q]["start_ms"], original.start_ms);
        assert_eq!(plan["utterances"][q]["end_ms"], original.end_ms);
    }
    let mut repeated_plan_request = request(&review);
    repeated_plan_request.assignments = vec![role, cue];
    let repeated_plan = cases
        .prepare(fixture.project_id.clone(), repeated_plan_request)
        .await?;
    let repeated_intent = cases
        .store
        .get_run(repeated_plan.run_id)
        .await?
        .ok_or("Missing repeated speech plan")?;
    let repeated_json: serde_json::Value = serde_json::from_str(&repeated_intent.plan_json)?;
    assert_ne!(prepared.run_id, repeated_plan.run_id);
    assert_ne!(plan["plan_digest"], repeated_json["plan_digest"]);
    for q in 0..review.source.cues.len() {
        assert_eq!(
            plan["utterances"][q]["seed_key"],
            repeated_json["utterances"][q]["seed_key"]
        );
        assert_eq!(
            plan["utterances"][q]["role_id"],
            repeated_json["utterances"][q]["role_id"]
        );
    }
    fixture
        .links
        .select_historical_result(SelectTranslationHistoryResult {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.base_id,
            expected_link_revision: approved.link_revision,
            updated_at: domain::chrono::Utc::now(),
        })
        .await?;
    assert!(
        cases
            .start(fixture.project_id.clone(), prepared.run_id, false)
            .await
            .is_err()
    );
    assert!(cases.store.latest_job(prepared.run_id).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn reviewed_text_cancels_via_common_jobs_and_resumes_a_new_attempt_to_ready_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, cases, engine, manager) = fixture().await?;
    assert!(
        cases
            .review(fixture.project_id.clone(), source(), false)
            .await
            .is_err()
    );
    let review = cases
        .review(fixture.project_id.clone(), source(), true)
        .await?;
    let mut invalid = request(&review);
    invalid.assignments.push(SpeechAssignment {
        scope: "cue".into(),
        target: "missing".into(),
        voice_id: "marina_elevenlabs_ru".into(),
        reference_id: "neutral".into(),
    });
    assert!(
        cases
            .prepare(fixture.project_id.clone(), invalid)
            .await
            .is_err()
    );
    let prepared = cases
        .prepare(fixture.project_id.clone(), request(&review))
        .await?;
    assert!(
        cases
            .start(ProjectId::new(), prepared.run_id, false)
            .await
            .is_err()
    );
    engine.invalid.store(true, Ordering::SeqCst);
    assert!(
        cases
            .start(fixture.project_id.clone(), prepared.run_id, false)
            .await
            .is_err()
    );
    engine.invalid.store(false, Ordering::SeqCst);
    let started = cases
        .start(fixture.project_id.clone(), prepared.run_id, false)
        .await?;
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        engine.checkpoint.notified(),
    )
    .await?;
    assert!(
        cases
            .start(fixture.project_id.clone(), prepared.run_id, false)
            .await
            .is_err()
    );
    let job = started.job.ok_or("Job missing")?;
    let cancelled = manager.cancel_job(&job.id).await?;
    assert_eq!(cancelled.status, domain::job::JobStatus::Cancelled);
    assert!(
        cases
            .state(fixture.project_id.clone(), prepared.run_id)
            .await?
            .result
            .is_none()
    );
    engine.hold.store(false, Ordering::SeqCst);
    let resumed = cases
        .start(fixture.project_id.clone(), prepared.run_id, true)
        .await?;
    assert_ne!(resumed.job.ok_or("Resume missing")?.id, job.id);
    for _ in 0..100 {
        fixture.flush().await?;
        if cases
            .state(fixture.project_id.clone(), prepared.run_id)
            .await?
            .result
            .is_some()
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let ready = cases
        .state(fixture.project_id.clone(), prepared.run_id)
        .await?;
    assert_eq!(
        ready.job.ok_or("Ready job missing")?.status,
        domain::job::JobStatus::Completed
    );
    assert!(ready.result.ok_or("Result missing")?.ready);
    let attempts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM speech_host_attempts WHERE run_id=?")
            .bind(prepared.run_id.to_string())
            .fetch_one(&fixture.pool)
            .await?;
    assert_eq!(attempts, 2);
    Ok(())
}

#[tokio::test]
async fn prepared_text_requires_confirmation_and_keeps_an_immutable_original()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, cases, _, _) = fixture().await?;
    let original = cases.source(&fixture.project_id, source()).await?;
    let mut prepared = original.clone();
    prepared.text = "Подготовленный русский текст.".into();
    let preparation = ports::marker::MarkerPreparation {
        version: "1.0.0".into(),
        original: original.clone(),
        prepared,
        changed_targets: 1,
        unresolved_targets: 0,
    };
    assert!(
        cases
            .review_prepared(
                fixture.project_id.clone(),
                source(),
                preparation.clone(),
                false
            )
            .await
            .is_err()
    );
    let mut changed_source = preparation.clone();
    changed_source.original.sha256 = "f".repeat(64);
    assert!(
        cases
            .review_prepared(fixture.project_id.clone(), source(), changed_source, true)
            .await
            .is_err()
    );
    let mut changed_structure = preparation.clone();
    changed_structure.prepared.kind = "subtitles".into();
    assert!(
        cases
            .review_prepared(
                fixture.project_id.clone(),
                source(),
                changed_structure,
                true
            )
            .await
            .is_err()
    );
    let review = cases
        .review_prepared(fixture.project_id.clone(), source(), preparation, true)
        .await?;
    assert_eq!(review.original_source, Some(original));
    assert_eq!(review.source.text, "Подготовленный русский текст.");
    let saved = cases
        .store
        .get_review(review.decision_id)
        .await?
        .ok_or("missing reviewed preparation")?;
    assert_eq!(saved.source, review.source);
    assert_eq!(saved.original_source, review.original_source);
    Ok(())
}

#[tokio::test]
async fn missing_runtime_is_specific_and_cannot_prepare_or_start_inference()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, mut cases, _, _) = fixture().await?;
    cases.catalog = None;
    cases.engine = None;
    cases.runtime_error = Some("Python missing: T:/missing/python.exe".into());
    let review = cases
        .review(fixture.project_id.clone(), source(), true)
        .await?;
    let error = cases
        .prepare(fixture.project_id.clone(), request(&review))
        .await
        .err()
        .ok_or("Expected runtime failure")?;
    assert!(error.to_string().contains("Python missing"));
    assert!(cases.list(fixture.project_id).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn unverified_reference_cannot_prepare_a_plan() -> Result<(), Box<dyn std::error::Error>> {
    let (fixture, mut cases, _, _) = fixture().await?;
    cases.catalog = Some(Arc::new(Catalog { qualified: false }));
    let review = cases
        .review(fixture.project_id.clone(), source(), true)
        .await?;
    let error = cases
        .prepare(fixture.project_id.clone(), request(&review))
        .await
        .err()
        .ok_or("Expected human qualification failure")?;
    assert!(error.to_string().contains("requires human qualification"));
    assert!(cases.list(fixture.project_id).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn cancellation_during_publication_discards_pending_files_and_allows_cached_resume()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, cases, engine, manager) = fixture().await?;
    engine.hold.store(false, Ordering::SeqCst);
    let review = cases
        .review(fixture.project_id.clone(), source(), true)
        .await?;
    let prepared = cases
        .prepare(fixture.project_id.clone(), request(&review))
        .await?;
    let started = cases
        .start(fixture.project_id.clone(), prepared.run_id, false)
        .await?;
    for _ in 0..100 {
        if cases.store.publication(prepared.run_id).await?.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(cases.store.publication(prepared.run_id).await?.is_some());
    manager
        .cancel_job(&started.job.ok_or("Missing job")?.id)
        .await?;
    assert!(cases.store.publication(prepared.run_id).await?.is_none());
    assert!(
        cases
            .index
            .list_by_project(&fixture.project_id)
            .await?
            .iter()
            .all(|a| a.kind != domain::media::ArtifactKind::GeneratedSpeech)
    );
    cases
        .start(fixture.project_id.clone(), prepared.run_id, true)
        .await?;
    for _ in 0..100 {
        fixture.flush().await?;
        if cases
            .state(fixture.project_id.clone(), prepared.run_id)
            .await?
            .result
            .is_some()
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let state = cases
        .state(fixture.project_id.clone(), prepared.run_id)
        .await?;
    let outbox: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT status, last_error FROM outbox_messages")
            .fetch_all(&fixture.pool)
            .await?;
    Err(format!(
        "Cancelled publication did not resume: job={:?}, outbox={outbox:?}",
        state.job
    )
    .into())
}
