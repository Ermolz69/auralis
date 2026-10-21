use super::{SpeechSourceRequest, SpeechUseCases, conflict, hash, invalid};
use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::{
    marker::MarkerPreparation,
    speech_workflow::{SpeechReview, SpeechSourceSnapshot},
};
use uuid::Uuid;

impl SpeechUseCases {
    pub async fn prepare_text(
        &self,
        project: ProjectId,
        request: SpeechSourceRequest,
    ) -> Result<MarkerPreparation, ApplicationError> {
        let source = self.source(&project, request).await?;
        self.marker
            .as_ref()
            .ok_or_else(|| {
                invalid(
                    self.marker_error
                        .clone()
                        .unwrap_or_else(|| "Маркировщик не настроен".into()),
                )
            })?
            .prepare(source)
            .await
            .map_err(ApplicationError::from)
    }

    pub async fn review_prepared(
        &self,
        project: ProjectId,
        request: SpeechSourceRequest,
        preparation: MarkerPreparation,
        confirmed: bool,
    ) -> Result<SpeechReview, ApplicationError> {
        if !confirmed {
            return Err(invalid("Подтвердите: Текст проверен").into());
        }
        if preparation.version != "1.0.0" {
            return Err(invalid("Неизвестная версия подготовки текста").into());
        }
        let original = self.source(&project, request).await?;
        if original != preparation.original {
            return Err(conflict("Исходный текст изменился; повторите подготовку").into());
        }
        validate_prepared(&original, &preparation.prepared)?;
        let mut source = preparation.prepared;
        source.sha256 = if source.cues.is_empty() {
            hash(source.text.as_bytes())
        } else {
            hash(&serde_json::to_vec(&source.cues).map_err(|e| invalid(e.to_string()))?)
        };
        let review = SpeechReview {
            decision_id: Uuid::new_v4(),
            project_id: project,
            source,
            original_source: Some(original),
        };
        self.store.save_review(&review).await?;
        Ok(review)
    }
}

fn validate_prepared(
    original: &SpeechSourceSnapshot,
    prepared: &SpeechSourceSnapshot,
) -> Result<(), ApplicationError> {
    if prepared.kind != original.kind
        || prepared.source_id != original.source_id
        || prepared.revision != original.revision
        || prepared.text.trim().is_empty()
        || prepared.text.len() > 1_048_576
        || prepared.cues.len() != original.cues.len()
    {
        return Err(invalid("Подготовка изменила источник или его структуру").into());
    }
    for (a, b) in original.cues.iter().zip(&prepared.cues) {
        if a.id != b.id
            || a.role != b.role
            || a.start_ms != b.start_ms
            || a.end_ms != b.end_ms
            || b.text.trim().is_empty()
        {
            return Err(invalid("Подготовка изменила порядок, роли или время реплик").into());
        }
    }
    if !prepared.cues.is_empty()
        && prepared.text
            != prepared
                .cues
                .iter()
                .map(|cue| cue.text.as_str())
                .collect::<Vec<_>>()
                .join("\n")
    {
        return Err(invalid("Подготовленный текст не совпадает с репликами").into());
    }
    Ok(())
}
