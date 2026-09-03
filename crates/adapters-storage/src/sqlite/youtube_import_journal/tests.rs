#![allow(clippy::unwrap_used)]

use super::SqliteYoutubeImportJournal;
use crate::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{SqliteProjectRepository, connect_sqlite},
};
use domain::{
    media::{ArtifactKind, MediaMetadata, MediaSource},
    project::Project,
};
use ports::{
    repository::ProjectRepository,
    storage::ArtifactStore,
    transaction::{CommitStagedArtifactWrite, CommitYoutubeImport},
    workspace::TempWorkspacePort,
    youtube_import::{YoutubeImportJournal, YoutubeImportSession, YoutubeImportState},
};
use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub(super) fn pause_before_commit() {
    if let Ok(root) = std::env::var("AURALIS_JOURNAL_CRASH_ROOT") {
        std::fs::write(Path::new(&root).join("checkpoint"), b"uncommitted").unwrap();
        loop {
            std::thread::park();
        }
    }
}

async fn session(root: &Path, original: Option<Project>) -> YoutubeImportSession {
    let original_updated_at = original.as_ref().map(Project::updated_at);
    let mut project = original.unwrap_or_else(|| Project::new("Journal crash".into()).unwrap());
    let metadata: MediaMetadata = serde_json::from_value(serde_json::json!({"duration_ms":1000,"has_video":true,"has_audio":true,"streams":[],"audio_tracks":[]})).unwrap();
    project
        .import_source(
            MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=journal".into(),
            },
            Some(metadata),
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    let allocation = LocalTempWorkspace::new(root)
        .create_allocation(project.id(), "youtube-resume")
        .await
        .unwrap();
    let source = allocation.absolute_path.join("original.mp4");
    std::fs::write(&source, b"video").unwrap();
    let staged = LocalArtifactStore::new(root.join("artifacts"))
        .import_external_file(
            project.id(),
            ArtifactKind::DownloadedVideo,
            &source,
            Some("original"),
        )
        .await
        .unwrap();
    YoutubeImportSession {
        project: project.to_snapshot(),
        original_updated_at,
        workspace_key: allocation.workspace_key.clone(),
        write: Some(CommitStagedArtifactWrite {
            project_id: project.id().clone(),
            artifact: staged.artifact,
            staging_key: staged.staging_key,
            final_key: staged.final_key,
            temp_workspace_key: Some(allocation.workspace_key),
        }),
        state: YoutubeImportState::Staged,
        revision: 1,
    }
}

fn command(session: &YoutubeImportSession) -> CommitYoutubeImport {
    CommitYoutubeImport {
        project: Project::from_snapshot(session.project.clone()).unwrap(),
        write: session.write.clone().unwrap(),
        original_updated_at: session.original_updated_at,
    }
}

#[tokio::test]
async fn youtube_journal_crash_child() {
    let Ok(root) = std::env::var("AURALIS_JOURNAL_CRASH_ROOT") else {
        return;
    };
    let pool = connect_sqlite(Path::new(&root).join("test.sqlite"))
        .await
        .unwrap();
    let journal = SqliteYoutubeImportJournal::new(pool);
    let session = journal.list().await.unwrap().remove(0);
    journal.commit(&session, command(&session)).await.unwrap();
    panic!("commit checkpoint was not reached");
}

#[tokio::test]
async fn youtube_journal_real_kill_inside_commit_rolls_back_all_writes_and_session_deletion() {
    for existing in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("test.sqlite");
        let pool = connect_sqlite(&db).await.unwrap();
        let repo = SqliteProjectRepository::new(pool.clone());
        let original = if existing {
            Some(
                repo.create(Project::new("Keep".into()).unwrap())
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        let expected = session(root.path(), original.clone()).await;
        SqliteYoutubeImportJournal::new(pool.clone())
            .insert(&expected)
            .await
            .unwrap();
        pool.close().await;
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "sqlite::youtube_import_journal::tests::youtube_journal_crash_child",
                "--nocapture",
            ])
            .env("AURALIS_JOURNAL_CRASH_ROOT", root.path())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let started = Instant::now();
        while !root.path().join("checkpoint").exists()
            && started.elapsed() < Duration::from_secs(20)
        {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let reached = root.path().join("checkpoint").exists();
        let _ = child.kill();
        child.wait().unwrap();
        assert!(reached, "child did not reach the uncommitted transaction");
        let pool = connect_sqlite(&db).await.unwrap();
        assert_eq!(
            SqliteProjectRepository::new(pool.clone())
                .list()
                .await
                .unwrap(),
            original.into_iter().collect::<Vec<_>>()
        );
        for query in [
            "SELECT COUNT(*) FROM artifacts",
            "SELECT COUNT(*) FROM outbox_messages",
        ] {
            assert_eq!(
                sqlx::query_scalar::<_, i64>(query)
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                0
            );
        }
        let journal = SqliteYoutubeImportJournal::new(pool.clone());
        let recovered = journal.list().await.unwrap().remove(0);
        assert_eq!(
            serde_json::to_value(&recovered).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
        journal
            .commit(&recovered, command(&recovered))
            .await
            .unwrap();
        assert!(journal.list().await.unwrap().is_empty());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM artifacts")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        pool.close().await;
    }
}

#[tokio::test]
async fn youtube_journal_v3_migration_preserves_projects_and_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("v3.sqlite");
    let pool = crate::sqlite::connection::create_pool(&db).await.unwrap();
    sqlx::raw_sql(include_str!("../schema.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Preserved".into()).unwrap())
        .await
        .unwrap();
    pool.close().await;
    for _ in 0..2 {
        let pool = connect_sqlite(&db).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("PRAGMA user_version")
                .fetch_one(&pool)
                .await
                .unwrap(),
            4
        );
        assert_eq!(
            SqliteProjectRepository::new(pool.clone())
                .get(project.id())
                .await
                .unwrap(),
            Some(project.clone())
        );
        assert!(
            SqliteYoutubeImportJournal::new(pool.clone())
                .list()
                .await
                .unwrap()
                .is_empty()
        );
        pool.close().await;
    }
}

#[tokio::test]
async fn youtube_journal_rejects_substituted_commit_payload_and_invalid_state() {
    let root = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(root.path().join("test.sqlite"))
        .await
        .unwrap();
    let journal = SqliteYoutubeImportJournal::new(pool.clone());
    let mut session = session(root.path(), None).await;
    journal.insert(&session).await.unwrap();
    let mut changed = command(&session);
    changed.write.staging_key = ".staging/foreign/file.mp4".into();
    assert!(journal.commit(&session, changed).await.is_err());
    session.state = YoutubeImportState::Downloading;
    assert!(journal.checkpoint(&mut session).await.is_err());
    assert_eq!(journal.list().await.unwrap()[0].revision, 1);
    pool.close().await;
}
