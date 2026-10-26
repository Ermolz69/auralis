use super::*;
use ports::speech_run::SpeechExecutionBudget;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Observer(AtomicUsize);

struct PendingObserver {
    entered: AtomicUsize,
    cancellation: Option<CancellationToken>,
}

#[async_trait]
impl TtsExecutionObserver for PendingObserver {
    async fn saved_checkpoint(&self, _: &SavedCheckpoint) -> Result<(), PortError> {
        self.entered.fetch_add(1, Ordering::SeqCst);
        if let Some(cancellation) = &self.cancellation {
            cancellation.cancel();
        }
        std::future::pending().await
    }
}

#[async_trait]
impl TtsExecutionObserver for Observer {
    async fn saved_checkpoint(&self, _: &SavedCheckpoint) -> Result<(), PortError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn fixture() -> Result<
    (
        tempfile::TempDir,
        WorkerIdentity,
        TtsExecutionRequest,
        Vec<u8>,
    ),
    PortError,
> {
    let directory = tempfile::tempdir().map_err(io_error)?;
    let project_id = ProjectId::new();
    let identity = WorkerIdentity {
        request_id: "fixture-request".into(),
        project_id: project_id.to_string(),
        run_id: "fixture-run".into(),
        attempt_id: "fixture-attempt".into(),
    };
    let request = TtsExecutionRequest {
        project_id,
        run_id: identity.run_id.clone(),
        attempt_id: identity.attempt_id.clone(),
        plan_digest: "a".repeat(64),
        expected_utterances: 1,
        expected_chunks: 100,
        plan_path: directory.path().join("plan.json"),
        profile_path: directory.path().join("profile.json"),
        out_file: directory.path().join("raw.wav"),
        manifest_path: directory.path().join("manifest.json"),
        cache_dir: directory.path().join("cache"),
        checkpoint_root: directory.path().join("checkpoints"),
        rebuild: false,
    };
    let command = WorkerRequest::encode(&identity, "execute", json!({}))?;
    Ok((directory, identity, request, command))
}

fn child(mode: &str) -> Result<Box<dyn ChildWrapper>, PortError> {
    let mut command = tokio::process::Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("tests/fixture.ps1"),
        ])
        .env("AURALIS_TEST_TTS_MODE", mode)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    owned_process::spawn(command).map_err(io_error)
}

#[tokio::test]
async fn progress_checkpoints_do_not_extend_the_explicit_absolute_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, identity, request, command) = fixture()?;
    let mut child = child("progress")?;
    let observer = Observer(AtomicUsize::new(0));
    let result = timeout(
        Duration::from_secs(6),
        run_worker(
            &mut child,
            &command,
            &identity,
            &request,
            &observer,
            CancellationToken::new(),
            SpeechExecutionBudget::try_from(3)?,
        ),
    )
    .await;
    terminate(&mut child).await?;
    let result = result.map_err(|_| "Worker progress renewed or bypassed its execution budget")?;
    assert!(
        matches!(result, Err(PortError::ExternalToolFailed { message, .. })
        if message.contains("execution deadline exceeded (3 seconds)"))
    );
    assert!(observer.0.load(Ordering::SeqCst) > 0);
    Ok(())
}

#[tokio::test]
async fn a_long_execution_budget_preserves_the_separate_cancellation_protocol()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, identity, request, command) = fixture()?;
    let mut child = child("cancel")?;
    let cancellation = CancellationToken::new();
    let observer = PendingObserver {
        entered: AtomicUsize::new(0),
        cancellation: Some(cancellation.clone()),
    };
    let result = timeout(
        Duration::from_secs(6),
        run_worker(
            &mut child,
            &command,
            &identity,
            &request,
            &observer,
            cancellation,
            SpeechExecutionBudget::try_from(172800)?,
        ),
    )
    .await;
    terminate(&mut child).await?;
    assert_eq!(observer.entered.load(Ordering::SeqCst), 1);
    assert!(matches!(result?, Ok(TtsExecutionOutcome::Cancelled)));
    Ok(())
}

#[tokio::test]
async fn pending_checkpoint_persistence_cannot_outlive_the_execution_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, identity, request, command) = fixture()?;
    let mut child = child("progress")?;
    let observer = PendingObserver {
        entered: AtomicUsize::new(0),
        cancellation: None,
    };
    let result = timeout(
        Duration::from_secs(6),
        run_worker(
            &mut child,
            &command,
            &identity,
            &request,
            &observer,
            CancellationToken::new(),
            SpeechExecutionBudget::try_from(3)?,
        ),
    )
    .await;
    terminate(&mut child).await?;
    assert_eq!(observer.entered.load(Ordering::SeqCst), 1);
    assert!(
        matches!(result?, Err(PortError::ExternalToolFailed { message, .. })
        if message.contains("execution deadline exceeded") && message.contains("persisting checkpoint"))
    );
    Ok(())
}

#[tokio::test]
async fn blocked_request_dispatch_is_part_of_the_execution_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, identity, _request, _) = fixture()?;
    let command =
        WorkerRequest::encode(&identity, "execute", json!({"padding":"x".repeat(900_000)}))?;
    let (mut writer, _unread) = tokio::io::duplex(64);
    let budget = SpeechExecutionBudget::try_from(3)?;
    let result = timeout(
        Duration::from_secs(6),
        worker::dispatch_request(
            &mut writer,
            &command,
            tokio::time::Instant::now() + budget.duration(),
            &CancellationToken::new(),
            budget,
        ),
    )
    .await;
    let result = result?;
    assert!(
        matches!(&result, Err(PortError::ExternalToolFailed { message, .. })
        if message.contains("execution deadline exceeded") && message.contains("dispatching request")),
        "{result:?}"
    );
    Ok(())
}
