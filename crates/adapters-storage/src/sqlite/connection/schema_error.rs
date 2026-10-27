use ports::error::PortError;

pub(super) fn non_final_schema_error(component: &str) -> PortError {
    PortError::Storage {
        operation: "validate_sqlite_schema",
        message: format!("The {component} does not use the current Auralis schema"),
    }
}
