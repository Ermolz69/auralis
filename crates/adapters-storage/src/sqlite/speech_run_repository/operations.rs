use super::*;

#[async_trait]
impl SpeechRunStore for SqliteSpeechRunStore {
    async fn register_run(&self, intent: &SpeechRunIntent) -> Result<(), PortError> {
        validate_intent(intent)?;
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO speech_run_intents (
                run_id, project_id, plan_json, plan_sha256, profile_id, profile_sha256,
                expected_utterances, expected_chunks, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(intent.run_id.to_string())
        .bind(intent.project_id.to_string())
        .bind(&intent.plan_json)
        .bind(&intent.plan_sha256)
        .bind(&intent.profile_id)
        .bind(&intent.profile_sha256)
        .bind(intent.expected_utterances)
        .bind(intent.expected_chunks)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlite_error("register_speech_run", error))?;
        if inserted.rows_affected() == 1
            || self.get_run(intent.run_id).await?.as_ref() == Some(intent)
        {
            Ok(())
        } else {
            Err(conflict("run ID already names another speech intent"))
        }
    }

    async fn get_run(&self, run_id: Uuid) -> Result<Option<SpeechRunIntent>, PortError> {
        let row = sqlx::query(
            "SELECT run_id, project_id, plan_json, plan_sha256, profile_id,
                    profile_sha256, expected_utterances, expected_chunks
               FROM speech_run_intents WHERE run_id = ?",
        )
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlite_error("get_speech_run", error))?;
        row.map(|row| {
            let project: String = row.try_get("project_id").map_err(decode_error)?;
            let project_id = ProjectId::from_str(&project)
                .map_err(|error| invalid_stored(run_id, "project_id", error.to_string()))?;
            let expected_utterances: i64 =
                row.try_get("expected_utterances").map_err(decode_error)?;
            let expected_chunks: i64 = row.try_get("expected_chunks").map_err(decode_error)?;
            let intent = SpeechRunIntent {
                run_id,
                project_id,
                plan_json: row.try_get("plan_json").map_err(decode_error)?,
                plan_sha256: row.try_get("plan_sha256").map_err(decode_error)?,
                profile_id: row.try_get("profile_id").map_err(decode_error)?,
                profile_sha256: row.try_get("profile_sha256").map_err(decode_error)?,
                expected_utterances: u32::try_from(expected_utterances).map_err(|error| {
                    invalid_stored(run_id, "expected_utterances", error.to_string())
                })?,
                expected_chunks: u32::try_from(expected_chunks).map_err(|error| {
                    invalid_stored(run_id, "expected_chunks", error.to_string())
                })?,
            };
            validate_intent(&intent)
                .map_err(|error| invalid_stored(run_id, "intent", error.to_string()))?;
            Ok(intent)
        })
        .transpose()
    }

    async fn create_pending(
        &self,
        job: Job,
        run_id: Uuid,
        attempt_id: Uuid,
    ) -> Result<Job, PortError> {
        if job.kind() != &JobKind::Speech
            || job.status() != &JobStatus::Pending
            || job.stage().is_some()
            || job.revision() != 1
            || job.started_at().is_some()
            || job.finished_at().is_some()
        {
            return Err(invalid_input(
                "speech host job must be a new pending speech job",
            ));
        }
        let values = job_to_row_values(&job)?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| map_sqlite_error("begin_speech_attempt", error))?;
        let inserted = sqlx::query(
            "INSERT INTO jobs (
                id, revision, project_id, title, kind, status, stage, progress_json,
                error_json, created_at, updated_at, started_at, finished_at
            ) SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
              FROM speech_run_intents AS intent
             WHERE intent.run_id = ? AND intent.project_id = ?",
        )
        .bind(&values.id)
        .bind(values.revision)
        .bind(&values.project_id)
        .bind(&values.title)
        .bind(&values.kind)
        .bind(&values.status)
        .bind(&values.stage)
        .bind(&values.progress_json)
        .bind(&values.error_json)
        .bind(&values.created_at)
        .bind(&values.updated_at)
        .bind(&values.started_at)
        .bind(&values.finished_at)
        .bind(run_id.to_string())
        .bind(&values.project_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| conflict_or_storage("create_speech_host_job", error))?;
        if inserted.rows_affected() != 1 {
            return Err(conflict("speech run does not belong to the job project"));
        }
        sqlx::query(
            "INSERT INTO speech_host_attempts (attempt_id, run_id, job_id, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(attempt_id.to_string())
        .bind(run_id.to_string())
        .bind(&values.id)
        .bind(&values.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|error| conflict_or_storage("associate_speech_attempt", error))?;
        tx.commit()
            .await
            .map_err(|error| map_sqlite_error("commit_speech_attempt", error))?;
        Ok(job)
    }

    async fn active_for_run(&self, run_id: Uuid) -> Result<Option<ActiveSpeechAttempt>, PortError> {
        let row = sqlx::query(
            "SELECT job.*, attempt.run_id AS host_run_id, attempt.attempt_id AS host_attempt_id
               FROM speech_host_attempts AS attempt
               JOIN jobs AS job ON job.id = attempt.job_id
              WHERE attempt.run_id = ? AND attempt.ended_at IS NULL",
        )
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlite_error("get_active_speech_attempt", error))?;
        row.map(decode_active).transpose()
    }

    async fn list_active(&self) -> Result<Vec<ActiveSpeechAttempt>, PortError> {
        let rows = sqlx::query(
            "SELECT job.*, attempt.run_id AS host_run_id, attempt.attempt_id AS host_attempt_id
               FROM speech_host_attempts AS attempt
               JOIN jobs AS job ON job.id = attempt.job_id
              WHERE attempt.ended_at IS NULL ORDER BY attempt.created_at, attempt.attempt_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlite_error("list_active_speech_attempts", error))?;
        rows.into_iter().map(decode_active).collect()
    }

    async fn start(
        &self,
        job: &Job,
        attempt_id: Uuid,
        expected_revision: u64,
    ) -> Result<(), PortError> {
        if job.kind() != &JobKind::Speech
            || job.status() != &JobStatus::Running
            || job.stage().is_some()
            || job.started_at().is_none()
            || job.finished_at().is_some()
            || job.revision() != expected_revision.saturating_add(1)
        {
            return Err(invalid_input(
                "speech host job must have a new running revision",
            ));
        }
        let values = job_to_row_values(job)?;
        let updated = sqlx::query(
            "UPDATE jobs SET revision = ?, status = ?, progress_json = ?, updated_at = ?, started_at = ?
              WHERE id = ? AND revision = ? AND project_id = ? AND kind = 'speech'
                AND status = 'pending'
                AND EXISTS (SELECT 1 FROM speech_host_attempts AS attempt
                             WHERE attempt.job_id = jobs.id AND attempt.attempt_id = ?
                               AND attempt.ended_at IS NULL)",
        )
        .bind(values.revision).bind(&values.status).bind(&values.progress_json)
        .bind(&values.updated_at).bind(&values.started_at)
        .bind(&values.id).bind(revision(expected_revision)?)
        .bind(&values.project_id).bind(attempt_id.to_string())
        .execute(&self.pool).await
        .map_err(|error| map_sqlite_error("start_speech_attempt", error))?;
        if updated.rows_affected() == 1 {
            Ok(())
        } else {
            Err(conflict("speech attempt or pending job revision changed"))
        }
    }

    async fn record_checkpoint(
        &self,
        job: &Job,
        checkpoint: &SpeechCheckpoint,
        expected_revision: u64,
    ) -> Result<(), PortError> {
        if job.kind() != &JobKind::Speech
            || job.status() != &JobStatus::Running
            || job.stage().is_some()
            || job.revision() != expected_revision.saturating_add(1)
            || job.progress().processed_items != Some(u64::from(checkpoint.sequence))
            || job.progress().total_items.is_none()
            || checkpoint.sequence == 0
            || !valid_hash(&checkpoint.plan_sha256)
            || !valid_hash(&checkpoint.cache_key)
            || !valid_hash(&checkpoint.audio_sha256)
            || !portable_id(&checkpoint.utterance_id)
            || !portable_id(&checkpoint.chunk_id)
            || checkpoint.duration_ms == 0
            || !matches!(
                checkpoint.origin.as_str(),
                "generated" | "cached" | "reused_in_job"
            )
        {
            return Err(invalid_input(
                "speech checkpoint does not match running job progress",
            ));
        }
        let values = job_to_row_values(job)?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| map_sqlite_error("begin_speech_checkpoint", error))?;
        let state: Option<(String, String, i64, i64, String, i64)> = sqlx::query_as(
            "SELECT attempt.run_id, intent.plan_sha256, intent.expected_chunks,
                    job.revision, job.status,
                    COALESCE((SELECT MAX(sequence) FROM speech_host_checkpoints
                               WHERE attempt_id = attempt.attempt_id), 0)
               FROM speech_host_attempts AS attempt
               JOIN speech_run_intents AS intent ON intent.run_id = attempt.run_id
               JOIN jobs AS job ON job.id = attempt.job_id
              WHERE attempt.attempt_id = ? AND attempt.ended_at IS NULL AND job.id = ?",
        )
        .bind(checkpoint.attempt_id.to_string())
        .bind(&values.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| map_sqlite_error("inspect_speech_checkpoint", error))?;
        let expected_sequence = i64::from(checkpoint.sequence);
        let total_chunks = job
            .progress()
            .total_items
            .and_then(|count| i64::try_from(count).ok())
            .ok_or_else(|| invalid_input("invalid total speech chunks"))?;
        if !matches!(state, Some((ref run, ref digest, chunks, revision, ref status, previous))
            if run == &checkpoint.run_id.to_string()
                && digest == &checkpoint.plan_sha256
                && chunks == total_chunks
                && expected_sequence <= chunks
                && revision == revision_value(expected_revision)?
                && status == "running"
                && previous + 1 == expected_sequence)
        {
            return Err(conflict(
                "stale speech attempt, plan, revision or checkpoint sequence",
            ));
        }
        sqlx::query(
            "INSERT INTO speech_host_checkpoints (
                attempt_id, sequence, utterance_id, cue_id, chunk_id, cache_key,
                audio_sha256, duration_ms, origin, saved_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(checkpoint.attempt_id.to_string())
        .bind(expected_sequence)
        .bind(&checkpoint.utterance_id)
        .bind(&checkpoint.cue_id)
        .bind(&checkpoint.chunk_id)
        .bind(&checkpoint.cache_key)
        .bind(&checkpoint.audio_sha256)
        .bind(revision_value(checkpoint.duration_ms)?)
        .bind(&checkpoint.origin)
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await
        .map_err(|error| conflict_or_storage("insert_speech_checkpoint", error))?;
        let updated = sqlx::query(
            "UPDATE jobs SET revision = ?, progress_json = ?, updated_at = ?
              WHERE id = ? AND revision = ? AND status = 'running' AND kind = 'speech'",
        )
        .bind(values.revision)
        .bind(&values.progress_json)
        .bind(&values.updated_at)
        .bind(&values.id)
        .bind(revision(expected_revision)?)
        .execute(&mut *tx)
        .await
        .map_err(|error| map_sqlite_error("update_speech_checkpoint_progress", error))?;
        if updated.rows_affected() != 1 {
            return Err(conflict(
                "speech job revision changed while saving checkpoint",
            ));
        }
        tx.commit()
            .await
            .map_err(|error| map_sqlite_error("commit_speech_checkpoint", error))
    }

    async fn finish(
        &self,
        job: &Job,
        attempt_id: Uuid,
        expected_revision: u64,
    ) -> Result<(), PortError> {
        if job.kind() != &JobKind::Speech
            || !matches!(job.status(), JobStatus::Cancelled | JobStatus::Failed)
            || job.stage().is_some()
            || job.finished_at().is_none()
            || job.revision() != expected_revision.saturating_add(1)
        {
            return Err(invalid_input(
                "speech attempt can only stop as cancelled or failed before publication",
            ));
        }
        let values = job_to_row_values(job)?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| map_sqlite_error("begin_finish_speech_attempt", error))?;
        let updated = sqlx::query(
            "UPDATE jobs SET revision = ?, status = ?, progress_json = ?, error_json = ?,
                    updated_at = ?, finished_at = ?
              WHERE id = ? AND revision = ? AND project_id = ? AND kind = 'speech'
                AND status IN ('pending', 'running', 'cancelling')
                AND EXISTS (SELECT 1 FROM speech_host_attempts AS attempt
                             WHERE attempt.job_id = jobs.id AND attempt.attempt_id = ?
                               AND attempt.ended_at IS NULL)",
        )
        .bind(values.revision)
        .bind(&values.status)
        .bind(&values.progress_json)
        .bind(&values.error_json)
        .bind(&values.updated_at)
        .bind(&values.finished_at)
        .bind(&values.id)
        .bind(revision(expected_revision)?)
        .bind(&values.project_id)
        .bind(attempt_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|error| map_sqlite_error("finish_speech_attempt", error))?;
        if updated.rows_affected() != 1 {
            return Err(conflict(
                "speech attempt or job revision changed before finish",
            ));
        }
        let ended = sqlx::query(
            "UPDATE speech_host_attempts SET ended_at = ?
              WHERE attempt_id = ? AND job_id = ? AND ended_at IS NULL",
        )
        .bind(&values.finished_at)
        .bind(attempt_id.to_string())
        .bind(&values.id)
        .execute(&mut *tx)
        .await
        .map_err(|error| map_sqlite_error("end_speech_attempt", error))?;
        if ended.rows_affected() != 1 {
            return Err(conflict("speech attempt ended before job"));
        }
        tx.commit()
            .await
            .map_err(|error| map_sqlite_error("commit_finish_speech_attempt", error))
    }
}
