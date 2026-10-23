use domain::{
    library::{LibraryAsset, LibraryAssetKind, LibraryLink, LibraryState},
    media::{Artifact, ArtifactKind, ArtifactLocation, ArtifactState},
    project::Project,
};

pub(super) fn asset_kind(kind: &ArtifactKind, project: &Project) -> Option<LibraryAssetKind> {
    use ArtifactKind::*;
    Some(match kind {
        SourceVideo
            if project
                .metadata()
                .is_some_and(|m| !m.has_video && m.has_audio) =>
        {
            LibraryAssetKind::Audio
        }
        SourceVideo | DownloadedVideo | PreviewVideo | FinalVideo => LibraryAssetKind::Video,
        ExtractedAudio | GeneratedSpeech | MixedAudio => LibraryAssetKind::Audio,
        OriginalSubtitle | TranslatedTranscript => LibraryAssetKind::Subtitle,
        GeneratedTranscript | NormalizedTranscript => LibraryAssetKind::Text,
        _ => return None,
    })
}

pub(super) fn add_asset(state: &mut LibraryState, asset: LibraryAsset) {
    if state.assets.iter().any(|a| {
        a.id == asset.id || (asset.artifact_id.is_some() && a.artifact_id == asset.artifact_id)
    }) {
        return;
    }
    if !state.standalone_owners.contains(&asset.owner_project_id)
        && !state.removed_projects.contains(&asset.owner_project_id)
    {
        state.links.push(LibraryLink {
            asset_id: asset.id.clone(),
            project_id: asset.owner_project_id.clone(),
            folder_id: None,
        });
    }
    state.assets.push(asset);
}

pub(super) fn hydrate_project(state: &mut LibraryState, project: &Project, artifacts: &[Artifact]) {
    for artifact in artifacts.iter().filter(|a| a.state == ArtifactState::Ready) {
        let Some(kind) = asset_kind(&artifact.kind, project) else {
            continue;
        };
        let path = match &artifact.location {
            ArtifactLocation::LocalPath(p) | ArtifactLocation::StorageKey(p) => p,
        };
        let format = ["srt", "vtt", "wav", "txt"]
            .into_iter()
            .find(|ext| path.to_lowercase().ends_with(&format!(".{ext}")))
            .map(str::to_owned);
        if kind == LibraryAssetKind::Text && format.as_deref() != Some("txt") {
            continue;
        }
        if matches!(
            artifact.kind,
            ArtifactKind::SourceVideo | ArtifactKind::DownloadedVideo
        ) && let Some(existing) = state
            .assets
            .iter_mut()
            .find(|a| a.id == project.id().to_string() && a.artifact_id.is_none())
        {
            existing.artifact_id = Some(artifact.id.to_string());
            existing.kind = kind;
            existing.format = format;
            continue;
        }
        let name = format!(
            "{} · {:?} · {}",
            project.title(),
            artifact.kind,
            &artifact.id.to_string()[..8]
        );
        add_asset(
            state,
            LibraryAsset {
                id: artifact.id.to_string(),
                owner_project_id: project.id().to_string(),
                artifact_id: Some(artifact.id.to_string()),
                name,
                kind,
                format,
                text: None,
                parent_id: None,
                tags: vec![],
                opened_at: None,
            },
        );
    }
    if project.source().is_some()
        && !artifacts.iter().any(|a| {
            matches!(
                a.kind,
                ArtifactKind::SourceVideo | ArtifactKind::DownloadedVideo
            ) && a.state == ArtifactState::Ready
        })
    {
        add_asset(
            state,
            LibraryAsset {
                id: project.id().to_string(),
                owner_project_id: project.id().to_string(),
                artifact_id: None,
                name: project.title().into(),
                kind: LibraryAssetKind::Video,
                format: None,
                text: None,
                parent_id: None,
                tags: vec![],
                opened_at: None,
            },
        );
    }
}
