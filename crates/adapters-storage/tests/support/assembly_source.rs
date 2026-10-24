use adapters_storage::{
    local::artifact_store::LocalArtifactStore,
    sqlite::{SqliteArtifactIndex, SqliteProjectRepository, SqliteSpeechRunStore, connect_sqlite},
};
use domain::{
    job::{Job, JobKind},
    media::ArtifactKind,
    project::{Project, ProjectId},
    speech::{
        Sha256Digest, SpeechId, TimingMode,
        timeline::{AudioTimelineRequest, MeasuredUtterance, SourceWindow},
    },
};
use ports::{
    audio_jobs::{AssemblySourceArtifact, AudioAssemblyIntent, AudioAssemblyPlan},
    repository::ProjectRepository,
    speech_audio::{SpeechAudioLayout, UtteranceAudioRange},
    speech_run::{SpeechCheckpoint, SpeechRunIntent, SpeechRunStore},
    speech_workflow::{
        SpeechPublication, SpeechPublishedFile, SpeechReview, SpeechSourceSnapshot,
        SpeechWorkflowStore,
    },
    storage::ArtifactStore,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tempfile::TempDir;
use uuid::Uuid;

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn pin() -> Result<Sha256Digest, &'static str> {
    Sha256Digest::parse("f".repeat(64))
}

pub struct AssemblySource {
    pub directory: TempDir,
    pub db_path: PathBuf,
    pub pool: sqlx::SqlitePool,
    pub project_id: ProjectId,
    pub speech: Arc<SqliteSpeechRunStore>,
    pub files: Arc<LocalArtifactStore>,
    pub index: Arc<SqliteArtifactIndex>,
    pub source: SpeechRunIntent,
    pub publication: SpeechPublication,
}

