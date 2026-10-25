use adapters_storage::sqlite::{
    connection::connect_sqlite, project_repository::SqliteProjectRepository,
};
use application::usecases::project::{
    contract::ProjectContractUseCase,
    create::{CreateProjectRequest, CreateProjectUseCase},
};
use async_trait::async_trait;
use domain::project::{Project, ProjectId};
use ports::{
    error::PortError,
    puza_contract::{PuzaContractPort, PuzaContractSnapshot, PuzaVoiceBinding},
    repository::ProjectRepository,
};
use std::sync::Arc;

struct ContractFixture {
    hash: String,
}

#[tokio::test]
async fn saved_default_voice_preserves_reviewed_source_and_other_roles_after_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("database.sqlite");
    let pool = connect_sqlite(&database).await?;
    let repository = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let project = repository
        .create(Project::new("Voice settings".into())?)
        .await?;
    let runtime = Arc::new(ContractFixture {
        hash: "a".repeat(64),
    });
    let service = ProjectContractUseCase {
        projects: repository.clone(),
        runtime: Some(runtime.clone()),
        unavailable_reason: "missing".into(),
    };
    let narrator = PuzaVoiceBinding {
        technical_reference_permission: None,
        role_id: "narrator".into(),
        display_name: "Рассказчик".into(),
        voice_id: "marina_elevenlabs_ru".into(),
        reference_id: "neutral-v2".into(),
        engine_id: "voxcpm2".into(),
    };
    let role = PuzaVoiceBinding {
        role_id: "role-0001".into(),
        display_name: "Алекс".into(),
        voice_id: "alex_analyst_elevenlabs_ru".into(),
        ..narrator.clone()
    };
    service
        .configure(
            project.id(),
            "Проверенный текст.".into(),
            vec![narrator.clone(), role.clone()],
        )
        .await?;
    let changed = PuzaVoiceBinding {
        voice_id: "alex_analyst_elevenlabs_ru".into(),
        ..narrator
    };
    service
        .configure_voice(project.id(), changed.clone())
        .await?;
    drop(service);
    drop(repository);
    pool.close().await;
    let repository = Arc::new(SqliteProjectRepository::new(
        connect_sqlite(&database).await?,
    ));
    let service = ProjectContractUseCase {
        projects: repository.clone(),
        runtime: Some(runtime),
        unavailable_reason: "missing".into(),
    };
    assert_eq!(
        service.default_voice(project.id()).await?,
        Some(changed.clone())
    );
    let snapshot = repository
        .contract_project(project.id())
        .await?
        .ok_or("missing receipt")?;
    assert_eq!(snapshot.source_text.as_deref(), Some("Проверенный текст."));
    assert_eq!(
        serde_json::from_str::<Vec<PuzaVoiceBinding>>(&snapshot.bindings_json)?,
        vec![changed, role]
    );
    Ok(())
}
#[async_trait]
impl PuzaContractPort for ContractFixture {
    async fn prepare_project(
        &self,
        _: &ProjectId,
        source_text: Option<String>,
        bindings: Vec<PuzaVoiceBinding>,
    ) -> Result<PuzaContractSnapshot, PortError> {
        Ok(PuzaContractSnapshot {
            contract_version: "0.2.0".into(),
            project_json: r#"{"version":1,"documents":[]}"#.into(),
            cast_json: r#"{"version":2,"roles":{}}"#.into(),
            source_text,
            bindings_json: serde_json::to_string(&bindings).map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?,
            sha256: self.hash.clone(),
        })
    }
}

#[tokio::test]
async fn missing_contract_rejects_project_creation_before_persisting_any_project()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("database.sqlite")).await?;
    let repository = Arc::new(SqliteProjectRepository::new(pool));
    let create = CreateProjectUseCase::new(repository.clone())
        .with_contract(None, "CONTRACT unavailable".into());
    let error = create
        .execute(CreateProjectRequest {
            title: "No contract".into(),
        })
        .await
        .err()
        .ok_or("must fail without CONTRACT")?;
    assert!(error.to_string().contains("CONTRACT unavailable"));
    assert!(repository.list().await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn contract_project_and_receipt_are_atomic_and_survive_reopening()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("database.sqlite");
    let pool = connect_sqlite(&database).await?;
    let repository = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let runtime = Arc::new(ContractFixture {
        hash: "a".repeat(64),
    });
    let create = CreateProjectUseCase::new(repository.clone())
        .with_contract(Some(runtime), "missing".into());
    let project = create
        .execute(CreateProjectRequest {
            title: "Validated project".into(),
        })
        .await?
        .project;
    assert!(repository.contract_project(project.id()).await?.is_some());
    drop(create);
    drop(repository);
    pool.close().await;
    let reopened = Arc::new(SqliteProjectRepository::new(
        connect_sqlite(&database).await?,
    ));
    assert_eq!(
        reopened
            .contract_project(project.id())
            .await?
            .map(|value| value.sha256),
        Some("a".repeat(64))
    );
    let bad = CreateProjectUseCase::new(reopened.clone()).with_contract(
        Some(Arc::new(ContractFixture {
            hash: "invalid".into(),
        })),
        "missing".into(),
    );
    assert!(
        bad.execute(CreateProjectRequest {
            title: "Rollback".into()
        })
        .await
        .is_err()
    );
    assert_eq!(reopened.list().await?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn legacy_projects_are_adopted_without_losing_data_and_voice_writes_use_cas()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let repository = Arc::new(SqliteProjectRepository::new(
        connect_sqlite(directory.path().join("database.sqlite")).await?,
    ));
    let project = repository
        .create(Project::new("Existing project".into())?)
        .await?;
    let contract = ProjectContractUseCase {
        projects: repository.clone(),
        runtime: Some(Arc::new(ContractFixture {
            hash: "a".repeat(64),
        })),
        unavailable_reason: "missing".into(),
    };
    let original = contract.ensure(project.id()).await?;
    let mut changed = original.clone();
    changed.sha256 = "b".repeat(64);
    repository
        .save_contract_project(project.id(), Some(&original.sha256), &changed)
        .await?;
    assert!(
        repository
            .save_contract_project(project.id(), Some(&original.sha256), &original)
            .await
            .is_err()
    );
    assert_eq!(
        repository
            .get(project.id())
            .await?
            .map(|value| value.title().to_owned()),
        Some("Existing project".into())
    );
    Ok(())
}
