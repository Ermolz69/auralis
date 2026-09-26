use application::error::ApplicationError;
use auralis_app::dto::error::CommandError;
use ports::error::PortError;
use std::error::Error;

#[test]
fn translation_cleanup_failure_requires_recovery_without_exposing_runtime_details()
-> Result<(), Box<dyn Error>> {
    let resource = "E:/private/runtime/model.gguf";
    let diagnostic = "private child process and OS cleanup diagnostics";
    let command = CommandError::from(ApplicationError::Port(PortError::CleanupFailed {
        resource: resource.into(),
        message: diagnostic.into(),
    }));
    assert!(matches!(command, CommandError::RecoveryRequired(_)));
    let wire = serde_json::to_value(&command)?;
    assert_eq!(wire["code"], "RECOVERY_REQUIRED");
    assert!(
        wire["message"]
            .as_str()
            .is_some_and(|message| message.contains("restart"))
    );
    let serialized = wire.to_string();
    assert!(!serialized.contains(resource));
    assert!(!serialized.contains(diagnostic));
    let cancelled = CommandError::from(ApplicationError::Port(PortError::Cancelled));
    assert_eq!(serde_json::to_value(cancelled)?["code"], "CANCELLED");
    Ok(())
}
