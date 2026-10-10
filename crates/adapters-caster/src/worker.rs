use async_trait::async_trait;
use domain::project::ProjectId;
use ports::caster::{
    CastResult, CasterPort, LogicalCast, ManualSelection, ReferenceDetails, VoiceCatalogPage,
    VoiceCompatibility, VoiceDetails,
};
use ports::error::PortError;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use uuid::Uuid;

const MAX_FRAME_BYTES: u64 = 1024 * 1024;
const MAX_DIAGNOSTIC_BYTES: u64 = 8192;
const WORKER_DEADLINE: Duration = Duration::from_secs(30);
const PINNED_CASTER_VERSION: &str = "0.2.0";
const PINNED_CONTRACT_VERSION: &str = "0.2.0";

#[derive(Clone, Debug)]
pub struct CasterRuntimeConfig {
    pub python_executable: PathBuf,
    pub voice_store: PathBuf,
}

#[derive(Clone)]
pub struct CasterWorkerAdapter {
    runtime: Option<CasterRuntimeConfig>,
    projects_root: PathBuf,
}

#[derive(Deserialize)]
struct WorkerError {
    code: String,
    message: String,
}

#[derive(Deserialize)]
struct WorkerResponse<T> {
    version: u8,
    runtime: WorkerRuntimeIdentity,
    request_id: Option<String>,
    project_id: Option<String>,
    ok: bool,
    result: Option<T>,
    error: Option<WorkerError>,
}

#[derive(Deserialize)]
struct WorkerRuntimeIdentity {
    caster_version: String,
    contract_version: String,
}

impl CasterWorkerAdapter {
    pub fn new(runtime: Option<CasterRuntimeConfig>, projects_root: PathBuf) -> Self {
        Self {
            runtime,
            projects_root,
        }
    }

    fn runtime(&self) -> Result<&CasterRuntimeConfig, PortError> {
        self.runtime.as_ref().ok_or_else(|| PortError::Unsupported {
            message: "Caster runtime is not installed".into(),
        })
    }

    fn project_path(&self, project: &ProjectId) -> PathBuf {
        self.projects_root.join(project.to_string())
    }

