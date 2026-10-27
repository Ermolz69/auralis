mod copy;
mod files;
mod manifest;
mod manifest_assets;
mod manifest_results;
mod reconcile;
mod tracks;

use async_trait::async_trait;
use domain::library::LibraryState;
use ports::{
    error::PortError,
    media_project::{MediaContractValidator, MediaOperation, MediaProjectStore},
};
use sqlx::SqlitePool;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

pub struct LocalMediaProjects {
    pool: SqlitePool,
    root: PathBuf,
    validator: Arc<dyn MediaContractValidator>,
    gate: Mutex<()>,
    projection_cache: Mutex<std::collections::HashMap<String, String>>,
}

impl LocalMediaProjects {
    pub fn new(
        pool: SqlitePool,
        root: PathBuf,
        validator: Arc<dyn MediaContractValidator>,
    ) -> Self {
        Self {
            pool,
            root,
            validator,
            gate: Mutex::new(()),
            projection_cache: Mutex::new(std::collections::HashMap::new()),
        }
    }

    async fn state(&self) -> Result<LibraryState, PortError> {
        use ports::library::LibraryRepository;
        super::library_repository::SqliteLibraryRepository::new(self.pool.clone())
            .load()
            .await
    }

    async fn idle(&self, id: &str) -> Result<(), PortError> {
        let busy: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE project_id=? AND status IN ('pending','running','cancelling'))")
            .bind(id).fetch_one(&self.pool).await.map_err(db)?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id=?)")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
        if !exists {
            return Err(invalid("Проект больше не существует"));
        }
        if busy {
            return Err(invalid(
                "Структура проекта будет обновлена после завершения его задач",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl MediaProjectStore for LocalMediaProjects {
    async fn bind_run(
        &self,
        project_id: &str,
        asset_id: &str,
        run_id: &str,
    ) -> Result<(), PortError> {
        files::identity(project_id)?;
        files::identity(asset_id)?;
        files::identity(run_id)?;
        let state = self.state().await?;
        if !state
            .assets
            .iter()
            .any(|a| a.id == asset_id && a.owner_project_id == project_id)
        {
            return Err(invalid("Материал не принадлежит проекту запуска"));
        }
        let changed=sqlx::query("INSERT INTO media_run_sources(run_id,asset_id,project_id) SELECT run_id,?,project_id FROM speech_run_intents WHERE run_id=? AND project_id=? ON CONFLICT(run_id) DO NOTHING")
            .bind(asset_id).bind(run_id).bind(project_id).execute(&self.pool).await.map_err(db)?;
        if changed.rows_affected() != 1 {
            return Err(invalid("Не удалось связать запуск с материалом"));
        }
        self.ensure(project_id).await
    }
    async fn ensure(&self, project_id: &str) -> Result<(), PortError> {
        let _guard = self.gate.lock().await;
        files::identity(project_id)?;
        let ready:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_project_receipts WHERE project_id=? AND state='ready')").bind(project_id).fetch_one(&self.pool).await.map_err(db)?;
        if ready {
            let expected: String = sqlx::query_scalar(
                "SELECT manifest_sha256 FROM media_project_receipts WHERE project_id=?",
            )
            .bind(project_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
            if files::digest(&files::safe_path(
                &self.root,
                &format!("{project_id}/auralis-project.json"),
            )?)
            .await?
            .1 != expected
            {
                return Err(invalid("Манифест проекта изменён вне Auralis"));
            }
            self.validator.validate(&self.root.join(project_id)).await?;
        }
        if let Err(error) = self.idle(project_id).await {
            if ready {
                return Ok(());
            }
            return Err(error);
        }
        self.finish_copies().await?;
        let state = self.state().await?;
        if state.links.iter().any(|l| {
            l.project_id == project_id
                && state
                    .assets
                    .iter()
                    .any(|a| a.id == l.asset_id && a.owner_project_id != project_id)
        }) {
            return Err(invalid(
                "Перенос материалов проекта ещё не завершён. Дождитесь копирования или повторите добавление.",
            ));
        }
        self.publish(project_id, &state, true).await?;
        self.validator.validate(&self.root.join(project_id)).await
    }
    async fn reconcile(&self, state: LibraryState) -> Result<LibraryState, PortError> {
        let _guard = self.gate.lock().await;
        self.reconcile_locked(state).await
    }
    async fn attach(
        &self,
        operation_id: &str,
        project_id: &str,
        asset_id: &str,
        duplicate: bool,
    ) -> Result<LibraryState, PortError> {
        let _guard = self.gate.lock().await;
        self.attach_locked(operation_id, project_id, asset_id, duplicate)
            .await
    }
    async fn operation(&self, id: &str) -> Result<Option<MediaOperation>, PortError> {
        use sqlx::Row;
        sqlx::query("SELECT * FROM media_copy_operations WHERE id=?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db)?
            .map(|r| {
                Ok(MediaOperation {
                    id: r.try_get("id").map_err(db)?,
                    project_id: r.try_get("project_id").map_err(db)?,
                    asset_id: serde_json::from_str::<domain::library::LibraryAsset>(
                        &r.try_get::<String, _>("asset_json").map_err(db)?,
                    )
                    .map_err(invalid)?
                    .id,
                    status: r.try_get("state").map_err(db)?,
                    completed_bytes: r.try_get::<i64, _>("completed_bytes").map_err(db)? as u64,
                    total_bytes: r.try_get::<i64, _>("total_bytes").map_err(db)? as u64,
                    error: r.try_get("error").map_err(db)?,
                })
            })
            .transpose()
    }
    async fn cancel(&self, id: &str) -> Result<(), PortError> {
        sqlx::query("UPDATE media_copy_operations SET state='cancelled' WHERE id=? AND state IN ('copying','pending')")
            .bind(id).execute(&self.pool).await.map_err(db)?;
        Ok(())
    }
    async fn structure(&self, project_id: &str) -> Result<serde_json::Value, PortError> {
        self.ensure(project_id).await?;
        let text: String = sqlx::query_scalar(
            "SELECT manifest_json FROM media_project_receipts WHERE project_id=? AND state='ready'",
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;
        serde_json::from_str(&text).map_err(invalid)
    }
    async fn validate(&self, project_id: &str) -> Result<(), PortError> {
        self.ensure(project_id).await
    }
}

fn invalid(error: impl std::fmt::Display) -> PortError {
    PortError::InvalidSource {
        message: error.to_string(),
    }
}
fn db(error: sqlx::Error) -> PortError {
    super::helpers::map_sqlite_error("media_project", error)
}
pub(crate) fn migration_error(error: sqlx::Error) -> PortError {
    db(error)
}

#[cfg(test)]
mod tests;
