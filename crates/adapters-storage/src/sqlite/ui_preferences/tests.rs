#![allow(clippy::unwrap_used)]
use super::SqliteUiPreferences;
use crate::sqlite::{SqliteProjectRepository, connect_sqlite};
use domain::project::Project;
use ports::{
    error::PortError, repository::ProjectRepository, ui_preferences::UiPreferencesRepository,
};

#[tokio::test]
async fn pin_import_waits_for_a_writer_and_duplicate_startup_import_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("startup-pins.sqlite");
    let pool = connect_sqlite(&path).await.unwrap();
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Preserved startup pin".into()).unwrap())
        .await
        .unwrap();
    let other_pool = connect_sqlite(&path).await.unwrap();
    let mut writer = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    sqlx::query("INSERT INTO project_pins VALUES (?, 0, 1)")
        .bind(project.id().to_string())
        .execute(&mut *writer)
        .await
        .unwrap();

    let entries = vec![(project.id().clone(), true)];
    let imported_entries = entries.clone();
    let prefs = SqliteUiPreferences::new(other_pool.clone());
    let mut importing = tokio::spawn(async move { prefs.import_pins(&imported_entries).await });
    // A deferred read-then-write upgrade returns BUSY immediately in WAL mode.
    // An immediate transaction waits at admission, before reading the marker.
    let early = tokio::time::timeout(std::time::Duration::from_millis(100), &mut importing).await;
    writer.commit().await.unwrap();
    let imported = match early {
        Ok(result) => result
            .unwrap()
            .expect("pin import must wait for the existing writer"),
        Err(_) => importing.await.unwrap().unwrap(),
    };
    assert!(imported.migrated);
    assert_eq!(imported.entries.len(), 1);
    assert!(
        !imported.entries[0].pinned,
        "legacy data must not overwrite the newer native pin"
    );

    let first = SqliteUiPreferences::new(pool.clone());
    let second = SqliteUiPreferences::new(other_pool.clone());
    let (left, right) = tokio::join!(first.import_pins(&entries), second.import_pins(&entries));
    assert_eq!(left.unwrap(), imported);
    assert_eq!(right.unwrap(), imported);
    let marker_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ui_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(marker_count, 1);
    pool.close().await;
    other_pool.close().await;
}

#[tokio::test]
async fn preferences_survive_reopen_and_legacy_never_overwrites_newer_values() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("preferences.sqlite");
    let pool = connect_sqlite(&path).await.unwrap();
    let prefs = SqliteUiPreferences::new(pool.clone());
    let projects = SqliteProjectRepository::new(pool.clone());
    let project = projects
        .create(Project::new("Pinned".into()).unwrap())
        .await
        .unwrap();
    assert!(prefs.get_theme().await.unwrap().is_none());
    let theme = prefs.import_theme("frost").await.unwrap();
    let changed = prefs.set_theme("abyss", theme.revision).await.unwrap();
    assert_eq!(prefs.import_theme("frost").await.unwrap(), changed);
    assert!(matches!(
        prefs.set_theme("ember", theme.revision).await,
        Err(PortError::Conflict { .. })
    ));
    let pin = prefs.set_pin(project.id(), false, 0).await.unwrap();
    let pins = prefs
        .import_pins(&[(project.id().clone(), true)])
        .await
        .unwrap();
    assert!(pins.migrated);
    assert_eq!(pins.entries, vec![pin.clone()]);
    let changed_pin = prefs
        .set_pin(project.id(), true, pin.revision)
        .await
        .unwrap();
    assert!(matches!(
        prefs.set_pin(project.id(), false, pin.revision).await,
        Err(PortError::Conflict { .. })
    ));
    pool.close().await;
    let reopened = connect_sqlite(&path).await.unwrap();
    let prefs = SqliteUiPreferences::new(reopened.clone());
    assert_eq!(prefs.get_theme().await.unwrap(), Some(changed));
    assert_eq!(prefs.get_pins().await.unwrap().entries, vec![changed_pin]);
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(project.id().to_string())
        .execute(&reopened)
        .await
        .unwrap();
    assert!(prefs.get_pins().await.unwrap().entries.is_empty());
    reopened.close().await;
}

#[tokio::test]
async fn pin_import_and_marker_roll_back_together_on_failure() {
    let root = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(root.path().join("rollback.sqlite"))
        .await
        .unwrap();
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Pin".into()).unwrap())
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TRIGGER reject_marker BEFORE INSERT ON ui_migrations BEGIN SELECT RAISE(ABORT, 'fixture'); END;").execute(&pool).await.unwrap();
    let prefs = SqliteUiPreferences::new(pool.clone());
    assert!(
        prefs
            .import_pins(&[(project.id().clone(), true)])
            .await
            .is_err()
    );
    let current = prefs.get_pins().await.unwrap();
    assert!(!current.migrated);
    assert!(current.entries.is_empty());
    pool.close().await;
}

#[tokio::test]
async fn version_four_upgrade_is_atomic_and_preserves_existing_projects() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("v4.sqlite");
    let pool = crate::sqlite::connection::create_pool(&path).await.unwrap();
    sqlx::raw_sql(concat!(
        include_str!("../schema.sql"),
        include_str!("../youtube_import_schema.sql")
    ))
    .execute(&pool)
    .await
    .unwrap();
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Preserved".into()).unwrap())
        .await
        .unwrap();
    pool.close().await;
    for _ in 0..2 {
        let pool = connect_sqlite(&path).await.unwrap();
        assert_eq!(
            SqliteProjectRepository::new(pool.clone())
                .get(project.id())
                .await
                .unwrap(),
            Some(project.clone())
        );
        assert!(
            SqliteUiPreferences::new(pool.clone())
                .get_theme()
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("PRAGMA user_version")
                .fetch_one(&pool)
                .await
                .unwrap(),
            18
        );
        pool.close().await;
    }
}