    async fn call<T: DeserializeOwned>(
        &self,
        project: &ProjectId,
        operation: &str,
        payload: Value,
        request_id: String,
    ) -> Result<T, PortError> {
        let runtime = self.runtime()?;
        if !runtime.python_executable.is_file() {
            return Err(PortError::Unsupported {
                message: "Caster Python executable is unavailable".into(),
            });
        }
        let request = json!({
            "version": 1,
            "request_id": request_id,
            "project_id": project.to_string(),
            "operation": operation,
            "payload": payload,
        });
        let mut frame = serde_json::to_vec(&request).map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?;
        frame.push(b'\n');
        if frame.len() as u64 > MAX_FRAME_BYTES {
            return Err(PortError::InvalidSource {
                message: "Caster request exceeds the frame limit".into(),
            });
        }
        let mut child = Command::new(&runtime.python_executable)
            .arg("-I")
            .arg("-m")
            .arg("puza_caster")
            .env_remove("PYTHONPATH")
            .env_remove("PYTHONHOME")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
        let mut stdin = child.stdin.take().ok_or_else(|| PortError::Unexpected {
            message: "Caster stdin was unavailable".into(),
        })?;
        stdin
            .write_all(&frame)
            .await
            .map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
        stdin.shutdown().await.map_err(|error| PortError::Io {
            message: error.to_string(),
        })?;
        drop(stdin);
        let stdout = child.stdout.take().ok_or_else(|| PortError::Unexpected {
            message: "Caster stdout was unavailable".into(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| PortError::Unexpected {
            message: "Caster stderr was unavailable".into(),
        })?;
        let outcome = tokio::time::timeout(WORKER_DEADLINE, async {
            let read_stdout = async {
                let mut data = Vec::new();
                stdout
                    .take(MAX_FRAME_BYTES + 2)
                    .read_to_end(&mut data)
                    .await
                    .map(|_| data)
            };
            let read_stderr = async {
                let mut data = Vec::new();
                stderr
                    .take(MAX_DIAGNOSTIC_BYTES + 1)
                    .read_to_end(&mut data)
                    .await
                    .map(|_| data)
            };
            tokio::try_join!(read_stdout, read_stderr, child.wait())
        })
        .await;
        let (output, diagnostics, status) = match outcome {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                return Err(PortError::Io {
                    message: error.to_string(),
                });
            }
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(PortError::ExternalToolFailed {
                    tool: "puza-caster".into(),
                    message: "Worker deadline exceeded".into(),
                });
            }
        };
        if !status.success()
            || !diagnostics.is_empty()
            || output.len() as u64 > MAX_FRAME_BYTES + 1
            || !output.ends_with(b"\n")
            || output[..output.len() - 1].contains(&b'\n')
        {
            return Err(PortError::ExternalToolFailed {
                tool: "puza-caster".into(),
                message: "Invalid worker completion or frame".into(),
            });
        }
        let response: WorkerResponse<T> =
            serde_json::from_slice(&output).map_err(|error| PortError::InvalidStoredData {
                entity_type: "caster_worker".into(),
                entity_id: request_id.clone(),
                field: "response".into(),
                message: error.to_string(),
            })?;
        if response.version != 1
            || response.runtime.caster_version != PINNED_CASTER_VERSION
            || response.runtime.contract_version != PINNED_CONTRACT_VERSION
            || response.request_id.as_deref() != Some(&request_id)
            || response.project_id.as_deref() != Some(&project.to_string())
        {
            return Err(PortError::Conflict {
                resource: "caster_worker".into(),
                message: "Stale or mismatched response".into(),
            });
        }
        if response.ok {
            return response.result.ok_or_else(|| PortError::InvalidStoredData {
                entity_type: "caster_worker".into(),
                entity_id: request_id,
                field: "result".into(),
                message: "Success has no result".into(),
            });
        }
        Err(map_worker_error(response.error))
    }

    fn payload_with_store(&self, extra: Value) -> Result<Value, PortError> {
        let runtime = self.runtime()?;
        let mut payload = extra
            .as_object()
            .cloned()
            .ok_or_else(|| PortError::Unexpected {
                message: "Caster payload must be an object".into(),
            })?;
        payload.insert("store".into(), json!(runtime.voice_store));
        Ok(Value::Object(payload))
    }

