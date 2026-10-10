use adapters_contract::{ContractPin, ContractWorkerAdapter, WorkerConfig};
use domain::speech::{SpeechPlan, SynthesisResult};
use ports::speech_contract::SpeechContractPort;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn installed() -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let executable = PathBuf::from(std::env::var("AURALIS_CONTRACT_WORKER")?);
    let fixtures = PathBuf::from(std::env::var("AURALIS_CONTRACT_FIXTURES")?);
    Ok((executable, fixtures))
}

fn adapter(
    executable: PathBuf,
    arguments: Vec<String>,
    cwd: PathBuf,
    exit_timeout: Duration,
) -> Result<ContractWorkerAdapter, Box<dyn Error>> {
    Ok(ContractWorkerAdapter::new(WorkerConfig {
        executable,
        arguments,
        working_directory: cwd,
        pin: ContractPin::bundled()?,
        capability_timeout: Duration::from_secs(5),
        validation_timeout: Duration::from_secs(10),
        exit_timeout,
    })?)
}

fn plan(fixtures: &Path) -> Result<SpeechPlan, Box<dyn Error>> {
    Ok(serde_json::from_slice(&std::fs::read(
        fixtures.join("plan-subtitle-v1.json"),
    )?)?)
}

#[tokio::test]
#[ignore = "requires the explicitly installed model-free CONTRACT wheel"]
async fn installed_worker_matches_cli_on_plan_voice_and_result() -> Result<(), Box<dyn Error>> {
    let (executable, fixtures) = installed()?;
    let worker = adapter(
        executable,
        vec!["worker".into()],
        fixtures.clone(),
        Duration::from_secs(5),
    )?;
    for (plan_name, result_name) in [
        ("plan-subtitle-v1.json", "result-subtitle-v1.json"),
        ("plan-plain-text-v1.json", "result-plain-text-v1.json"),
    ] {
        let valid: SpeechPlan = serde_json::from_slice(&std::fs::read(fixtures.join(plan_name))?)?;
        let accepted = worker.validate_plan(&valid).await?;
        assert!(accepted.valid && accepted.errors.is_empty());
        assert!(worker.validate_voice(&valid.voices[0]).await?.valid);
        let result: SynthesisResult =
            serde_json::from_slice(&std::fs::read(fixtures.join(result_name))?)?;
        assert!(worker.validate_result(&valid, &result).await?.valid);
        let mut wrong = valid;
        wrong.version = 2;
        let rejected = worker.validate_plan(&wrong).await?;
        assert!(!rejected.valid && rejected.errors[0].code == "invalid_schema");
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires the explicitly installed model-free Python runtime"]
async fn synthetic_process_rejects_stale_eof_malformed_and_unknown_completion()
-> Result<(), Box<dyn Error>> {
    let (cli, fixtures) = installed()?;
    let python = cli
        .parent()
        .ok_or("worker has no parent")?
        .join("python.exe");
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/fixture_worker.py");
    let pin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("contract-pin-v1.json");
    for mode in ["stale", "eof", "partial", "malformed", "unknown"] {
        let worker = adapter(
            python.clone(),
            vec![
                "-u".into(),
                script.display().to_string(),
                pin.display().to_string(),
                mode.into(),
            ],
            fixtures.clone(),
            Duration::from_secs(2),
        )?;
        let error = worker
            .validate_plan(&plan(&fixtures)?)
            .await
            .expect_err(mode);
        let message = error.to_string();
        assert!(
            message.contains("worker") || message.contains("Worker"),
            "{mode}: {message}"
        );
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires the explicitly installed model-free Python runtime"]
async fn owned_worker_tree_exits_before_timeout_error_is_returned() -> Result<(), Box<dyn Error>> {
    let (cli, fixtures) = installed()?;
    let python = cli
        .parent()
        .ok_or("worker has no parent")?
        .join("python.exe");
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/fixture_worker.py");
    let pin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("contract-pin-v1.json");
    let temporary = tempfile::tempdir()?;
    let pid_file = temporary.path().join("child.pid");
    let worker = adapter(
        python.clone(),
        vec![
            "-u".into(),
            script.display().to_string(),
            pin.display().to_string(),
            "tree".into(),
            pid_file.display().to_string(),
        ],
        fixtures.clone(),
        Duration::from_millis(500),
    )?;
    assert!(worker.validate_plan(&plan(&fixtures)?).await.is_err());
    let pid = std::fs::read_to_string(pid_file)?.parse::<u32>()?;
    let check = std::process::Command::new(python)
        .args([
            "-c",
            "import psutil,sys; sys.exit(1 if psutil.pid_exists(int(sys.argv[1])) else 0)",
            &pid.to_string(),
        ])
        .status()?;
    assert!(
        check.success(),
        "owned grandchild remained alive after worker cleanup"
    );
    Ok(())
}
