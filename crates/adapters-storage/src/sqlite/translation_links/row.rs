use ports::error::PortError;
use ports::translation_links::TranslationLink;
use sqlx::{FromRow, SqliteExecutor};
use uuid::Uuid;

const SELECT_LINK: &str = "SELECT translation_id, project_id, source_artifact_id,
    source_sha256, source_format, source_language, target_language, active_run_id,
    selected_result_id, output_artifact_id, revision, created_at, updated_at
    FROM project_translations WHERE translation_id = ?";

#[derive(FromRow)]
pub(super) struct LinkRow {
    translation_id: String,
    project_id: String,
    source_artifact_id: String,
    source_sha256: String,
    source_format: String,
    source_language: String,
    target_language: String,
    active_run_id: Option<String>,
    selected_result_id: Option<String>,
    output_artifact_id: Option<String>,
    revision: i64,
    created_at: String,
    updated_at: String,
}

pub(super) async fn load<'e, E>(
    executor: E,
    translation_id: Uuid,
) -> Result<Option<TranslationLink>, PortError>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, LinkRow>(SELECT_LINK)
        .bind(translation_id.to_string())
        .fetch_optional(executor)
        .await
        .map_err(|error| super::super::helpers::map_sqlite_error("read_translation_link", error))?
        .map(LinkRow::decode)
        .transpose()
}

impl LinkRow {
    pub(super) fn decode(self) -> Result<TranslationLink, PortError> {
        let invalid = |field: &str| PortError::InvalidStoredData {
            entity_type: "TranslationLink".into(),
            entity_id: self.translation_id.clone(),
            field: field.into(),
            message: "invalid stored value".into(),
        };
        Ok(TranslationLink {
            translation_id: Uuid::parse_str(&self.translation_id)
                .map_err(|_| invalid("translation_id"))?,
            project_id: self.project_id.parse().map_err(|_| invalid("project_id"))?,
            source_artifact_id: self
                .source_artifact_id
                .parse()
                .map_err(|_| invalid("source_artifact_id"))?,
            source_sha256: self.source_sha256,
            source_format: self.source_format,
            source_language: self.source_language,
            target_language: self.target_language,
            active_run_id: self
                .active_run_id
                .map(|value| Uuid::parse_str(&value).map_err(|_| invalid("active_run_id")))
                .transpose()?,
            selected_result_id: self
                .selected_result_id
                .map(|value| Uuid::parse_str(&value).map_err(|_| invalid("selected_result_id")))
                .transpose()?,
            output_artifact_id: self
                .output_artifact_id
                .map(|value| value.parse().map_err(|_| invalid("output_artifact_id")))
                .transpose()?,
            revision: u64::try_from(self.revision).map_err(|_| invalid("revision"))?,
            created_at: self.created_at.parse().map_err(|_| invalid("created_at"))?,
            updated_at: self.updated_at.parse().map_err(|_| invalid("updated_at"))?,
        })
    }
}
