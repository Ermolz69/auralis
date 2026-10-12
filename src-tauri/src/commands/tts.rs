use application::usecases::speech::{
    PrepareSpeechRequest, SpeechRunView, SpeechSourceRequest, SpeechUseCases,
};
use application::usecases::speech_catalog::SpeechCatalogUseCase;
use ports::marker::MarkerPreparation;
use ports::puza_contract::PuzaVoiceBinding;
#[command]
pub async fn get_speech_voice_selection_cmd(
    project_id: String,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<Option<PuzaVoiceBinding>, CommandError> {
    speech
        .default_voice(parse_project_id(&project_id)?)
        .await
        .map_err(CommandError::from)
}
#[command]
pub async fn save_speech_voice_selection_cmd(
    project_id: String,
    profile_id: String,
    voice_id: String,
    reference_id: String,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<PuzaVoiceBinding, CommandError> {
    speech
        .save_voice(
            parse_project_id(&project_id)?,
            profile_id,
            voice_id,
            reference_id,
        )
        .await
        .map_err(CommandError::from)
}
use ports::speech_catalog::SpeechCatalogSnapshot;
use ports::speech_workflow::{SpeechReview, SpeechSourceSnapshot};
use std::sync::Arc;
use tauri::{State, command};
use uuid::Uuid;

#[command]
pub async fn prepare_speech_text_cmd(
    project_id: String,
    source: SpeechSourceRequest,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<MarkerPreparation, CommandError> {
    speech
        .prepare_text(parse_project_id(&project_id)?, source)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn review_prepared_speech_text_cmd(
    project_id: String,
    source: SpeechSourceRequest,
    preparation: MarkerPreparation,
    confirmed: bool,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<SpeechReview, CommandError> {
    speech
        .review_prepared(
            parse_project_id(&project_id)?,
            source,
            preparation,
            confirmed,
        )
        .await
        .map_err(CommandError::from)
}

use crate::dto::error::{CommandError, parse_project_id};

#[command]
pub async fn get_speech_catalog_cmd(
    project_id: String,
    catalog: State<'_, Arc<SpeechCatalogUseCase>>,
) -> Result<SpeechCatalogSnapshot, CommandError> {
    catalog
        .execute(&parse_project_id(&project_id)?)
        .await
        .map_err(CommandError::from)
}

fn run_uuid(value: &str) -> Result<Uuid, CommandError> {
    Uuid::parse_str(value).map_err(|_| {
        CommandError::from(application::error::ApplicationError::InvalidOperation {
            message: "Invalid speech run ID".into(),
        })
    })
}

#[command]
pub async fn get_speech_source_cmd(
    project_id: String,
    source: SpeechSourceRequest,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<SpeechSourceSnapshot, CommandError> {
    speech
        .source(&parse_project_id(&project_id)?, source)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn review_speech_text_cmd(
    project_id: String,
    source: SpeechSourceRequest,
    confirmed: bool,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<SpeechReview, CommandError> {
    speech
        .review(parse_project_id(&project_id)?, source, confirmed)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn prepare_speech_plan_cmd(
    project_id: String,
    request: PrepareSpeechRequest,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<SpeechRunView, CommandError> {
    speech
        .prepare(parse_project_id(&project_id)?, request)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn start_speech_run_cmd(
    project_id: String,
    run_id: String,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<SpeechRunView, CommandError> {
    speech
        .start(parse_project_id(&project_id)?, run_uuid(&run_id)?, false)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn resume_speech_run_cmd(
    project_id: String,
    run_id: String,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<SpeechRunView, CommandError> {
    speech
        .start(parse_project_id(&project_id)?, run_uuid(&run_id)?, true)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn get_speech_run_cmd(
    project_id: String,
    run_id: String,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<SpeechRunView, CommandError> {
    speech
        .state(parse_project_id(&project_id)?, run_uuid(&run_id)?)
        .await
        .map_err(CommandError::from)
}

#[command]
pub async fn list_speech_runs_cmd(
    project_id: String,
    speech: State<'_, Arc<SpeechUseCases>>,
) -> Result<Vec<SpeechRunView>, CommandError> {
    speech
        .list(parse_project_id(&project_id)?)
        .await
        .map_err(CommandError::from)
}
