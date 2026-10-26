use super::*;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, time::UNIX_EPOCH};

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn file_hash(path: &Path) -> TestResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut sha = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        sha.update(&buffer[..read]);
    }
    Ok(format!("{:x}", sha.finalize()))
}

pub fn runtime(value: &Value, managed_root: PathBuf) -> TestResult<TtsRuntimeConfig> {
    let path = |name: &str| -> TestResult<PathBuf> {
        let path = PathBuf::from(value[name].as_str().ok_or("runtime path missing")?);
        if !path.is_absolute() {
            return Err("runtime path must be absolute".into());
        }
        Ok(path)
    };
    if value["version"] != 1 {
        return Err("unsupported input configuration".into());
    }
    Ok(TtsRuntimeConfig {
        execution_budget: Default::default(),
        managed_root,
        python_executable: path("python_executable")?,
        package_root: path("package_root")?,
        voice_store: path("voice_store")?,
        profile_root: path("profile_root")?,
        model_cache: path("model_cache")?,
        voxcpm_revision: value["voxcpm_revision"]
            .as_str()
            .ok_or("revision missing")?
            .into(),
        integrity_sha256: value["integrity_sha256"]
            .as_str()
            .ok_or("inventory missing")?
            .into(),
    })
}

fn walk(
    root: &Path,
    directory: &Path,
    label: &str,
    output: &mut BTreeMap<String, Value>,
) -> TestResult<()> {
    let mut entries: Vec<_> = std::fs::read_dir(directory)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() || !path.canonicalize()?.starts_with(root) {
            return Err("input tree redirects outside its frozen root".into());
        }
        if kind.is_dir() {
            walk(root, &path, label, output)?;
            continue;
        }
        if !kind.is_file() {
            return Err("input is not a regular file".into());
        }
        let metadata = entry.metadata()?;
        let relative = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        let suffix = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let startup_path_config = suffix == "pth"
            && path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("site-packages"));
        let unused_weight = !startup_path_config
            && matches!(
                suffix.as_str(),
                "pt" | "pth" | "safetensors" | "gguf" | "onnx" | "ckpt" | "bin"
            );
        let digest = if unused_weight {
            None
        } else {
            Some(file_hash(&path)?)
        };
        output.insert(format!("{label}/{relative}"), json!({"bytes":metadata.len(),"sha256":digest,
            "modified_ns":u64::try_from(metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos())?,
            "scope":if unused_weight {"unused weight-like asset left unopened"} else {"byte hashed"}}));
    }
    Ok(())
}

pub fn snapshot(
    config: &TtsRuntimeConfig,
    config_file: &Path,
) -> TestResult<BTreeMap<String, Value>> {
    let mut result = BTreeMap::new();
    let runtime = config
        .python_executable
        .parent()
        .and_then(Path::parent)
        .ok_or("runtime root missing")?
        .canonicalize()?;
    let models = config.model_cache.canonicalize()?;
    for (label, root) in [
        ("package", config.package_root.canonicalize()?),
        ("runtime", runtime),
        ("voices", config.voice_store.canonicalize()?),
    ] {
        if models.starts_with(&root) || root.starts_with(&models) {
            return Err("metadata snapshot must not traverse model cache".into());
        }
        walk(&root, &root, label, &mut result)?;
    }
    result.insert(
        "runtime-config".into(),
        json!({"sha256":file_hash(config_file)?}),
    );
    Ok(result)
}
