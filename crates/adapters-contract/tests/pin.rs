use adapters_contract::ContractPin;
use std::error::Error;

#[test]
fn contract_pin_rejects_replaced_schema_and_package_version() -> Result<(), Box<dyn Error>> {
    let mut pin = ContractPin::bundled()?;
    let digest = pin
        .schema_sha256
        .remove("voice-snapshot-v1.schema.json")
        .ok_or("bundled voice schema pin missing")?;
    pin.schema_sha256
        .insert("unrelated-v1.schema.json".into(), digest);
    assert!(pin.validate().is_err());

    let mut pin = ContractPin::bundled()?;
    pin.package_version = "0.3.0".into();
    assert!(pin.validate().is_err());
    Ok(())
}
