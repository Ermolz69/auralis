use ports::error::PortError;
use uuid::Uuid;

pub(super) const MAX_LINK_REVISION: u64 = 9_007_199_254_740_991;

pub(super) fn uuid(value: Uuid, field: &'static str) -> Result<(), PortError> {
    if value.is_nil() {
        return Err(invalid(field));
    }
    Ok(())
}

pub(super) fn hash(value: &str) -> Result<(), PortError> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value != value.to_ascii_lowercase()
    {
        return Err(invalid("source SHA-256"));
    }
    Ok(())
}

pub(super) fn text(value: &str, field: &'static str) -> Result<(), PortError> {
    if value.trim().is_empty() || value.len() > 64 {
        return Err(invalid(field));
    }
    Ok(())
}

pub(super) fn conflict(message: &'static str) -> PortError {
    PortError::Conflict {
        resource: "TranslationLink".into(),
        message: message.into(),
    }
}

fn invalid(field: &'static str) -> PortError {
    PortError::InvalidSource {
        message: format!("invalid translation {field}"),
    }
}
