use crate::source::{AssemblySource, digest, pin};
use domain::media::{ArtifactKind, ArtifactState};
use ports::{
    artifact_index::ArtifactIndex,
    storage::ArtifactStore,
    video_export::{REPLACE_AUDIO_POLICY, VideoAssemblySource, VideoInspection},
};

pub async fn video_source(
    fixture: &AssemblySource,
) -> Result<VideoAssemblySource, Box<dyn std::error::Error>> {
    let bytes = b"Synthetic source video; no codec acceptance";
    let file = fixture.directory.path().join("source-video.fixture");
    std::fs::write(&file, bytes)?;
    let staged = fixture
        .files
        .import_external_file(
            &fixture.project_id,
            ArtifactKind::DownloadedVideo,
            &file,
            Some("fixture.mp4"),
        )
        .await?;
    fixture
        .index
        .add(&fixture.project_id, &staged.artifact)
        .await?;
    fixture
        .files
        .finalize_staged_artifact(
            &staged.staging_key,
            &staged.final_key,
            Some(staged.size_bytes),
        )
        .await?;
    fixture
        .index
        .update_state(
            &staged.artifact.id,
            ArtifactState::Ready,
            Some(chrono::Utc::now()),
        )
        .await?;
    let hash = domain::speech::Sha256Digest::parse(digest(bytes))?;
    Ok(VideoAssemblySource {
        policy_id: REPLACE_AUDIO_POLICY.into(),
        artifact: ports::audio_jobs::AssemblySourceArtifact {
            artifact_id: staged.artifact.id,
            sha256: hash.clone(),
            size_bytes: bytes.len() as u64,
        },
        media: VideoInspection {
            sha256: hash,
            size_bytes: bytes.len() as u64,
            duration_ms: 5000,
            width: 320,
            height: 180,
            video_codec: "synthetic".into(),
            ffprobe_sha256: pin()?,
        },
    })
}
