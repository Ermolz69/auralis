use super::{
    AudioAssemblyUseCases, InspectAssemblyRequest,
    computed::ComputedAssembly,
    error::{conflict, invalid},
};
use crate::error::ApplicationError;
use domain::{
    media::{ArtifactKind, ArtifactLocation, ArtifactState},
    project::ProjectId,
    speech::{Sha256Digest, SpeechId},
};
use ports::{
    audio_inspection::{ManagedAudioInput, SpeechAudioInspectionRequest},
    audio_jobs::{AssemblySourceArtifact, AudioAssemblyIntent, AudioAssemblyPlan},
    cancellation::CancellationToken,
    speech_run::MAX_SPEECH_PLAN_BYTES,
    speech_workflow::SpeechPublication,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

impl AudioAssemblyUseCases {
    pub(super) async fn compute(
        &self,
        project: ProjectId,
        request: InspectAssemblyRequest,
        cancel: CancellationToken,
    ) -> Result<ComputedAssembly, ApplicationError> {
        self.projects
            .get(&project)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project.clone()))?;
        let source = self
            .speech
            .get_run(request.source_run_id)
            .await?
            .ok_or_else(|| invalid("Selected raw speech run is missing"))?;
        if source.project_id != project {
            return Err(conflict("Selected speech run belongs to another project").into());
        }
        if source.plan_json.len() > MAX_SPEECH_PLAN_BYTES
            || format!("{:x}", Sha256::digest(source.plan_json.as_bytes())) != source.plan_sha256
        {
            return Err(invalid("Selected speech plan bytes changed").into());
        }
        let publication = self
            .speech
            .publication(source.run_id)
            .await?
            .filter(|publication| publication.ready)
            .ok_or_else(|| {
                invalid("Wait for complete raw speech publication before audio assembly")
            })?;
        if publication.run_id != source.run_id {
            return Err(invalid("Raw speech publication identity changed").into());
        }
        let (raw_wav, aggregate) = self
            .source_file(&project, &publication, "wav", ArtifactKind::GeneratedSpeech)
            .await?;
        let (raw_manifest, manifest) = self
            .source_file(
                &project,
                &publication,
                "manifest",
                ArtifactKind::SpeechManifest,
            )
            .await?;
        let (layout, layout_input) = self
            .source_file(
                &project,
                &publication,
                "audio_layout",
                ArtifactKind::SpeechManifest,
            )
            .await?;
        let value: serde_json::Value = serde_json::from_str(&source.plan_json)
            .map_err(|_| invalid("Saved speech plan JSON is invalid"))?;
        let utterances = value
            .get("utterances")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| invalid("Saved speech plan has no ordered utterances"))?;
        if utterances.len() != source.expected_utterances as usize {
            return Err(invalid("Saved speech plan coverage changed").into());
        }
        let ids = utterances
            .iter()
            .map(|utterance| {
                SpeechId::parse(
                    utterance
                        .get("utterance_id")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| invalid("Saved speech utterance has no identity"))?,
                )
                .map_err(invalid)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let inspection = self
            .inspector
            .inspect(
                SpeechAudioInspectionRequest {
                    aggregate,
                    manifest,
                    layout: layout_input,
                    source_plan_sha256: Sha256Digest::parse(source.plan_sha256).map_err(invalid)?,
                    expected_utterance_ids: ids,
                    settings: request.settings,
                },
                cancel,
            )
            .await?;
        let plan = AudioAssemblyPlan {
            project_id: project.clone(),
            source_run_id: source.run_id,
            source_attempt_id: publication.attempt_id,
            raw_wav,
            raw_manifest,
            layout,
            ffmpeg_sha256: self.ffmpeg_sha256.clone(),
            assembly_policy: "pcm16_48k_exact_concat_v1".into(),
            timeline: inspection.timeline_request.clone(),
        };
        let plan_json = serde_json::to_string(&plan)
            .map_err(|_| invalid("Assembly preview cannot be serialized"))?;
        if plan_json.len() > ports::audio_jobs::MAX_ASSEMBLY_PLAN_BYTES {
            return Err(invalid("Assembly preview exceeds the plan byte ceiling").into());
        }
        let intent = AudioAssemblyIntent {
            assembly_id: Uuid::new_v4(),
            project_id: project,
            source_run_id: source.run_id,
            source_attempt_id: publication.attempt_id,
            plan_sha256: Sha256Digest::parse(format!("{:x}", Sha256::digest(plan_json.as_bytes())))
                .map_err(invalid)?,
            plan_json,
        };
        Ok(ComputedAssembly {
            intent,
            plan,
            inspection,
            profile_id: source.profile_id,
        })
    }

    async fn source_file(
        &self,
        project: &ProjectId,
        publication: &SpeechPublication,
        role: &str,
        kind: ArtifactKind,
    ) -> Result<(AssemblySourceArtifact, ManagedAudioInput), ApplicationError> {
        let files: Vec<_> = publication
            .files
            .iter()
            .filter(|file| file.kind == role)
            .collect();
        if files.len() != 1 {
            return Err(invalid(if role == "audio_layout" { "This raw result has no verified utterance layout; install the streaming TTS package and render a new result" } else { "Raw speech publication has missing or repeated files" }).into());
        }
        let file = files[0];
        let artifact = self
            .index
            .get(&file.artifact_id)
            .await?
            .ok_or_else(|| invalid("Raw speech artifact is not fully ready"))?;
        let ArtifactLocation::StorageKey(key) = &artifact.location else {
            return Err(invalid("Raw speech input is not managed").into());
        };
        if !key.starts_with(&format!("{project}/"))
            || artifact.kind != kind
            || artifact.state != ArtifactState::Ready
            || artifact.size_bytes != Some(file.size_bytes)
        {
            return Err(conflict("Raw speech artifact ownership, type or size changed").into());
        }
        let reference = AssemblySourceArtifact {
            artifact_id: file.artifact_id.clone(),
            sha256: Sha256Digest::parse(file.sha256.clone()).map_err(invalid)?,
            size_bytes: file.size_bytes,
        };
        let input = ManagedAudioInput {
            path: self.artifacts.resolve_artifact(&artifact).await?,
            sha256: reference.sha256.clone(),
            size_bytes: file.size_bytes,
        };
        Ok((reference, input))
    }
}
