use adapters_storage::sqlite::{connection::connect_sqlite, ui_preferences::SqliteUiPreferences};
use application::usecases::system::ui_preferences::UiPreferencesUseCase;
use ports::ui_preferences::UiPreferencesRepository;
use std::sync::Arc;

#[tokio::test]
async fn removed_themes_migrate_once_and_survive_reopening()
-> Result<(), Box<dyn std::error::Error>> {
    for (old, expected) in [
        ("auralis", "dark"),
        ("abyss", "dark"),
        ("indigo", "dark"),
        ("ember", "dark"),
        ("violet", "dark"),
        ("frost", "light"),
        ("polar", "light"),
        ("sandstone", "light"),
    ] {
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("preferences.sqlite");
        let pool = connect_sqlite(&database).await?;
        let repository = Arc::new(SqliteUiPreferences::new(pool.clone()));
        repository.import_theme(old).await?;
        let service = UiPreferencesUseCase::new(repository.clone());
        let migrated = service.get_theme().await?.ok_or("missing theme")?;
        assert_eq!(migrated.value, expected);
        assert_eq!(migrated.revision, 2);
        assert_eq!(service.get_theme().await?, Some(migrated.clone()));
        assert!(service.set_theme(old, migrated.revision).await.is_err());
        drop(service);
        drop(repository);
        pool.close().await;
        let reopened = SqliteUiPreferences::new(connect_sqlite(&database).await?);
        assert_eq!(reopened.get_theme().await?, Some(migrated));
    }
    Ok(())
}

#[tokio::test]
async fn legacy_import_keeps_a_newer_choice_and_unknown_values_are_preserved()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("preferences.sqlite")).await?;
    let repository = Arc::new(SqliteUiPreferences::new(pool));
    let service = UiPreferencesUseCase::new(repository.clone());
    let imported = service.import_theme("frost").await?;
    assert_eq!(imported.value, "light");
    let selected = service.set_theme("dark", imported.revision).await?;
    assert_eq!(service.import_theme("sandstone").await?, selected);
    assert!(
        service
            .set_theme("future-theme", selected.revision)
            .await
            .is_err()
    );
    let unknown = repository
        .set_theme("future-theme", selected.revision)
        .await?;
    assert_eq!(service.get_theme().await?, Some(unknown));
    Ok(())
}
