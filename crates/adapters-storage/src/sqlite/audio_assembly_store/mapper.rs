use super::{
    error::{invalid, uuid},
    row::AssemblyIntentRow,
    validation,
};
use domain::{project::ProjectId, speech::Sha256Digest};
use ports::{PortError, audio_jobs::AudioAssemblyIntent};
use std::str::FromStr;

pub(crate) fn intent(row: AssemblyIntentRow) -> Result<AudioAssemblyIntent, PortError> {
    let intent = AudioAssemblyIntent {
        assembly_id: uuid(&row.assembly_id)?,
        project_id: ProjectId::from_str(&row.project_id)
            .map_err(|_| invalid("Stored assembly project is invalid"))?,
        source_run_id: uuid(&row.source_run_id)?,
        source_attempt_id: uuid(&row.source_attempt_id)?,
        plan_json: row.plan_json,
        plan_sha256: Sha256Digest::parse(row.plan_sha256).map_err(invalid)?,
    };
    validation::intent(&intent)?;
    Ok(intent)
}
