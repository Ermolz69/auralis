use domain::{media::ArtifactId, project::ProjectId};
use ports::PortError;
use sqlx::SqliteConnection;

pub(crate) async fn is_discarded(
    connection: &mut SqliteConnection,
    project: &ProjectId,
    artifact: &ArtifactId,
    key: Option<&str>,
) -> Result<bool, PortError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM speech_discarded_artifacts d JOIN speech_run_intents i ON i.run_id=d.run_id WHERE d.artifact_id=? AND i.project_id=? AND (? IS NULL OR d.final_key=?)) OR EXISTS(SELECT 1 FROM audio_assembly_discarded_artifacts d JOIN audio_assembly_intents i ON i.assembly_id=d.assembly_id WHERE d.artifact_id=? AND i.project_id=? AND (? IS NULL OR d.final_key=?))")
        .bind(artifact.to_string()).bind(project.to_string()).bind(key).bind(key).bind(artifact.to_string()).bind(project.to_string()).bind(key).bind(key)
        .fetch_one(connection).await.map_err(|error| crate::sqlite::helpers::map_sqlite_error("discarded_artifact", error))
}
