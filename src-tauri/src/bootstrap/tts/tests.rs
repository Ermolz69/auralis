use super::InstalledTtsRuntime;
use serde_json::{Value, json};

fn configuration() -> Value {
    json!({"version":1,"python_executable":"fixture/python.exe", "package_root":"fixture/packages",
        "voice_store":"fixture/voices", "profile_root":"fixture/profiles", "model_cache":"fixture/models",
        "voxcpm_revision":"32279effe8c19989596f05d353d1447f51d9e915", "integrity_sha256":"a".repeat(64)})
}

#[test]
fn old_runtime_configuration_keeps_its_default_and_explicit_budgets_are_bounded()
-> Result<(), Box<dyn std::error::Error>> {
    let original = configuration();
    let legacy: InstalledTtsRuntime = serde_json::from_value(original.clone())?;
    assert_eq!(legacy.execution_budget_seconds.seconds(), 2700);
    let mut value = original.clone();
    value["execution_budget_seconds"] = json!(172800);
    let full: InstalledTtsRuntime = serde_json::from_value(value)?;
    assert_eq!(full.execution_budget_seconds.seconds(), 172800);
    for budget in [
        json!(0),
        json!(172801),
        json!(-1),
        json!(1.5),
        json!("172800"),
        Value::Null,
    ] {
        let mut value = original.clone();
        value["execution_budget_seconds"] = budget;
        assert!(serde_json::from_value::<InstalledTtsRuntime>(value).is_err());
    }
    Ok(())
}
