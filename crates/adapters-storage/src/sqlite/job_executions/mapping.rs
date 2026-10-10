use domain::job::JobId;
use ports::{PortError, job_execution::ExecutionDescriptor};

pub(super) fn decode(id: &JobId, json: &str) -> Result<ExecutionDescriptor, PortError> {
    let descriptor: ExecutionDescriptor =
        serde_json::from_str(json).map_err(|e| invalid(id, &e.to_string()))?;
    descriptor
        .validate()
        .map_err(|e| invalid(id, &e.to_string()))?;
    Ok(descriptor)
}

fn invalid(id: &JobId, message: &str) -> PortError {
    PortError::InvalidStoredData {
        entity_type: "job_execution".into(),
        entity_id: id.to_string(),
        field: "descriptor_json".into(),
        message: message.into(),
    }
}
