use domain::project::{Project, ProjectId};
use ports::{error::PortError, puza_contract::PuzaContractSnapshot};
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::{helpers::map_sqlite_error, project_mapper::project_to_row_values};

pub(super) async fn create(
    pool: &SqlitePool,
    project: Project,
    contract: PuzaContractSnapshot,
) -> Result<Project, PortError> {
    let row = project_to_row_values(&project)?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| map_sqlite_error("create_contract_project", e))?;
    sqlx::query("INSERT INTO projects (id,revision,title,status,source_json,metadata_json,source_language,target_language,transcript_json,active_job_id,last_terminal_job_id,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(row.id).bind(row.revision).bind(row.title).bind(row.status)
        .bind(row.source_json).bind(row.metadata_json).bind(row.source_language)
        .bind(row.target_language).bind(row.transcript_json).bind(row.active_job_id)
        .bind(row.last_terminal_job_id).bind(row.created_at).bind(row.updated_at)
        .execute(&mut *tx).await.map_err(|e| map_sqlite_error("create_contract_project", e))?;
    insert(&mut tx, project.id(), &contract).await?;
    tx.commit()
        .await
        .map_err(|e| map_sqlite_error("create_contract_project", e))?;
    Ok(project)
}

pub(crate) async fn insert(
    tx: &mut Transaction<'_, Sqlite>,
    id: &ProjectId,
    contract: &PuzaContractSnapshot,
) -> Result<(), PortError> {
    let json = serde_json::to_string(contract).map_err(|e| invalid(e.to_string()))?;
    sqlx::query("INSERT INTO puza_contract_projects (project_id,snapshot_json,snapshot_sha256) VALUES (?,?,?)")
        .bind(id.to_string()).bind(json).bind(&contract.sha256).execute(&mut **tx)
        .await.map_err(|e| map_sqlite_error("insert_contract_project", e))?;
    Ok(())
}

pub(super) async fn read(
    pool: &SqlitePool,
    id: &ProjectId,
) -> Result<Option<PuzaContractSnapshot>, PortError> {
    let value: Option<(String, String)> = sqlx::query_as(
        "SELECT snapshot_json,snapshot_sha256 FROM puza_contract_projects WHERE project_id=?",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_sqlite_error("read_contract_project", e))?;
    value
        .map(|(json, hash)| {
            let contract: PuzaContractSnapshot =
                serde_json::from_str(&json).map_err(|e| invalid(e.to_string()))?;
            if contract.sha256 != hash {
                return Err(invalid("CONTRACT project receipt changed"));
            }
            Ok(contract)
        })
        .transpose()
}

pub(super) async fn save(
    pool: &SqlitePool,
    id: &ProjectId,
    expected: Option<&str>,
    contract: &PuzaContractSnapshot,
) -> Result<(), PortError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| map_sqlite_error("save_contract_project", e))?;
    let current: Option<String> =
        sqlx::query_scalar("SELECT snapshot_sha256 FROM puza_contract_projects WHERE project_id=?")
            .bind(id.to_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| map_sqlite_error("save_contract_project", e))?;
    if current.as_deref() != expected {
        return Err(PortError::Conflict {
            resource: "CONTRACT project".into(),
            message: "Project voice configuration changed; reload it".into(),
        });
    }
    if current.is_none() {
        insert(&mut tx, id, contract).await?;
    } else {
        let json = serde_json::to_string(contract).map_err(|e| invalid(e.to_string()))?;
        sqlx::query("UPDATE puza_contract_projects SET snapshot_json=?,snapshot_sha256=? WHERE project_id=?")
            .bind(json).bind(&contract.sha256).bind(id.to_string()).execute(&mut *tx)
            .await.map_err(|e| map_sqlite_error("save_contract_project", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| map_sqlite_error("save_contract_project", e))?;
    Ok(())
}

fn invalid(message: impl Into<String>) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
