use domain::outbox::{OutboxMessage, OutboxPayload};
use domain::project::{Project, ProjectStatus};
use ports::error::PortError;
use ports::transaction::CommitYoutubeImport;
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::artifact_writes::insert_new_artifact;
use super::outbox_writes::save_outbox_message;
use super::project_writes::update_project_conditional;
use crate::sqlite::helpers::map_sqlite_error;
use crate::sqlite::project_mapper::project_to_row_values;

pub(super) async fn commit(
    pool: &SqlitePool,
    command: CommitYoutubeImport,
) -> Result<(), PortError> {
    command.validate()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| map_sqlite_error("begin_youtube_import", e))?;
    write(&mut tx, command).await?;
    tx.commit()
        .await
        .map_err(|e| map_sqlite_error("commit_youtube_import", e))
}

pub(crate) async fn write(
    tx: &mut Transaction<'_, Sqlite>,
    command: CommitYoutubeImport,
) -> Result<(), PortError> {
    command.validate()?;
    match command.original_updated_at {
        Some(updated_at) => {
            update_project_conditional(
                tx,
                &command.project,
                updated_at,
                &ProjectStatus::Draft,
                None,
            )
            .await?;
        }
        None => insert_project(tx, &command.project).await?,
    }
    let write = command.write;
    insert_new_artifact(tx, &write.project_id, &write.artifact).await?;
    save_outbox_message(
        tx,
        &OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
            project_id: write.project_id,
            artifact_id: write.artifact.id,
            staging_key: write.staging_key,
            final_key: write.final_key,
        }),
    )
    .await?;
    if let Some(workspace_key) = write.temp_workspace_key {
        save_outbox_message(
            tx,
            &OutboxMessage::new(OutboxPayload::DeleteWorkspaceAllocation { workspace_key }),
        )
        .await?;
    }
    Ok(())
}

async fn insert_project(
    tx: &mut Transaction<'_, Sqlite>,
    project: &Project,
) -> Result<(), PortError> {
    let row = project_to_row_values(project)?;
    sqlx::query("INSERT INTO projects (id, revision, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(row.id).bind(row.revision).bind(row.title).bind(row.status)
        .bind(row.source_json).bind(row.metadata_json).bind(row.source_language)
        .bind(row.target_language).bind(row.transcript_json).bind(row.active_job_id)
        .bind(row.last_terminal_job_id).bind(row.created_at).bind(row.updated_at)
        .execute(&mut **tx).await.map_err(|e| {
            if let sqlx::Error::Database(ref error) = e
                && error.is_unique_violation()
            {
                return PortError::Conflict { resource: format!("Project {}", project.id()), message: "Project already exists".into() };
            }
            map_sqlite_error("insert_youtube_project", e)
        })?;
    Ok(())
}
