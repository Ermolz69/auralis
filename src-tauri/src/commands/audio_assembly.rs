use crate::dto::error::{CommandError, parse_project_id};
use application::usecases::audio_assembly::{
    AudioAssemblyPreview, AudioAssemblyUseCases, AudioAssemblyView, InspectAssemblyRequest,
    PrepareAssemblyRequest,
};
use ports::cancellation::CancellationToken;
use std::sync::Arc;
use tauri::{State, command};
use uuid::Uuid;

fn assembly_id(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value).map_err(|_| {
        CommandError::from(application::error::ApplicationError::InvalidOperation {
            message: "Invalid audio assembly ID".into(),
        })
    })
}

#[command]
pub async fn inspect_audio_assembly_cmd(
    project_id: String,
    request: InspectAssemblyRequest,
    cases: State<'_, Arc<AudioAssemblyUseCases>>,
) -> Result<AudioAssemblyPreview, CommandError> {
    cases
        .inspect(
            parse_project_id(&project_id)?,
            request,
            CancellationToken::new(),
        )
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn prepare_audio_assembly_cmd(
    project_id: String,
    request: PrepareAssemblyRequest,
    cases: State<'_, Arc<AudioAssemblyUseCases>>,
) -> Result<AudioAssemblyView, CommandError> {
    cases
        .prepare(parse_project_id(&project_id)?, request)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn start_audio_assembly_cmd(
    project_id: String,
    assembly_id: String,
    cases: State<'_, Arc<AudioAssemblyUseCases>>,
) -> Result<AudioAssemblyView, CommandError> {
    cases
        .start(
            parse_project_id(&project_id)?,
            self::assembly_id(&assembly_id)?,
            false,
        )
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn resume_audio_assembly_cmd(
    project_id: String,
    assembly_id: String,
    cases: State<'_, Arc<AudioAssemblyUseCases>>,
) -> Result<AudioAssemblyView, CommandError> {
    cases
        .start(
            parse_project_id(&project_id)?,
            self::assembly_id(&assembly_id)?,
            true,
        )
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn get_audio_assembly_cmd(
    project_id: String,
    assembly_id: String,
    cases: State<'_, Arc<AudioAssemblyUseCases>>,
) -> Result<AudioAssemblyView, CommandError> {
    cases
        .state(
            parse_project_id(&project_id)?,
            self::assembly_id(&assembly_id)?,
        )
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn verify_audio_assembly_cmd(
    project_id: String,
    assembly_id: String,
    cases: State<'_, Arc<AudioAssemblyUseCases>>,
) -> Result<AudioAssemblyView, CommandError> {
    cases
        .verify(
            parse_project_id(&project_id)?,
            self::assembly_id(&assembly_id)?,
        )
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn list_audio_assemblies_cmd(
    project_id: String,
    cases: State<'_, Arc<AudioAssemblyUseCases>>,
) -> Result<Vec<AudioAssemblyView>, CommandError> {
    cases
        .list(parse_project_id(&project_id)?)
        .await
        .map_err(CommandError::from)
}
