use super::super::{control::WorkControl, error, pcm};
use ports::{PortError, video_export::VideoSourceInput};
use std::{fs, path::Path};

pub(super) fn managed_file(root: &Path, file: &Path, limit: u64) -> Result<u64, PortError> {
    let metadata = fs::symlink_metadata(file).map_err(error::io)?;
    let resolved = fs::canonicalize(file).map_err(error::io)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || !resolved.starts_with(root)
        || resolved == root
        || metadata.len() == 0
        || metadata.len() > limit
    {
        return Err(error::invalid(
            "Video export requires a bounded owned regular file",
        ));
    }
    Ok(metadata.len())
}

pub(super) fn inspect(
    root: &Path,
    source: &VideoSourceInput,
    control: &WorkControl,
) -> Result<String, PortError> {
    if managed_file(root, &source.path, 1_u64 << 32)? != source.size_bytes {
        return Err(error::invalid("Source video size changed"));
    }
    let digest = pcm::hash_file(&source.path, control)?;
    if source
        .expected_sha256
        .as_ref()
        .is_some_and(|expected| expected.as_str() != digest)
    {
        return Err(error::invalid("Reviewed source video hash changed"));
    }
    Ok(digest)
}
