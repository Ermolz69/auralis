use auralis_app::bootstrap::caster::verified_manifest;
use sha2::{Digest, Sha256};

#[test]
fn caster_manifest_receipt_must_match_the_selected_installed_runtime()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let scripts = directory.path().join("Scripts");
    std::fs::create_dir(&scripts)?;
    let python = scripts.join("python.exe");
    std::fs::write(&python, b"fixture executable")?;
    let manifest = directory.path().join("caster-wheelhouse-manifest.json");
    std::fs::write(&manifest, b"pinned fixture manifest")?;
    let expected = format!("{:x}", Sha256::digest(b"pinned fixture manifest"));

    assert!(verified_manifest(&python, &expected));
    std::fs::write(&manifest, b"different manifest")?;
    assert!(!verified_manifest(&python, &expected));
    assert!(!verified_manifest(&python, "not-a-sha256"));
    Ok(())
}