impl AssemblySource {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let parent =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/revoice/audio-jobs-unit");
        fs::create_dir_all(&parent)?;
        let directory = tempfile::tempdir_in(parent)?;
        let db_path = directory.path().join("app.sqlite");
        let pool = connect_sqlite(&db_path).await?;
        let project = SqliteProjectRepository::new(pool.clone())
            .create(Project::new("Synthetic assembly source".into())?)
            .await?;
        let project_id = project.id().clone();
        let speech = Arc::new(SqliteSpeechRunStore::new(pool.clone()));
        let files = Arc::new(LocalArtifactStore::new(directory.path().join("managed")));
        let index = Arc::new(SqliteArtifactIndex::new(pool.clone()));
        let text = "Model-free speech fixture";
        let text_hash = digest(text.as_bytes());
        let decision_id = Uuid::new_v4();
        speech
            .save_review(&SpeechReview {
                original_source: None,
                decision_id,
                project_id: project_id.clone(),
                source: SpeechSourceSnapshot {
                    kind: "text".into(),
                    source_id: None,
                    revision: text_hash.clone(),
                    sha256: text_hash.clone(),
                    text: text.into(),
                    cues: vec![],
                },
            })
            .await?;
        let plan_json = json!({"version":1,"review":{"decision_id":decision_id},"source":{"source_sha256":text_hash},"utterances":[{"utterance_id":"one","text":text}]}).to_string();
        let source = SpeechRunIntent {
            run_id: Uuid::new_v4(),
            project_id: project_id.clone(),
            plan_sha256: digest(plan_json.as_bytes()),
            plan_json,
            profile_id: "fixture_profile".into(),
            profile_sha256: "a".repeat(64),
            expected_utterances: 1,
            expected_chunks: 1,
        };
        speech.register_reviewed_run(&source, decision_id).await?;
        let attempt_id = Uuid::new_v4();
        let mut job = speech
            .create_pending(
                Job::new(
                    project_id.clone(),
                    "Raw speech fixture".into(),
                    JobKind::Speech,
                ),
                source.run_id,
                attempt_id,
            )
            .await?;
        let revision = job.revision();
        job.start()?;
        speech.start(&job, attempt_id, revision).await?;
        let revision = job.revision();
        job.report_speech_progress(1, 1)?;
        speech
            .record_checkpoint(
                &job,
                &SpeechCheckpoint {
                    run_id: source.run_id,
                    attempt_id,
                    plan_sha256: source.plan_sha256.clone(),
                    sequence: 1,
                    utterance_id: "one".into(),
                    cue_id: None,
                    chunk_id: "one-chunk-0001".into(),
                    cache_key: "b".repeat(64),
                    audio_sha256: "c".repeat(64),
                    duration_ms: 100,
                    origin: "cached".into(),
                },
                revision,
            )
            .await?;
        let work = directory.path().join("source-files");
        fs::create_dir(&work)?;
        let raw_path = work.join("raw.wav");
        let wave = wave();
        fs::write(&raw_path, &wave)?;
        let raw_sha = digest(&wave);
        let manifest_path = work.join("manifest.json");
        let manifest = json!({"version":4,"audio_assembly":"pcm16_stream_fade10ms_peak089125_kaiser5_v1","sample_rate":48000,"output":raw_path,"output_sha256":raw_sha,"speech_plan":{"expected_utterance_ids":["one"]},"segments":[{"utterance_id":"one","chunk_id":"one-chunk-0001","kind":"text","status":"cached","audio":"E:/missing/worker-cache.wav"}]});
        fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
        let layout = SpeechAudioLayout {
            aggregate_path: raw_path.clone(),
            aggregate_sha256: Sha256Digest::parse(raw_sha)?,
            source_plan_sha256: Sha256Digest::parse(source.plan_sha256.clone())?,
            raw_manifest_sha256: Sha256Digest::parse(digest(&fs::read(&manifest_path)?))?,
            assembly_policy: "pcm16_stream_fade10ms_peak089125_kaiser5_v1".into(),
            sample_rate: 48000,
            channels: 1,
            frame_count: 4800,
            utterances: vec![UtteranceAudioRange {
                utterance_id: SpeechId::parse("one")?,
                first_frame: 0,
                frame_count: 4800,
                chunk_ids: vec!["one-chunk-0001".into()],
                source_window: Some(SourceWindow {
                    start_ms: 100,
                    end_ms: 500,
                }),
            }],
        };
        let layout_path = work.join("layout.json");
        fs::write(&layout_path, serde_json::to_vec(&layout)?)?;
        let mut staged = Vec::new();
        let mut produced = Vec::new();
        for (role, kind, path) in [
            ("wav", ArtifactKind::GeneratedSpeech, &raw_path),
            ("manifest", ArtifactKind::SpeechManifest, &manifest_path),
            ("segment", ArtifactKind::GeneratedSpeechSegment, &raw_path),
            ("audio_layout", ArtifactKind::SpeechManifest, &layout_path),
        ] {
            let item = files
                .import_external_file(&project_id, kind, path, None)
                .await?;
            produced.push(SpeechPublishedFile {
                artifact_id: item.artifact.id.clone(),
                kind: role.into(),
                sha256: digest(&fs::read(path)?),
                size_bytes: item.size_bytes,
            });
            staged.push(item);
        }
        speech
            .publish(
                &job,
                &SpeechPublication {
                    run_id: source.run_id,
                    attempt_id,
                    duration_ms: 100,
                    files: produced,
                    ready: false,
                },
                &staged,
            )
            .await?;
        for item in staged {
            files
                .finalize_staged_artifact(&item.staging_key, &item.final_key, Some(item.size_bytes))
                .await?;
            sqlx::query("UPDATE artifacts SET state='ready',ready_at=? WHERE id=?")
                .bind(domain::chrono::Utc::now().to_rfc3339())
                .bind(item.artifact.id.to_string())
                .execute(&pool)
                .await?;
        }
        speech
            .complete_publication(source.run_id)
            .await?
            .ok_or("Raw fixture publication did not complete")?;
        let publication = speech
            .publication(source.run_id)
            .await?
            .ok_or("Raw fixture publication missing")?;
        Ok(Self {
            directory,
            db_path,
            pool,
            project_id,
            speech,
            files,
            index,
            source,
            publication,
        })
    }

    pub fn reference(
        &self,
        role: &str,
    ) -> Result<AssemblySourceArtifact, Box<dyn std::error::Error>> {
        let file = self
            .publication
            .files
            .iter()
            .find(|file| file.kind == role)
            .ok_or("Source role is missing")?;
        Ok(AssemblySourceArtifact {
            artifact_id: file.artifact_id.clone(),
            sha256: Sha256Digest::parse(file.sha256.clone())?,
            size_bytes: file.size_bytes,
        })
    }

    pub fn plan(&self) -> Result<AudioAssemblyPlan, Box<dyn std::error::Error>> {
        let raw_wav = self.reference("wav")?;
        let raw_manifest = self.reference("manifest")?;
        Ok(AudioAssemblyPlan {
            project_id: self.project_id.clone(),
            source_run_id: self.source.run_id,
            source_attempt_id: self.publication.attempt_id,
            layout: self.reference("audio_layout")?,
            ffmpeg_sha256: pin()?,
            assembly_policy: "pcm16_48k_exact_concat_v1".into(),
            timeline: AudioTimelineRequest {
                source_plan_sha256: Sha256Digest::parse(self.source.plan_sha256.clone())?,
                raw_manifest_sha256: raw_manifest.sha256.clone(),
                mode: TimingMode::Continuous,
                expected_utterance_ids: vec![SpeechId::parse("one")?],
                source_duration_ms: None,
                utterances: vec![MeasuredUtterance {
                    utterance_id: SpeechId::parse("one")?,
                    audio_sha256: raw_wav.sha256.clone(),
                    sample_rate: 48000,
                    channels: 1,
                    frame_count: 4800,
                    source_window: Some(SourceWindow {
                        start_ms: 100,
                        end_ms: 500,
                    }),
                    pause_after_ms: 0,
                }],
            },
            raw_wav,
            raw_manifest,
        })
    }

    pub fn intent(&self) -> Result<AudioAssemblyIntent, Box<dyn std::error::Error>> {
        self.with_plan(self.plan()?)
    }
    pub fn with_plan(
        &self,
        plan: AudioAssemblyPlan,
    ) -> Result<AudioAssemblyIntent, Box<dyn std::error::Error>> {
        let plan_json = serde_json::to_string(&plan)?;
        Ok(AudioAssemblyIntent {
            assembly_id: Uuid::new_v4(),
            project_id: plan.project_id,
            source_run_id: plan.source_run_id,
            source_attempt_id: plan.source_attempt_id,
            plan_sha256: Sha256Digest::parse(digest(plan_json.as_bytes()))?,
            plan_json,
        })
    }
}

pub fn wave() -> Vec<u8> {
    let size = 9600_u32;
    let mut data = b"RIFF".to_vec();
    data.extend_from_slice(&(size + 36).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16_u32.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&48000_u32.to_le_bytes());
    data.extend_from_slice(&96000_u32.to_le_bytes());
    data.extend_from_slice(&2_u16.to_le_bytes());
    data.extend_from_slice(&16_u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&size.to_le_bytes());
    for index in 0..4800_i16 {
        data.extend_from_slice(&((index % 400) - 200).to_le_bytes());
    }
    data
}
