use sqlx::SqlitePool;

pub struct SqliteAudioAssemblyStore {
    pub(super) pool: SqlitePool,
}

impl SqliteAudioAssemblyStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}
