use adapters_translate::TranslationPackageService;
use auralis_translation::SourceHash;
use auralis_translation_llamacpp::ReleaseManifest;
use ports::error::PortError;
use ports::translation_links::TranslationRunIntent;
use std::io::Write;

#[test]
fn missing_package_is_not_selectable() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let service = TranslationPackageService::new(
        directory.path().join("translation-packages"),
        directory.path().join("cache"),
        directory.path().join("translation-package-selection.json"),
    );
    let status = service.status()?;
    assert!(!status.installed);
    assert!(!status.selected);
    assert!(service.select(&status.release_id, &status.backend).is_err());
    assert!(
        !directory
            .path()
            .join("translation-package-selection.json")
            .exists()
    );
    Ok(())
}

#[test]
fn package_storage_never_replaces_an_unexpected_file() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let packages = directory.path().join("translation-packages");
    std::fs::write(&packages, b"user data")?;
    let service = TranslationPackageService::new(
        packages.clone(),
        directory.path().join("cache"),
        directory.path().join("translation-package-selection.json"),
    );
    assert!(service.install().is_err());
    assert_eq!(std::fs::read(packages)?, b"user data");
    Ok(())
}

#[test]
fn selection_marker_with_unknown_release_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let marker_path = directory.path().join("translation-package-selection.json");
    let service = TranslationPackageService::new(
        directory.path().join("translation-packages"),
        directory.path().join("cache"),
        marker_path.clone(),
    );
    let release_id = service.status()?.release_id;
    std::fs::write(
        &marker_path,
        format!(
            "{{\"schema_version\":1,\"release_id\":\"{}-other\",\"backend\":\"cpu\"}}",
            release_id
        ),
    )?;
    assert!(service.selected_runtime().is_err());
    assert!(!service.status()?.selected);
    Ok(())
}

#[test]
#[ignore = "requires a complete pinned upstream asset cache"]
fn installs_and_selects_a_real_cached_package() -> Result<(), Box<dyn std::error::Error>> {
    let cache = std::env::var_os("AURALIS_TEST_PACKAGE_ASSET_DIR")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| std::io::Error::other("AURALIS_TEST_PACKAGE_ASSET_DIR is missing"))?;
    let directory = tempfile::tempdir()?;
    let packages = directory.path().join("translation-packages");
    let marker = directory.path().join("translation-package-selection.json");
    let service = TranslationPackageService::new(packages.clone(), cache.clone(), marker.clone());
    let installed = service.install()?;
    assert!(installed.installed);
    assert!(!installed.selected);
    let selected_runtime = service.select(&installed.release_id, &installed.backend)?;
    let release = ReleaseManifest::from_json(
        include_bytes!(
            "../../../modules/auralis-translate/models/releases/hy_mt2_1_8b_q4_k_m.windows_x64_cpu.experimental.json"
        ),
        include_bytes!(
            "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
        ),
    )?;
    let release_root = packages.join(&release.id);
    let final_root = release_root.join("cpu");
    let mut runtime_file = std::fs::OpenOptions::new()
        .write(true)
        .open(final_root.join("runtime/llama-server.exe"))?;
    runtime_file.write_all(&[0])?;
    runtime_file.sync_all()?;
    drop(runtime_file);
    let profile = include_str!(
        "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
    );
    let intent = TranslationRunIntent {
        run_id: uuid::Uuid::new_v4(),
        translation_id: uuid::Uuid::new_v4(),
        profile_manifest_json: profile.into(),
        profile_sha256: SourceHash::digest(profile.as_bytes()).to_string(),
        max_target_segments: 8,
        context_before_segments: 0,
        context_after_segments: 0,
        parser_version: 1,
        policy_fingerprint: "package-integrity-probe".into(),
        created_at: chrono::Utc::now(),
    };
    let admission = tokio::runtime::Builder::new_current_thread().build()?;
    assert!(matches!(
        admission.block_on(selected_runtime.acquire(&intent)),
        Err(PortError::Conflict { .. })
    ));
    assert!(
        service
            .select(&installed.release_id, &installed.backend)
            .is_err()
    );
    let model = final_root
        .join("model")
        .join(&release.model_asset().filename);
    let mut file = std::fs::OpenOptions::new().write(true).open(model)?;
    file.write_all(&[0])?;
    file.sync_all()?;
    drop(file);
    assert!(
        service
            .select(&installed.release_id, &installed.backend)
            .is_err()
    );
    let previous = release_root.join(".previous-cpu");
    std::fs::rename(&final_root, &previous)?;
    let repaired = service.install()?;
    assert!(repaired.installed);
    assert!(repaired.selected);
    assert!(!previous.exists());
    let reopened = TranslationPackageService::new(packages, cache, marker);
    let status = reopened.status()?;
    assert!(status.installed);
    assert!(status.selected);
    assert!(reopened.selected_runtime()?.is_some());
    Ok(())
}
