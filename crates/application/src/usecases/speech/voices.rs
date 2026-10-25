use super::{SpeechUseCases, invalid};
use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::puza_contract::PuzaVoiceBinding;
impl SpeechUseCases {
    pub async fn default_voice(
        &self,
        project: ProjectId,
    ) -> Result<Option<PuzaVoiceBinding>, ApplicationError> {
        self.contract
            .as_ref()
            .ok_or_else(|| invalid("CONTRACT недоступен"))?
            .default_voice(&project)
            .await
    }
    pub async fn save_voice(
        &self,
        project: ProjectId,
        profile_id: String,
        voice_id: String,
        reference_id: String,
        allow_unverified_neutral: bool,
    ) -> Result<PuzaVoiceBinding, ApplicationError> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| invalid("TTS недоступен"))?
            .snapshot(&project)
            .await?;
        let profile = catalog
            .profiles
            .iter()
            .find(|profile| profile.profile_id == profile_id)
            .ok_or_else(|| invalid("Неизвестный профиль озвучки"))?;
        let binding = PuzaVoiceBinding {
            role_id: "narrator".into(),
            display_name: "Рассказчик".into(),
            voice_id,
            reference_id,
            engine_id: profile.engine_id.clone(),
            technical_reference_permission: allow_unverified_neutral
                .then(|| uuid::Uuid::new_v4().to_string()),
        };
        self.contract
            .as_ref()
            .ok_or_else(|| invalid("CONTRACT недоступен"))?
            .configure_voice(&project, binding)
            .await
    }
}
