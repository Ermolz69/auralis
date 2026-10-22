use super::*;
use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::{
    speech_workflow::{SpeechPublication, SpeechReview, SpeechWorkflowStore},
    storage::StagedArtifact,
};

#[async_trait]
impl SpeechWorkflowStore for SqliteSpeechRunStore {
    async fn register_reviewed_run(
        &self,
        intent: &SpeechRunIntent,
        decision_id: Uuid,
    ) -> Result<(), PortError> {
        validate_intent(intent)?;
        let plan: serde_json::Value =
            serde_json::from_str(&intent.plan_json).map_err(|e| invalid_input(&e.to_string()))?;
        let review = self
            .get_review(decision_id)
            .await?
            .ok_or_else(|| invalid_input("Speech review is missing"))?;
        if review.project_id != intent.project_id
            || plan["review"]["decision_id"].as_str() != Some(decision_id.to_string().as_str())
            || plan["source"]["source_sha256"].as_str() != Some(review.source.sha256.as_str())
        {
            return Err(invalid_input("Reviewed speech intent identity differs"));
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(decode_error)?;
        sqlx::query("INSERT INTO speech_run_intents VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(intent.run_id.to_string())
            .bind(intent.project_id.to_string())
            .bind(&intent.plan_json)
            .bind(&intent.plan_sha256)
            .bind(&intent.profile_id)
            .bind(&intent.profile_sha256)
            .bind(intent.expected_utterances)
            .bind(intent.expected_chunks)
            .bind(Utc::now().to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| conflict_or_storage("register_reviewed_speech", e))?;
        sqlx::query("INSERT INTO speech_run_reviews VALUES (?, ?)")
            .bind(intent.run_id.to_string())
            .bind(decision_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(decode_error)?;
        tx.commit().await.map_err(decode_error)
    }
    async fn save_review(&self, review: &SpeechReview) -> Result<(), PortError> {
        if review.source.text.trim().is_empty() || !valid_hash(&review.source.sha256) {
            return Err(invalid_input("Reviewed text is empty or has no identity"));
        }
        let json = serde_json::to_string(review).map_err(|e| invalid_input(&e.to_string()))?;
        sqlx::query("INSERT INTO speech_text_reviews VALUES (?, ?, ?, ?)")
            .bind(review.decision_id.to_string())
            .bind(review.project_id.to_string())
            .bind(json)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| conflict_or_storage("save_speech_review", e))?;
        Ok(())
    }

    async fn get_review(&self, decision_id: Uuid) -> Result<Option<SpeechReview>, PortError> {
        let json: Option<String> = sqlx::query_scalar(
            "SELECT snapshot_json FROM speech_text_reviews WHERE decision_id = ?",
        )
        .bind(decision_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(decode_error)?;
        json.map(|s| {
            serde_json::from_str(&s)
                .map_err(|e| invalid_stored(decision_id, "review", e.to_string()))
        })
        .transpose()
    }

    async fn list_run_ids(&self, project: &ProjectId) -> Result<Vec<Uuid>, PortError> {
        let ids: Vec<String> = sqlx::query_scalar("SELECT run_id FROM speech_run_intents WHERE project_id = ? ORDER BY created_at DESC LIMIT 100")
            .bind(project.to_string()).fetch_all(&self.pool).await.map_err(decode_error)?;
        ids.into_iter()
            .map(|id| Uuid::parse_str(&id).map_err(|e| invalid_input(&e.to_string())))
            .collect()
    }

    async fn latest_job(&self, run_id: Uuid) -> Result<Option<Job>, PortError> {
        let row = sqlx::query_as::<_, JobRow>("SELECT job.* FROM jobs job JOIN speech_host_attempts a ON a.job_id = job.id WHERE a.run_id = ? ORDER BY a.created_at DESC, a.rowid DESC LIMIT 1")
            .bind(run_id.to_string()).fetch_optional(&self.pool).await.map_err(decode_error)?;
        row.map(row_to_job).transpose()
    }

    async fn publication(&self, run_id: Uuid) -> Result<Option<SpeechPublication>, PortError> {
        let row: Option<(String, bool)> = sqlx::query_as(
            "SELECT publication_json, ready FROM speech_publications WHERE run_id = ?",
        )
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(decode_error)?;
        row.map(|(json, ready)| {
            let mut result: SpeechPublication = serde_json::from_str(&json)
                .map_err(|e| invalid_stored(run_id, "publication", e.to_string()))?;
            result.ready = ready;
            Ok(result)
        })
        .transpose()
    }

    async fn publish(
        &self,
        job: &Job,
        publication: &SpeechPublication,
        staged: &[StagedArtifact],
    ) -> Result<(), PortError> {
        if job.kind() != &JobKind::Speech
            || job.status() != &JobStatus::Running
            || staged.len() != publication.files.len()
            || staged.is_empty()
            || publication.ready
            || publication.duration_ms == 0
        {
            return Err(invalid_input("Invalid speech publication"));
        }
        if publication.files.iter().filter(|f| f.kind == "wav").count() != 1
            || publication
                .files
                .iter()
                .filter(|f| f.kind == "manifest")
                .count()
                != 1
            || publication
                .files
                .iter()
                .any(|f| !matches!(f.kind.as_str(), "wav" | "manifest" | "segment"))
        {
            return Err(invalid_input(
                "Speech publication requires one WAV, one manifest and its segments",
            ));
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(decode_error)?;
        let eligible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM speech_host_attempts a JOIN jobs j ON j.id=a.job_id JOIN speech_run_intents i ON i.run_id=a.run_id WHERE a.attempt_id=? AND a.run_id=? AND a.job_id=? AND a.ended_at IS NULL AND j.status='running' AND j.revision=? AND j.project_id=? AND (SELECT COUNT(*) FROM speech_host_checkpoints WHERE attempt_id=a.attempt_id)=i.expected_chunks)")
            .bind(publication.attempt_id.to_string()).bind(publication.run_id.to_string()).bind(job.id().to_string())
            .bind(revision(job.revision())?).bind(job.project_id().to_string()).fetch_one(&mut *tx).await.map_err(decode_error)?;
        if !eligible {
            return Err(conflict("Stale or incomplete speech publication"));
        }
        let chunks: i64 =
            sqlx::query_scalar("SELECT expected_chunks FROM speech_run_intents WHERE run_id=?")
                .bind(publication.run_id.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(decode_error)?;
        if publication
            .files
            .iter()
            .filter(|f| f.kind == "segment")
            .count() as i64
            != chunks
        {
            return Err(invalid_input(
                "Published audio segment coverage differs from its plan",
            ));
        }
        for (artifact, file) in staged.iter().zip(&publication.files) {
            let expected_kind = match file.kind.as_str() {
                "wav" => domain::media::ArtifactKind::GeneratedSpeech,
                "manifest" => domain::media::ArtifactKind::SpeechManifest,
                _ => domain::media::ArtifactKind::GeneratedSpeechSegment,
            };
            if artifact.artifact.id != file.artifact_id
                || artifact.size_bytes != file.size_bytes
                || !valid_hash(&file.sha256)
                || artifact.artifact.state != domain::media::ArtifactState::PendingFinalize
                || artifact.artifact.kind != expected_kind
                || artifact.artifact.location
                    != domain::media::ArtifactLocation::StorageKey(artifact.final_key.clone())
                || !artifact
                    .final_key
                    .starts_with(&format!("{}/", job.project_id()))
            {
                return Err(invalid_input("Publication and staged artifact differ"));
            }
            crate::sqlite::unit_of_work::insert_new_artifact(
                &mut tx,
                job.project_id(),
                &artifact.artifact,
            )
            .await?;
            let message = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
                project_id: job.project_id().clone(),
                artifact_id: file.artifact_id.clone(),
                staging_key: artifact.staging_key.clone(),
                final_key: artifact.final_key.clone(),
            });
            crate::sqlite::unit_of_work::outbox_writes::save_outbox_message(&mut tx, &message)
                .await?;
        }
        let json = serde_json::to_string(publication).map_err(|e| invalid_input(&e.to_string()))?;
        sqlx::query("INSERT INTO speech_publications VALUES (?, ?, ?, 0, ?)")
            .bind(publication.run_id.to_string())
            .bind(publication.attempt_id.to_string())
            .bind(json)
            .bind(Utc::now().to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| conflict_or_storage("publish_speech", e))?;
        tx.commit().await.map_err(decode_error)
    }

    async fn complete_publication(&self, run_id: Uuid) -> Result<Option<Job>, PortError> {
        let Some(publication) = self.publication(run_id).await? else {
            return Ok(None);
        };
        if publication.ready {
            return Ok(None);
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(decode_error)?;
        let row = sqlx::query_as::<_, JobRow>("SELECT j.* FROM jobs j JOIN speech_host_attempts a ON a.job_id=j.id WHERE a.attempt_id=? AND a.run_id=? AND a.ended_at IS NULL")
            .bind(publication.attempt_id.to_string()).bind(run_id.to_string()).fetch_optional(&mut *tx).await.map_err(decode_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut job = row_to_job(row)?;
        if job.status() != &JobStatus::Running {
            return Ok(None);
        }
        for file in &publication.files {
            let ready: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id=? AND project_id=? AND state='ready' AND size_bytes=?)")
                .bind(file.artifact_id.to_string()).bind(job.project_id().to_string()).bind(revision_value(file.size_bytes)?)
                .fetch_one(&mut *tx).await.map_err(decode_error)?;
            if !ready {
                return Ok(None);
            }
        }
        let previous = job.revision();
        job.mark_completed()
            .map_err(|e| invalid_input(&e.to_string()))?;
        let values = job_to_row_values(&job)?;
        let updated = sqlx::query("UPDATE jobs SET status=?,revision=?,progress_json=?,updated_at=?,finished_at=? WHERE id=? AND revision=? AND status='running'")
            .bind(values.status).bind(values.revision).bind(values.progress_json).bind(values.updated_at).bind(&values.finished_at)
            .bind(values.id).bind(revision(previous)?).execute(&mut *tx).await.map_err(decode_error)?;
        if updated.rows_affected() != 1 {
            return Err(conflict("Speech completion lost its attempt"));
        }
        sqlx::query(
            "UPDATE speech_host_attempts SET ended_at=? WHERE attempt_id=? AND ended_at IS NULL",
        )
        .bind(values.finished_at)
        .bind(publication.attempt_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(decode_error)?;
        sqlx::query("UPDATE speech_publications SET ready=1 WHERE run_id=?")
            .bind(run_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(decode_error)?;
        tx.commit().await.map_err(decode_error)?;
        Ok(Some(job))
    }

    async fn close_interrupted(&self) -> Result<(), PortError> {
        sqlx::query("UPDATE speech_host_attempts SET ended_at=? WHERE ended_at IS NULL AND EXISTS(SELECT 1 FROM jobs WHERE jobs.id=speech_host_attempts.job_id AND status IN ('failed','cancelled','completed'))")
            .bind(Utc::now().to_rfc3339()).execute(&self.pool).await.map_err(decode_error)?;
        Ok(())
    }

    async fn discard_unready_publication(
        &self,
        run_id: Uuid,
        attempt_id: Uuid,
    ) -> Result<bool, PortError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(decode_error)?;
        let json: Option<String> = sqlx::query_scalar("SELECT publication_json FROM speech_publications WHERE run_id=? AND attempt_id=? AND ready=0")
            .bind(run_id.to_string()).bind(attempt_id.to_string()).fetch_optional(&mut *tx).await.map_err(decode_error)?;
        let Some(json) = json else {
            return Ok(false);
        };
        let publication: SpeechPublication = serde_json::from_str(&json)
            .map_err(|e| invalid_stored(run_id, "publication", e.to_string()))?;
        for file in publication.files {
            let payload: Option<String> = sqlx::query_scalar("SELECT payload_json FROM outbox_messages WHERE json_extract(payload_json,'$.type')='finalize_staged_artifact' AND json_extract(payload_json,'$.artifact_id')=? LIMIT 1")
                .bind(file.artifact_id.to_string()).fetch_optional(&mut *tx).await.map_err(decode_error)?;
            if let Some(payload) = payload {
                let value: OutboxPayload =
                    serde_json::from_str(&payload).map_err(|e| invalid_input(&e.to_string()))?;
                if let OutboxPayload::FinalizeStagedArtifact {
                    staging_key,
                    final_key,
                    ..
                } = value
                {
                    sqlx::query("INSERT INTO speech_discarded_artifacts VALUES (?,?,?,?,?)")
                        .bind(file.artifact_id.to_string())
                        .bind(run_id.to_string())
                        .bind(attempt_id.to_string())
                        .bind(&final_key)
                        .bind(Utc::now().to_rfc3339())
                        .execute(&mut *tx)
                        .await
                        .map_err(decode_error)?;
                    for storage_key in [staging_key, final_key] {
                        let cleanup =
                            OutboxMessage::new(OutboxPayload::DeleteStorageKey { storage_key });
                        crate::sqlite::unit_of_work::outbox_writes::save_outbox_message(
                            &mut tx, &cleanup,
                        )
                        .await?;
                    }
                }
            }
            sqlx::query("DELETE FROM artifacts WHERE id=?")
                .bind(file.artifact_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(decode_error)?;
        }
        sqlx::query("DELETE FROM speech_publications WHERE run_id=? AND attempt_id=? AND ready=0")
            .bind(run_id.to_string())
            .bind(attempt_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(decode_error)?;
        tx.commit().await.map_err(decode_error)?;
        Ok(true)
    }
}
