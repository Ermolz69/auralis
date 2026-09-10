#![allow(clippy::unwrap_used)]

use super::{CommitManagedSourceImport, CommitStagedArtifactWrite, tests::create_valid_command};
use crate::error::PortError;
use domain::media::{ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState};
use domain::project::{Project, ProjectStatus};

fn staged_command(command: &CommitManagedSourceImport) -> CommitStagedArtifactWrite {
    CommitStagedArtifactWrite {
        project_id: command.project.id().clone(),
        artifact: command.artifact.clone(),
        staging_key: command.staging_key.clone(),
        final_key: command.final_key.clone(),
        temp_workspace_key: None,
    }
}

fn assert_outcome(result: Result<(), PortError>, expected: Option<&str>, scenario: &str) {
    match (result, expected) {
        (Ok(()), None) => {}
        (Err(PortError::Unexpected { message }), Some(expected)) => {
            assert_eq!(message, expected, "{scenario}");
        }
        (result, expected) => panic!("{scenario}: expected {expected:?}, got {result:?}"),
    }
}

fn assert_both(command: &CommitManagedSourceImport, expected: Option<&str>) {
    let scenario = format!(
        "final={:?}, staging={:?}, location={:?}",
        command.final_key, command.staging_key, command.artifact.location
    );
    assert_outcome(
        command.validate(),
        expected,
        &format!("managed: {scenario}"),
    );
    assert_outcome(
        staged_command(command).validate(),
        expected,
        &format!("staged: {scenario}"),
    );
}

#[test]
fn both_commands_reject_invalid_key_syntax() {
    for invalid_key in [
        "",
        "single-component",
        "{prefix}/",
        "{prefix}//video.mp4",
        "{prefix}/./video.mp4",
        "{prefix}/../video.mp4",
        "../{prefix}/video.mp4",
        "/{prefix}/video.mp4",
        r"\{prefix}\video.mp4",
        r"\\server\{prefix}\video.mp4",
        r"C:\{prefix}\video.mp4",
        "C:/{prefix}/video.mp4",
        "C:{prefix}/video.mp4",
        r"{prefix}\video.mp4",
        "s3://bucket/{prefix}/video.mp4",
    ] {
        for invalid_final in [false, true] {
            let (mut command, project_id, _) = create_valid_command();
            let expected = if invalid_final {
                command.final_key = invalid_key.replace("{prefix}", &project_id.to_string());
                command.artifact.location = ArtifactLocation::StorageKey(command.final_key.clone());
                "final_key must be a clean relative storage key"
            } else {
                command.staging_key = invalid_key.replace("{prefix}", ".staging");
                "staging_key must be a clean relative storage key"
            };
            assert_both(&command, Some(expected));
        }
    }
}

#[test]
fn both_commands_preserve_ownership_location_and_error_precedence() {
    for (final_key, staging_key, location_override, expected) in [
        ("{project}/video.mp4", ".staging/video.mp4", None, None),
        (
            "{project}/.hidden/video.mp4",
            ".staging/.hidden/video.mp4",
            None,
            None,
        ),
        (
            "{project}/..hidden/video.mp4",
            ".staging/..hidden/video.mp4",
            None,
            None,
        ),
        (
            "{project}-other/video.mp4",
            ".staging/video.mp4",
            None,
            Some("final_key must start with the project ID"),
        ),
        (
            "other-project/video.mp4",
            ".staging/video.mp4",
            None,
            Some("final_key must start with the project ID"),
        ),
        (
            "{project}/video.mp4",
            "other/video.mp4",
            None,
            Some("staging_key must start with .staging"),
        ),
        (
            "{project}/video.mp4",
            ".staging-other/video.mp4",
            None,
            Some("staging_key must start with .staging"),
        ),
        (
            "{project}/video.mp4",
            "{project}/video.mp4",
            None,
            Some("staging_key must start with .staging"),
        ),
        (
            ".staging/video.mp4",
            ".staging/video.mp4",
            None,
            Some("final_key must start with the project ID"),
        ),
        (
            "{project}/video.mp4",
            ".staging/video.mp4",
            Some("other/location.mp4"),
            Some("Artifact location must match final_key StorageKey"),
        ),
        (
            "../final",
            "../staging",
            Some("other/location.mp4"),
            Some("Artifact location must match final_key StorageKey"),
        ),
        (
            "../final",
            "../staging",
            None,
            Some("final_key must be a clean relative storage key"),
        ),
        (
            "other-project/video.mp4",
            "../staging",
            None,
            Some("staging_key must be a clean relative storage key"),
        ),
        (
            "other-project/video.mp4",
            "other/video.mp4",
            None,
            Some("final_key must start with the project ID"),
        ),
    ] {
        let (mut command, project_id, _) = create_valid_command();
        command.final_key = final_key.replace("{project}", &project_id.to_string());
        command.staging_key = staging_key.replace("{project}", &project_id.to_string());
        command.artifact.location = ArtifactLocation::StorageKey(
            location_override.unwrap_or(&command.final_key).to_string(),
        );
        assert_both(&command, expected);
    }
}

#[test]
fn both_commands_reject_local_paths_even_when_the_value_matches_final_key() {
    let (mut command, _, _) = create_valid_command();
    command.artifact.location = ArtifactLocation::LocalPath(command.final_key.clone());
    assert_both(
        &command,
        Some("Artifact location must match final_key StorageKey"),
    );
}

#[test]
fn managed_requirements_remain_separate_and_precede_shared_validation() {
    let (mut command, _, artifact_id) = create_valid_command();
    let ready_project = command.project.clone();
    let mut snapshot = ready_project.to_snapshot();
    snapshot.status = ProjectStatus::Draft;
    snapshot.source = None;
    command.project = Project::from_snapshot(snapshot).unwrap();
    command.artifact.kind = ArtifactKind::ExtractedAudio;
    command.artifact.state = ArtifactState::Ready;
    command.artifact.id = ArtifactId::new();

    for expected in [
        "Project must be in ReadyForProcessing status",
        "Artifact must be SourceVideo",
        "Artifact must be PendingFinalize",
        "Project source artifact ID does not match artifact",
        "Project source must be ManagedLocalFile",
    ] {
        assert_outcome(
            command.validate(),
            Some(expected),
            "managed-only requirement",
        );
        assert_outcome(
            staged_command(&command).validate(),
            None,
            "generic staged write",
        );

        command.artifact.location = ArtifactLocation::LocalPath(command.final_key.clone());
        assert_outcome(
            command.validate(),
            Some(expected),
            "managed check takes priority",
        );
        command.artifact.location = ArtifactLocation::StorageKey(command.final_key.clone());

        match expected {
            "Project must be in ReadyForProcessing status" => {
                command.project = ready_project.clone()
            }
            "Artifact must be SourceVideo" => command.artifact.kind = ArtifactKind::SourceVideo,
            "Artifact must be PendingFinalize" => {
                command.artifact.state = ArtifactState::PendingFinalize
            }
            "Project source artifact ID does not match artifact" => {
                command.artifact.id = artifact_id.clone();
                let mut snapshot = ready_project.to_snapshot();
                snapshot.source = Some(domain::media::MediaSource::YoutubeUrl {
                    url: "https://youtube.com/watch?v=test".into(),
                });
                command.project = Project::from_snapshot(snapshot).unwrap();
            }
            _ => {}
        }
    }
}
