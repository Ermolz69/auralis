use auralis_translation_llamacpp::ReleaseManifest;

pub(super) const RELEASE: &[u8] = include_bytes!(
    "../../../../modules/auralis-translate/models/releases/hy_mt2_1_8b_q4_k_m.windows_x64_cpu.experimental.json"
);
pub(super) const PROFILE: &[u8] = include_bytes!(
    "../../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
);
pub(super) const BACKEND: &str = "cpu";

pub(super) fn release() -> Result<ReleaseManifest, super::TranslationPackageError> {
    Ok(ReleaseManifest::from_json(RELEASE, PROFILE)?)
}
