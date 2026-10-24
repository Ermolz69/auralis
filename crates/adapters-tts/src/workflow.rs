use crate::{
    SavedCheckpoint, TtsExecutionObserver, TtsExecutionOutcome, TtsExecutionRequest,
    TtsWorkerClient,
};
use async_trait::async_trait;
use domain::project::ProjectId;
use ports::{
    cancellation::CancellationToken,
    error::PortError,
    speech_run::{MAX_SPEECH_PLAN_BYTES, SpeechCheckpoint, SpeechRunIntent},
    speech_workflow::{
        SpeechExecutionPort, SpeechExecutionResult, SpeechProducedFile, SpeechProgressObserver,
    },
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

impl TtsWorkerClient {
    fn run_root(&self, intent: &SpeechRunIntent) -> PathBuf {
        self.runtime()
            .managed_root
            .join(intent.project_id.to_string())
            .join("speech")
            .join(intent.run_id.to_string())
    }

    fn cache_root(&self, intent: &SpeechRunIntent) -> PathBuf {
        self.runtime()
            .managed_root
            .join(intent.project_id.to_string())
            .join("speech")
            .join("cache")
    }

    async fn saved_plan(&self, intent: &SpeechRunIntent) -> Result<PathBuf, PortError> {
        let anchor = tokio::fs::read_to_string(self.run_root(intent).join("runtime-sha256.txt"))
            .await
            .map_err(io_error)?;
        if anchor != self.runtime().integrity_sha256 {
            return Err(invalid("TTS runtime changed; prepare a new run"));
        }
        let path = self.run_root(intent).join("plan.json");
        let bytes = read_bounded_json(&path).await?;
        if format!("{:x}", Sha256::digest(&bytes)) != intent.plan_sha256
            || bytes != intent.plan_json.as_bytes()
        {
            return Err(invalid("Saved speech plan changed; prepare a new run"));
        }
        Ok(path)
    }
}

#[async_trait]
impl SpeechExecutionPort for TtsWorkerClient {
    async fn compose(
        &self,
        project: ProjectId,
        run_id: Uuid,
        profile_id: String,
        draft: String,
    ) -> Result<SpeechRunIntent, PortError> {
        if draft.len() > MAX_SPEECH_PLAN_BYTES {
            return Err(invalid("Speech draft exceeds the worker plan limit"));
        }
        let root = self
            .runtime()
            .managed_root
            .join(project.to_string())
            .join("speech")
            .join(run_id.to_string());
        tokio::fs::create_dir_all(&root).await.map_err(io_error)?;
        self.verify_profile_integrity(
            &self
                .runtime()
                .profile_root
                .join(format!("{profile_id}.json")),
        )
        .await?;
        tokio::fs::write(
            root.join("runtime-sha256.txt"),
            &self.runtime().integrity_sha256,
        )
        .await
        .map_err(io_error)?;
        let draft_path = root.join("draft.json");
        let plan_path = root.join("plan.json");
        tokio::fs::write(&draft_path, draft)
            .await
            .map_err(io_error)?;
        let composed = TtsWorkerClient::compose(
            self,
            &project,
            &run_id.to_string(),
            "prepare",
            &draft_path,
            &plan_path,
            &profile_id,
        )
        .await?;
        let plan_json = String::from_utf8(read_bounded_json(&plan_path).await?)
            .map_err(|_| invalid("Composed speech plan is not UTF-8"))?;
        let plan: Value = serde_json::from_str(&plan_json).map_err(|e| invalid(&e.to_string()))?;
        let profile_sha256 = plan["profile"]["profile_sha256"]
            .as_str()
            .ok_or_else(|| invalid("Composed plan has no profile hash"))?
            .to_string();
        Ok(SpeechRunIntent {
            run_id,
            project_id: project,
            plan_sha256: format!("{:x}", Sha256::digest(plan_json.as_bytes())),
            plan_json,
            profile_id,
            profile_sha256,
            expected_utterances: u32::try_from(composed.utterance_count)
                .map_err(|_| invalid("Too many utterances"))?,
            expected_chunks: u32::try_from(composed.chunk_count)
                .map_err(|_| invalid("Too many chunks"))?,
        })
    }

    async fn validate(&self, intent: &SpeechRunIntent) -> Result<(), PortError> {
        self.verify_profile_integrity(
            &self
                .runtime()
                .profile_root
                .join(format!("{}.json", intent.profile_id)),
        )
        .await?;
        let plan_path = self.saved_plan(intent).await?;
        let validated = self
            .validate_plan(
                &intent.project_id,
                &intent.run_id.to_string(),
                "validate",
                &plan_path,
                &intent.profile_id,
            )
            .await?;
        let plan: Value =
            serde_json::from_str(&intent.plan_json).map_err(|e| invalid(&e.to_string()))?;
        if validated.utterance_count != intent.expected_utterances as usize
            || validated.chunk_count != intent.expected_chunks as usize
            || Some(validated.plan_digest.as_str()) != plan["plan_digest"].as_str()
        {
            return Err(invalid("Speech plan coverage or identity changed"));
        }
        Ok(())
    }

    async fn execute(
        &self,
        intent: SpeechRunIntent,
        attempt_id: Uuid,
        observer: Arc<dyn SpeechProgressObserver>,
        cancel: CancellationToken,
    ) -> Result<SpeechExecutionResult, PortError> {
        self.saved_plan(&intent).await?;
        let plan: Value =
            serde_json::from_str(&intent.plan_json).map_err(|e| invalid(&e.to_string()))?;
        let root = self.run_root(&intent);
        let attempt = root.join("attempts").join(attempt_id.to_string());
        tokio::fs::create_dir_all(&attempt)
            .await
            .map_err(io_error)?;
        let request = TtsExecutionRequest {
            project_id: intent.project_id.clone(),
            run_id: intent.run_id.to_string(),
            attempt_id: attempt_id.to_string(),
            plan_digest: plan["plan_digest"]
                .as_str()
                .ok_or_else(|| invalid("Missing plan digest"))?
                .into(),
            expected_utterances: intent.expected_utterances as usize,
            expected_chunks: intent.expected_chunks as usize,
            plan_path: root.join("plan.json"),
            profile_path: self
                .runtime()
                .profile_root
                .join(format!("{}.json", intent.profile_id)),
            out_file: attempt.join("speech.wav"),
            manifest_path: attempt.join("manifest.json"),
            // Keep existing resumable runs on their original cache. New runs share
            // verified project audio so a changed cue regenerates only its chunks.
            cache_dir: if root.join("cache").exists() {
                root.join("cache")
            } else {
                self.cache_root(&intent)
            },
            checkpoint_root: root.join("checkpoints"),
            rebuild: false,
        };
        let bridge = Observer {
            observer,
            run_id: intent.run_id,
            attempt_id,
            plan_sha256: intent.plan_sha256,
        };
        match TtsWorkerClient::execute(self, request.clone(), &bridge, cancel.clone()).await? {
            TtsExecutionOutcome::Cancelled => Ok(SpeechExecutionResult::Cancelled),
            TtsExecutionOutcome::Completed(result) => {
                let bytes = read_bounded_json(&result.manifest).await?;
                if format!("{:x}", Sha256::digest(&bytes)) != result.manifest_sha256 {
                    return Err(invalid("Speech result manifest changed after verification"));
                }
                let manifest: Value =
                    serde_json::from_slice(&bytes).map_err(|e| invalid(&e.to_string()))?;
                let layout =
                    crate::output_layout::produce(request, result.clone(), &manifest, cancel)
                        .await?;
                let mut files = vec![
                    SpeechProducedFile {
                        kind: "wav".into(),
                        path: result.output,
                        sha256: result.output_sha256,
                    },
                    SpeechProducedFile {
                        kind: "manifest".into(),
                        path: result.manifest,
                        sha256: result.manifest_sha256,
                    },
                ];
                if let Some(file) = layout {
                    files.push(file);
                }
                for segment in manifest["segments"]
                    .as_array()
                    .ok_or_else(|| invalid("Missing result segments"))?
                {
                    files.push(SpeechProducedFile {
                        kind: "segment".into(),
                        path: PathBuf::from(
                            segment["audio"]
                                .as_str()
                                .ok_or_else(|| invalid("Missing segment audio"))?,
                        ),
                        sha256: segment["audio_sha256"]
                            .as_str()
                            .ok_or_else(|| invalid("Missing segment hash"))?
                            .into(),
                    });
                }
                Ok(SpeechExecutionResult::Completed {
                    files,
                    duration_ms: result.duration_ms,
                })
            }
        }
    }
}

async fn read_bounded_json(path: &std::path::Path) -> Result<Vec<u8>, PortError> {
    let metadata = tokio::fs::symlink_metadata(path).await.map_err(io_error)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_SPEECH_PLAN_BYTES as u64
    {
        return Err(invalid(
            "Speech plan or manifest is not a bounded regular file",
        ));
    }
    let mut bytes = Vec::new();
    tokio::fs::File::open(path)
        .await
        .map_err(io_error)?
        .take(MAX_SPEECH_PLAN_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(io_error)?;
    if bytes.len() > MAX_SPEECH_PLAN_BYTES {
        return Err(invalid(
            "Speech plan or manifest grew beyond its byte limit",
        ));
    }
    Ok(bytes)
}

struct Observer {
    observer: Arc<dyn SpeechProgressObserver>,
    run_id: Uuid,
    attempt_id: Uuid,
    plan_sha256: String,
}

#[async_trait]
impl TtsExecutionObserver for Observer {
    async fn saved_checkpoint(&self, checkpoint: &SavedCheckpoint) -> Result<(), PortError> {
        self.observer
            .checkpoint(SpeechCheckpoint {
                run_id: self.run_id,
                attempt_id: self.attempt_id,
                plan_sha256: self.plan_sha256.clone(),
                sequence: u32::try_from(checkpoint.sequence)
                    .map_err(|_| invalid("Checkpoint sequence overflow"))?,
                utterance_id: checkpoint.utterance_id.clone(),
                cue_id: checkpoint.cue_id.clone(),
                chunk_id: checkpoint.chunk_id.clone(),
                cache_key: checkpoint.cache_key.clone(),
                audio_sha256: checkpoint.audio_sha256.clone(),
                duration_ms: checkpoint.duration_ms,
                origin: checkpoint.status.clone(),
            })
            .await
    }
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
fn io_error(e: std::io::Error) -> PortError {
    PortError::Io {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn retained_json_is_bounded_before_deserialization()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("oversized.json");
        let file = std::fs::File::create(&path)?;
        file.set_len(super::MAX_SPEECH_PLAN_BYTES as u64 + 1)?;
        drop(file);
        assert!(super::read_bounded_json(&path).await.is_err());
        assert!(super::read_bounded_json(root.path()).await.is_err());
        std::fs::write(&path, b"{\"kept\":true}")?;
        assert_eq!(super::read_bounded_json(&path).await?, b"{\"kept\":true}");
        Ok(())
    }
}
