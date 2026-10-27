use async_trait::async_trait;
use domain::library::LibraryState;
use ports::{error::PortError, library::LibraryRepository};
use sqlx::{Row, SqlitePool};

pub struct SqliteLibraryRepository {
    pool: SqlitePool,
}
impl SqliteLibraryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}
fn error(e: sqlx::Error) -> PortError {
    super::helpers::map_sqlite_error("workspace_library", e)
}
fn invalid(e: impl std::fmt::Display) -> PortError {
    PortError::InvalidSource {
        message: e.to_string(),
    }
}

#[async_trait]
impl LibraryRepository for SqliteLibraryRepository {
    async fn load(&self) -> Result<LibraryState, PortError> {
        let row = sqlx::query("SELECT revision, payload FROM workspace_library WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(error)?;
        let Some(row) = row else {
            return Ok(LibraryState::default());
        };
        let mut state: LibraryState =
            serde_json::from_str(row.try_get::<&str, _>("payload").map_err(error)?)
                .map_err(invalid)?;
        state.revision = row.try_get::<i64, _>("revision").map_err(error)? as u64;
        state.validate().map_err(invalid)?;
        Ok(state)
    }
    async fn save(&self, mut state: LibraryState) -> Result<LibraryState, PortError> {
        state.validate().map_err(invalid)?;
        let expected = state.revision;
        state.revision = expected
            .checked_add(1)
            .filter(|v| *v <= i64::MAX as u64)
            .ok_or_else(|| invalid("Library revision overflow"))?;
        let payload = serde_json::to_string(&state).map_err(invalid)?;
        if payload.len() > 16_000_000 {
            return Err(invalid("Library catalog is too large"));
        }
        let result = if expected == 0 {
            sqlx::query("INSERT INTO workspace_library(id, revision, payload) VALUES(1, 1, ?) ON CONFLICT(id) DO NOTHING").bind(payload).execute(&self.pool).await
        } else {
            sqlx::query("UPDATE workspace_library SET revision = ?, payload = ? WHERE id = 1 AND revision = ?").bind(state.revision as i64).bind(payload).bind(expected as i64).execute(&self.pool).await
        }.map_err(error)?;
        if result.rows_affected() != 1 {
            return Err(PortError::Conflict {
                resource: "Library".into(),
                message: "Library changed; reload before retrying".into(),
            });
        }
        Ok(state)
    }
}
