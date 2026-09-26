use crate::format_inspector::read_bounded;

use async_trait::async_trait;
use auralis_translation::{ResultId, RunId, SegmentId};
use auralis_translation_formats::srt::SrtParsePolicy;
use auralis_translation_formats::vtt::VttParsePolicy;
use auralis_translation_sqlite::{BranchEditSpec, DbError, EditSpec, SqliteConfig, TranslateDb};
use ports::error::PortError;
use ports::translation_links::{TranslationLink, TranslationRunIntent};
use ports::translation_result_edit::{TranslationResultEdit, TranslationResultEditorPort};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct TranslateResultEditor {
    database_path: PathBuf,
}

impl TranslateResultEditor {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

#[async_trait]
impl TranslationResultEditorPort for TranslateResultEditor {
    async fn commit_branch_edit(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        branch: ports::translation_branch_edit::TranslationBranchEdit,
    ) -> Result<Uuid, PortError> {
        let database_path = self.database_path.clone();
        let source_path = source_path.to_path_buf();
        let link = link.clone();
        let intent = intent.clone();
        tokio::task::spawn_blocking(move || {
            commit_edit(
                &database_path,
                &source_path,
                &link,
                &intent,
                branch.edit,
                Some(branch.expected_head_result_id),
            )
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
    async fn commit_edit(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        edit: TranslationResultEdit,
    ) -> Result<Uuid, PortError> {
        let database_path = self.database_path.clone();
        let source_path = source_path.to_path_buf();
        let link = link.clone();
        let intent = intent.clone();
        tokio::task::spawn_blocking(move || {
            commit_edit(&database_path, &source_path, &link, &intent, edit, None)
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
}

fn commit_edit(
    database_path: &Path,
    source_path: &Path,
    link: &TranslationLink,
    intent: &TranslationRunIntent,
    edit: TranslationResultEdit,
    expected_head: Option<Uuid>,
) -> Result<Uuid, PortError> {
    if link.translation_id != intent.translation_id
        || expected_head.is_none()
            && link
                .active_run_id
                .is_some_and(|active| active != intent.run_id)
    {
        return Err(conflict("project link and frozen run intent differ"));
    }
    let run_id = RunId::new(intent.run_id).ok_or_else(|| invalid("run ID is nil"))?;
    let base_result_id =
        ResultId::new(edit.base_result_id).ok_or_else(|| invalid("base result ID is nil"))?;
    let result_id = ResultId::new(edit.result_id).ok_or_else(|| invalid("result ID is nil"))?;
    let segment_id =
        SegmentId::new(edit.segment_id).ok_or_else(|| invalid("segment ID must be positive"))?;
    let max_bytes = match link.source_format.as_str() {
        "srt" => SrtParsePolicy::default().max_bytes(),
        "vtt" => VttParsePolicy::default().max_bytes(),
        _ => return Err(invalid("unsupported subtitle format")),
    };
    let source = read_bounded(source_path, max_bytes)?;
    if !database_path.is_file() {
        return Err(PortError::NotFound {
            resource: "Translate database".into(),
        });
    }
    let mut database = TranslateDb::open(database_path, SqliteConfig::default())
        .map_err(|error| storage("open_translate_database", error))?;
    let plan = crate::frozen_run::verify(&database, &source, link, intent)?;
    let base = database
        .result(base_result_id)
        .map_err(|error| storage("read_translate_base_result", error))?;
    if base.run_id != run_id || base.source_hash != plan.source_hash() {
        return Err(conflict("base result does not belong to the active run"));
    }
    let edit = EditSpec {
        base_result_id,
        result_id,
        segment_id,
        lines: edit.lines,
    };
    let committed = if let Some(head) = expected_head {
        let expected_head_result_id =
            ResultId::new(head).ok_or_else(|| invalid("observed head ID is nil"))?;
        database.commit_branch_edit(
            &BranchEditSpec {
                edit,
                expected_head_result_id,
            },
            &plan,
        )
    } else {
        database.commit_edit(&edit, &plan)
    }
    .map_err(map_edit_error)?;
    Ok(committed.result_id.get())
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "translation result edit".into(),
        message: message.into(),
    }
}

fn map_edit_error(error: DbError) -> PortError {
    match error {
        DbError::Conflict(message) => conflict(message),
        DbError::InvalidSpec(message) => invalid(message),
        DbError::Verification(message) => invalid(&message),
        other => storage("commit_translate_result_edit", other),
    }
}

fn storage(operation: &'static str, error: DbError) -> PortError {
    PortError::Storage {
        operation,
        message: error.to_string(),
    }
}