    async fn ensure_project_directory(&self, project: &ProjectId) -> Result<(), PortError> {
        tokio::fs::create_dir_all(&self.projects_root)
            .await
            .map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
        let path = self.project_path(project);
        match tokio::fs::symlink_metadata(&path).await {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tokio::fs::create_dir(&path)
                    .await
                    .map_err(|error| PortError::Io {
                        message: error.to_string(),
                    })?;
            }
            Err(error) => {
                return Err(PortError::Io {
                    message: error.to_string(),
                });
            }
        }
        self.existing_project_directory(project).await?;
        Ok(())
    }

    async fn existing_project_directory(&self, project: &ProjectId) -> Result<bool, PortError> {
        let path = self.project_path(project);
        match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(PortError::InvalidSource {
                    message: "Managed cast project path is not an ordinary directory".into(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => {
                return Err(PortError::Io {
                    message: error.to_string(),
                });
            }
        }
        let canonical_root =
            tokio::fs::canonicalize(&self.projects_root)
                .await
                .map_err(|error| PortError::Io {
                    message: error.to_string(),
                })?;
        let canonical_project =
            tokio::fs::canonicalize(&path)
                .await
                .map_err(|error| PortError::Io {
                    message: error.to_string(),
                })?;
        if canonical_project.parent() != Some(Path::new(&canonical_root)) {
            return Err(PortError::InvalidSource {
                message: "Managed cast project path escapes its root".into(),
            });
        }
        Ok(true)
    }
}

fn map_worker_error(error: Option<WorkerError>) -> PortError {
    let Some(error) = error else {
        return PortError::Unexpected {
            message: "Caster returned an error without a code".into(),
        };
    };
    match error.code.as_str() {
        "project_busy" => PortError::Busy {
            message: error.message,
        },
        "stale_cast_revision" | "cast_recovery_required" | "cast_request_conflict" => {
            PortError::Conflict {
                resource: "cast".into(),
                message: error.message,
            }
        }
        "voice_store_missing" | "unknown_voice_reference" => PortError::NotFound {
            resource: error.message,
        },
        "invalid_worker_request"
        | "invalid_schema"
        | "voice_reference_unqualified"
        | "voice_reference_incompatible"
        | "unsupported_cast_engine" => PortError::InvalidSource {
            message: error.message,
        },
        _ => PortError::ExternalToolFailed {
            tool: "puza-caster".into(),
            message: format!("{}: {}", error.code, error.message),
        },
    }
}

#[async_trait]
impl CasterPort for CasterWorkerAdapter {
    async fn catalog_page(
        &self,
        project: &ProjectId,
        after: Option<String>,
        limit: u32,
    ) -> Result<VoiceCatalogPage, PortError> {
        let payload = self.payload_with_store(json!({"after": after, "limit": limit}))?;
        self.call(project, "catalog_page", payload, Uuid::new_v4().to_string())
            .await
    }
    async fn inspect_voice(
        &self,
        project: &ProjectId,
        voice_id: String,
        after_reference: Option<String>,
        limit: u32,
    ) -> Result<VoiceDetails, PortError> {
        let payload = self.payload_with_store(
            json!({"voice_id": voice_id, "after_reference": after_reference, "limit": limit}),
        )?;
        self.call(
            project,
            "inspect_voice",
            payload,
            Uuid::new_v4().to_string(),
        )
        .await
    }
    async fn inspect_reference(
        &self,
        project: &ProjectId,
        voice_id: String,
        reference_id: String,
    ) -> Result<ReferenceDetails, PortError> {
        let payload =
            self.payload_with_store(json!({"voice_id": voice_id, "reference_id": reference_id}))?;
        self.call(
            project,
            "inspect_reference",
            payload,
            Uuid::new_v4().to_string(),
        )
        .await
    }
    async fn check_compatibility(
        &self,
        project: &ProjectId,
        voice_id: String,
        engine_id: String,
        emotion_id: String,
    ) -> Result<VoiceCompatibility, PortError> {
        let payload = self.payload_with_store(
            json!({"voice_id": voice_id, "engine_id": engine_id, "emotion_id": emotion_id}),
        )?;
        self.call(
            project,
            "check_compatibility",
            payload,
            Uuid::new_v4().to_string(),
        )
        .await
    }
    async fn current_cast(&self, project: &ProjectId) -> Result<Option<CastResult>, PortError> {
        if !self.existing_project_directory(project).await? {
            return Ok(None);
        }
        #[derive(Deserialize)]
        struct Current {
            current: Option<CastResult>,
        }
        let current: Current = self
            .call(
                project,
                "current_cast",
                json!({"project": self.project_path(project)}),
                Uuid::new_v4().to_string(),
            )
            .await?;
        Ok(current.current)
    }
    async fn assign_roles(
        &self,
        project: &ProjectId,
        cast: LogicalCast,
        selections: BTreeMap<String, ManualSelection>,
        expected_revision: Option<String>,
        request_id: String,
    ) -> Result<CastResult, PortError> {
        self.ensure_project_directory(project).await?;
        let payload = self.payload_with_store(json!({"project": self.project_path(project), "cast": cast, "selections": selections, "expected_revision": expected_revision}))?;
        self.call(project, "assign_roles", payload, request_id)
            .await
    }
    async fn recover_cast(&self, project: &ProjectId) -> Result<String, PortError> {
        if !self.existing_project_directory(project).await? {
            return Ok("not_needed".into());
        }
        #[derive(Deserialize)]
        struct Status {
            status: String,
        }
        let response: Status = self
            .call(
                project,
                "recover_cast",
                json!({"project": self.project_path(project)}),
                Uuid::new_v4().to_string(),
            )
            .await?;
        Ok(response.status)
    }
}
