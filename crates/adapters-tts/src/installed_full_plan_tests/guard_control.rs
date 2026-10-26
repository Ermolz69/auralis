use super::*;
use tokio::io::AsyncReadExt;

pub async fn run(runtime: &TtsRuntimeConfig, root: &Path) -> TestResult<Value> {
    let guard_root = root.join("guard-control");
    for name in ["metadata-audit", "metadata-tmp"] {
        std::fs::create_dir_all(guard_root.join(name))?;
    }
    std::fs::write(
        guard_root.join("metadata-guard.py"),
        include_str!("guard.py"),
    )?;
    save(
        &guard_root,
        "metadata-guard.json",
        &json!({"write_root":guard_root,"model_cache":runtime.model_cache,
        "package_root":runtime.package_root,"audit_root":guard_root.join("metadata-audit")}),
    )?;
    let outside = root.join("guard-control-must-not-exist");
    let mut command = tokio::process::Command::new(&runtime.python_executable);
    command.args(["-I","-B","-c"])
        .arg(guarded_script(&guard_root,include_str!("guard_check.py")))
        .arg(&runtime.package_root).arg(json!({"root":guard_root,"model_cache":runtime.model_cache,"outside_owned_guard":outside}).to_string())
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped());
    configure_guard(&mut command, &guard_root);
    let mut child = crate::owned_process::spawn(command)?;
    let result = tokio::time::timeout(std::time::Duration::from_secs(45), async {
        let mut stderr = Vec::new();
        child
            .stderr()
            .take()
            .ok_or("guard-control stderr missing")?
            .take(8193)
            .read_to_end(&mut stderr)
            .await?;
        Ok::<_, Box<dyn std::error::Error>>((child.wait().await?, stderr))
    })
    .await;
    crate::owned_process::terminate(&mut child, "metadata guard control").await?;
    let (status, errors) = result??;
    if !status.success() {
        return Err(format!(
            "metadata guard self-control failed: {}",
            String::from_utf8_lossy(&errors)
        )
        .into());
    }
    if outside.exists() {
        return Err("guard allowed its inert outside-output write".into());
    }
    let entries: Vec<_> = std::fs::read_dir(guard_root.join("metadata-audit"))
        .map_err(|error| lifecycle::unconfirmed(error.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|error| lifecycle::unconfirmed(error.to_string()))?;
    if entries.len() != 1 {
        return Err(lifecycle::unconfirmed("guard self-control audit missing").into());
    }
    let report: Value = serde_json::from_slice(
        &std::fs::read(entries[0].path())
            .map_err(|error| lifecycle::unconfirmed(error.to_string()))?,
    )
    .map_err(|error| lifecycle::unconfirmed(error.to_string()))?;
    proof::confirm_report(&report)?;
    if report["violations"]
        .as_array()
        .ok_or("self-control denials missing")?
        .len()
        != 8
        || report["engine_calls"] != 4
        || report["model_file_opens"] != 2
        || !report["framework_modules_loaded"]
            .as_array()
            .ok_or("self-control framework audit missing")?
            .is_empty()
    {
        return Err("guard self-control did not reject every inert action".into());
    }
    ensure(
        report["profile_active_before_exit"] == true
            && report["thread_profile_active_before_exit"] == true,
        "guard profile was not active on main/new threads",
    )?;
    Ok(report)
}
