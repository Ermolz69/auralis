use application::usecases::caster::CasterUseCases;
use ports::caster::{
    CastResult, LogicalCast, ManualSelection, VoiceCatalogPage, VoiceCompatibility, VoiceDetails,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tauri::{AppHandle, State, command};

use super::caster_audio::{ReferencePlaybackDto, prepare_reference_playback};

use crate::dto::error::{CommandError, parse_project_id};

#[command]
pub async fn list_voice_catalog_cmd(
    project_id: String,
    after: Option<String>,
    limit: u32,
    caster: State<'_, Arc<CasterUseCases>>,
) -> Result<VoiceCatalogPage, CommandError> {
    caster
        .catalog_page(&parse_project_id(&project_id)?, after, limit)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn inspect_voice_cmd(
    project_id: String,
    voice_id: String,
    after_reference: Option<String>,
    limit: u32,
    caster: State<'_, Arc<CasterUseCases>>,
) -> Result<VoiceDetails, CommandError> {
    caster
        .inspect_voice(
            &parse_project_id(&project_id)?,
            voice_id,
            after_reference,
            limit,
        )
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn inspect_voice_reference_cmd(
    app: AppHandle,
    project_id: String,
    voice_id: String,
    reference_id: String,
    caster: State<'_, Arc<CasterUseCases>>,
) -> Result<ReferencePlaybackDto, CommandError> {
    let reference = caster
        .inspect_reference(&parse_project_id(&project_id)?, voice_id, reference_id)
        .await
        .map_err(CommandError::from)?;
    prepare_reference_playback(&app, reference).await
}

#[command]
pub async fn check_voice_compatibility_cmd(
    project_id: String,
    voice_id: String,
    engine_id: String,
    emotion_id: String,
    caster: State<'_, Arc<CasterUseCases>>,
) -> Result<VoiceCompatibility, CommandError> {
    caster
        .check_compatibility(
            &parse_project_id(&project_id)?,
            voice_id,
            engine_id,
            emotion_id,
        )
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn get_manual_cast_cmd(
    project_id: String,
    caster: State<'_, Arc<CasterUseCases>>,
) -> Result<Option<CastResult>, CommandError> {
    caster
        .current_cast(&parse_project_id(&project_id)?)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn assign_manual_cast_cmd(
    project_id: String,
    cast: LogicalCast,
    selections: BTreeMap<String, ManualSelection>,
    expected_revision: Option<String>,
    request_id: String,
    caster: State<'_, Arc<CasterUseCases>>,
) -> Result<CastResult, CommandError> {
    caster
        .assign_roles(
            &parse_project_id(&project_id)?,
            cast,
            selections,
            expected_revision,
            request_id,
        )
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn recover_manual_cast_cmd(
    project_id: String,
    caster: State<'_, Arc<CasterUseCases>>,
) -> Result<String, CommandError> {
    caster
        .recover_cast(&parse_project_id(&project_id)?)
        .await
        .map_err(CommandError::from)
}
