use super::TranslationPackageService;
use super::catalog;
use super::selection::PackageSelection;

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
fn selection_marker_accepts_only_the_pinned_release() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let marker_path = directory.path().join("translation-package-selection.json");
    PackageSelection::save(&marker_path)?;
    let marker = PackageSelection::load(&marker_path)?
        .ok_or_else(|| std::io::Error::other("missing saved selection"))?;
    assert!(marker.matches_catalog());

    let release = catalog::release()?;
    std::fs::write(
        &marker_path,
        format!(
            "{{\"schema_version\":1,\"release_id\":\"{}-other\",\"backend\":\"cpu\"}}",
            release.id
        ),
    )?;
    assert!(PackageSelection::load(&marker_path).is_err());
    PackageSelection::save(&marker_path)?;
    assert!(PackageSelection::load(&marker_path).is_ok());
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
    service.select(&installed.release_id, &installed.backend)?;
    let reopened = TranslationPackageService::new(packages, cache, marker);
    let status = reopened.status()?;
    assert!(status.installed);
    assert!(status.selected);
    assert!(reopened.selected_runtime()?.is_some());
    Ok(())
}
