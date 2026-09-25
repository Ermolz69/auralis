#[derive(Debug, thiserror::Error)]
pub enum TranslationPackageError {
    #[error("the selected model package is invalid: {0}")]
    Invalid(&'static str),
    #[error("another model installation is running")]
    Busy,
    #[error(transparent)]
    Manifest(#[from] auralis_translation_llamacpp::ReleaseManifestError),
    #[error(transparent)]
    Download(#[from] auralis_translation_llamacpp::AssetDownloadError),
    #[error(transparent)]
    Install(#[from] auralis_translation_llamacpp::OfflineInstallError),
    #[error(transparent)]
    Runtime(#[from] ports::error::PortError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
