use super::history::HistoryFixture;
use adapters_storage::sqlite::SqliteTranslationEditIntentStore;
use domain::chrono::Utc;
use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState};
use ports::storage::StagedArtifact;
use ports::translation_edit_intents::{TranslationEditIntent, TranslationEditIntentStore};
use ports::translation_links::{
    PublicationIntent, StartTranslationRun, TranslationLink, TranslationLinkStore,
};
use std::error::Error;
use uuid::Uuid;

pub struct ManualPublicationFixture {
    pub history: HistoryFixture,
    pub edit: TranslationEditIntent,
    pub publication: PublicationIntent,
}

impl ManualPublicationFixture {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let history = HistoryFixture::new().await?;
        sqlx::query("INSERT INTO translation_run_intents (run_id, translation_id, profile_manifest_json, profile_sha256, max_target_segments, context_before_segments, context_after_segments, parser_version, policy_fingerprint, created_at) VALUES (?, ?, '{}', ?, 1, 0, 0, 1, ?, ?)")
            .bind(history.intent.run_id.to_string()).bind(history.intent.translation_id.to_string())
            .bind("b".repeat(64)).bind("c".repeat(64)).bind(Utc::now().to_rfc3339())
            .execute(&history.pool).await?;
        let first = history.ready(1).await?;
        let second = history.ready(2).await?;
        let edit = TranslationEditIntent {
            project_id: history.intent.project_id.clone(),
            translation_id: history.intent.translation_id,
            result_id: Uuid::new_v4(),
            base_result_id: first.result_id,
            run_id: history.intent.run_id,
            expected_head_result_id: second.result_id,
            expected_link_revision: history.link().await?.revision,
            request_sha256: "d".repeat(64),
            created_at: Utc::now(),
        };
        SqliteTranslationEditIntentStore::new(history.pool.clone())
            .record(edit.clone())
            .await?;
        let publication = PublicationIntent {
            translation_id: edit.translation_id,
            result_id: edit.result_id,
            result_revision: 3,
            run_id: edit.run_id,
            artifact_id: ArtifactId::new(),
            source_sha256: history.intent.source_sha256.clone(),
            expected_link_revision: edit.expected_link_revision,
            needs_review: true,
            created_at: Utc::now(),
        };
        Ok(Self {
            history,
            edit,
            publication,
        })
    }

    pub fn staged(&self) -> StagedArtifact {
        let id = self.publication.artifact_id.clone();
        let final_key = format!("{}/translated/{id}.srt", self.edit.project_id);
        StagedArtifact {
            artifact: Artifact {
                id: id.clone(),
                kind: ArtifactKind::TranslatedTranscript,
                location: ArtifactLocation::StorageKey(final_key.clone()),
                size_bytes: Some(42),
                state: ArtifactState::PendingFinalize,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                ready_at: None,
            },
            staging_key: format!(".staging/{id}.srt"),
            final_key,
            size_bytes: 42,
        }
    }

    pub async fn stage(&self) -> Result<(), Box<dyn Error>> {
        self.history
            .store
            .commit_staged_publication(self.publication.clone(), self.staged())
            .await?;
        Ok(())
    }

    pub async fn mark_ready(&self) -> Result<(), Box<dyn Error>> {
        sqlx::query("UPDATE artifacts SET state = 'ready' WHERE id = ?")
            .bind(self.publication.artifact_id.to_string())
            .execute(&self.history.pool)
            .await?;
        Ok(())
    }

    pub async fn finish(&self) -> Result<TranslationLink, Box<dyn Error>> {
        Ok(self
            .history
            .store
            .finalize_ready_publication(self.edit.translation_id, self.edit.result_id, Utc::now())
            .await?)
    }

    pub async fn choose_base(&self) -> Result<TranslationLink, Box<dyn Error>> {
        Ok(self
            .history
            .store
            .select_historical_result(self.history.selection(
                self.edit.base_result_id,
                self.history.link().await?.revision,
            ))
            .await?)
    }

    pub async fn begin_other_run(&mut self) -> Result<TranslationLink, Box<dyn Error>> {
        let run_id = Uuid::new_v4();
        sqlx::query("INSERT INTO translation_run_intents (run_id, translation_id, profile_manifest_json, profile_sha256, max_target_segments, context_before_segments, context_after_segments, parser_version, policy_fingerprint, created_at) SELECT ?, translation_id, profile_manifest_json, profile_sha256, max_target_segments, context_before_segments, context_after_segments, parser_version, policy_fingerprint, created_at FROM translation_run_intents WHERE run_id = ?")
            .bind(run_id.to_string()).bind(self.edit.run_id.to_string()).execute(&self.history.pool).await?;
        let before = self.history.link().await?;
        let started = self
            .history
            .store
            .start_run(StartTranslationRun {
                translation_id: self.edit.translation_id,
                run_id,
                expected_active_run_id: before.active_run_id,
                expected_link_revision: before.revision,
                updated_at: Utc::now(),
            })
            .await?;
        self.history.intent.run_id = run_id;
        Ok(started)
    }
}
