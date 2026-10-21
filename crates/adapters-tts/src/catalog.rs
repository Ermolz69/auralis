use async_trait::async_trait;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::speech_catalog::{
    SpeechCatalogPort, SpeechCatalogSnapshot, SpeechProfileOption, SpeechReferenceOption,
    SpeechVoiceOption,
};

use crate::client::TtsWorkerClient;

#[async_trait]
impl SpeechCatalogPort for TtsWorkerClient {
    async fn snapshot(&self, project: &ProjectId) -> Result<SpeechCatalogSnapshot, PortError> {
        self.verify_integrity(None).await?;
        let capabilities = self.capabilities(project).await?;
        let catalog = self.catalog(project).await?;
        let profiles = capabilities
            .installed_profiles
            .into_iter()
            .filter(|profile| {
                matches!(
                    (profile.profile_id.as_str(), profile.engine_id.as_str()),
                    ("native-plan-voxcpm2", "voxcpm2") | ("native-plan-espeech", "espeech")
                )
            })
            .map(|profile| SpeechProfileOption {
                profile_id: profile.profile_id,
                profile_sha256: profile.profile_sha256,
                engine_id: profile.engine_id,
            })
            .collect();
        let voices = catalog
            .voices
            .into_iter()
            .filter(|voice| !matches!(voice.voice_id.as_str(), "ciri" | "geralt"))
            .map(|voice| SpeechVoiceOption {
                voice_id: voice.voice_id,
                display_name: voice.display_name,
                description: voice.description,
                manifest_sha256: voice.manifest_sha256,
                references: voice
                    .references
                    .into_iter()
                    .map(|reference| SpeechReferenceOption {
                        reference_id: reference.reference_id,
                        engine_id: reference.engine_id,
                        emotion_id: reference.emotion_id,
                        transcript: reference.transcript,
                        audio_sha256: reference.audio_sha256,
                        qualification_status: reference.qualification,
                    })
                    .collect(),
            })
            .collect();
        Ok(SpeechCatalogSnapshot { profiles, voices })
    }
}
