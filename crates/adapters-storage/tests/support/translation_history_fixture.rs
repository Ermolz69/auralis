use adapters_storage::sqlite::{SqliteTranslationLinkStore, connect_sqlite};
use domain::chrono::Utc;
use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState};
use domain::project::ProjectId;
use ports::storage::StagedArtifact;
use ports::translation_links::{
    PublicationIntent, SelectTranslationHistoryResult, TranslationIntent, TranslationLink,
    TranslationLinkStore,
};
use sqlx::SqlitePool;
use std::error::Error;
use uuid::Uuid;

pub struct HistoryFixture {
    pub pool: SqlitePool,
    pub store: SqliteTranslationLinkStore,
    pub intent: TranslationIntent,
    _directory: tempfile::TempDir,
}

impl HistoryFixture {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
        let project_id = ProjectId::new();
        let source_id = ArtifactId::new();
        sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'History', 'Draft', 'now', 'now')")
            .bind(project_id.to_string()).execute(&pool).await?;
        sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', 'source.srt', 'ready', 'now', 'now')")
            .bind(source_id.to_string()).bind(project_id.to_string()).execute(&pool).await?;
        let intent = TranslationIntent {
            translation_id: Uuid::new_v4(),
            project_id,
            source_artifact_id: source_id,
            source_sha256: "a".repeat(64),
            source_format: "srt".into(),
            source_language: "zh".into(),
            target_language: "ru".into(),
            run_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };
        let store = SqliteTranslationLinkStore::new(pool.clone());
        store.begin(intent.clone()).await?;
        Ok(Self {
            pool,
            store,
            intent,
            _directory: directory,
        })
    }

    pub async fn link(&self) -> Result<TranslationLink, Box<dyn Error>> {
        self.store
            .get(self.intent.translation_id)
            .await?
            .ok_or_else(|| "missing history link".into())
    }

    pub async fn stage(
        &self,
        revision: u32,
        expected_link_revision: u64,
    ) -> Result<PublicationIntent, Box<dyn Error>> {
        let artifact_id = ArtifactId::new();
        let final_key = format!("{}/translated/{artifact_id}.srt", self.intent.project_id);
        let publication = PublicationIntent {
            translation_id: self.intent.translation_id,
            result_id: Uuid::new_v4(),
            result_revision: revision,
            run_id: self.intent.run_id,
            artifact_id: artifact_id.clone(),
            source_sha256: self.intent.source_sha256.clone(),
            expected_link_revision,
            needs_review: true,
            created_at: Utc::now(),
        };
        self.store
            .commit_staged_publication(
                publication.clone(),
                StagedArtifact {
                    artifact: Artifact {
                        id: artifact_id.clone(),
                        kind: ArtifactKind::TranslatedTranscript,
                        location: ArtifactLocation::StorageKey(final_key.clone()),
                        size_bytes: Some(42),
                        state: ArtifactState::PendingFinalize,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        ready_at: None,
                    },
                    staging_key: format!(".staging/{artifact_id}.srt"),
                    final_key,
                    size_bytes: 42,
                },
            )
            .await?;
        Ok(publication)
    }

    pub async fn ready(&self, revision: u32) -> Result<PublicationIntent, Box<dyn Error>> {
        let publication = self.stage(revision, self.link().await?.revision).await?;
        sqlx::query("UPDATE artifacts SET state = 'ready' WHERE id = ?")
            .bind(publication.artifact_id.to_string())
            .execute(&self.pool)
            .await?;
        self.store
            .finalize_ready_publication(
                self.intent.translation_id,
                publication.result_id,
                Utc::now(),
            )
            .await?;
        Ok(publication)
    }

    pub fn selection(&self, result_id: Uuid, revision: u64) -> SelectTranslationHistoryResult {
        SelectTranslationHistoryResult {
            project_id: self.intent.project_id.clone(),
            translation_id: self.intent.translation_id,
            result_id,
            expected_link_revision: revision,
            updated_at: Utc::now(),
        }
    }
}
