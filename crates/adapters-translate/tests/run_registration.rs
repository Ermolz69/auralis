use adapters_translate::{
    TranslateResultReader, TranslateRunControl, TranslateRunExecutor, TranslateRunRecovery,
    TranslateRunRegistrar,
};
use auralis_translation::{BlockPolicy, RunId, SourceHash, TranslationId};
use auralis_translation_formats::srt::SrtRunPlan;
use auralis_translation_formats::vtt::VttRunPlan;
use auralis_translation_sqlite::{RunStop, SqliteConfig, TranslateDb};
use chrono::Utc;
use domain::job::JobId;
use domain::media::ArtifactId;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::translation_execution::TranslationExecutionPort;
use ports::translation_links::{TranslationLink, TranslationRunIntent};
use ports::translation_result::TranslationResultPort;
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use ports::translation_run_recovery::TranslationRunRecoveryPort;
use ports::translation_run_registration::TranslationRunRegistrationPort;
use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);

fn records(format: &str, source: &[u8]) -> (TranslationLink, TranslationRunIntent) {
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let now = Utc::now();
    let policy = BlockPolicy::default();
    let policy_fingerprint = match format {
        "srt" => SrtRunPlan::policy_fingerprint(policy),
        "vtt" => VttRunPlan::policy_fingerprint(policy),
        _ => unreachable!(),
    };
    (
        TranslationLink {
            translation_id,
            project_id: ProjectId::new(),
            source_artifact_id: ArtifactId::new(),
            source_sha256: SourceHash::digest(source).to_string(),
            source_format: format.into(),
            source_language: "zh".into(),
            target_language: "ru".into(),
            active_run_id: Some(run_id),
            selected_result_id: None,
            output_artifact_id: None,
            revision: 1,
            created_at: now,
            updated_at: now,
        },
        TranslationRunIntent {
            run_id,
            translation_id,
            profile_manifest_json: PROFILE.into(),
            profile_sha256: SourceHash::digest(PROFILE.as_bytes()).to_string(),
            max_target_segments: policy.max_target_segments() as u32,
            context_before_segments: policy.context_before_segments() as u32,
            context_after_segments: policy.context_after_segments() as u32,
            parser_version: 1,
            policy_fingerprint: policy_fingerprint.to_string(),
            created_at: now,
        },
    )
}

#[tokio::test]
async fn registration_is_idempotent_and_preserves_original() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"1\r\n00:00:01,000 --> 00:00:02,000\r\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\r\n";
    let source_path = directory.path().join("source.srt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(&source_path, source)?;
    let (link, intent) = records("srt", source);
    let registrar = TranslateRunRegistrar::new(database_path.clone());
    registrar.ensure_run(&source_path, &link, &intent).await?;
    registrar.ensure_run(&source_path, &link, &intent).await?;

    let database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    let translation_id = TranslationId::new(link.translation_id).ok_or("nil translation ID")?;
    let run_id = RunId::new(intent.run_id).ok_or("nil run ID")?;
    let translation = database.translation(translation_id)?;
    assert_eq!(
        translation.project_id.as_deref(),
        Some(link.project_id.to_string().as_str())
    );
    assert_eq!(
        translation.source_artifact_id.as_deref(),
        Some(link.source_artifact_id.to_string().as_str())
    );
    assert_eq!(database.segments(translation_id)?.len(), 1);
    assert_eq!(database.run(run_id)?.blocks.len(), 1);
    assert_eq!(std::fs::read(&source_path)?, source);
    Ok(())
}

#[tokio::test]
async fn recovery_adapter_only_pauses_its_matching_host_attempt() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
    let source_path = directory.path().join("source.srt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(&source_path, source)?;
    let (link, intent) = records("srt", source);
    TranslateRunRegistrar::new(database_path.clone())
        .ensure_run(&source_path, &link, &intent)
        .await?;
    let run_id = RunId::new(intent.run_id).ok_or("nil run ID")?;
    let owner = JobId::new();
    let mut database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    let run = database.run(run_id)?;
    database.begin_initial_attempt(&run, Some(&owner.to_string()))?;
    drop(database);

    let recovery = TranslateRunRecovery::new(database_path.clone());
    let other = JobId::new();
    assert!(
        recovery
            .recover_for_host(intent.run_id, &other)
            .await
            .is_err()
    );
    let database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    assert_eq!(
        database.run_state(run_id)?,
        auralis_translation::RunState::Running
    );
    drop(database);
    assert!(recovery.recover_for_host(intent.run_id, &owner).await?);
    assert!(!recovery.recover_for_host(intent.run_id, &owner).await?);
    let recovered = recovery.state(intent.run_id).await?;
    assert_eq!(recovered.run_id, intent.run_id);
    assert_eq!(recovered.phase, TranslationRunPhase::Paused);
    assert_eq!((recovered.saved_blocks, recovered.total_blocks), (0, 1));
    let database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    assert_eq!(
        database.run_state(run_id)?,
        auralis_translation::RunState::Paused
    );
    assert_eq!(std::fs::read(source_path)?, source);
    Ok(())
}

#[tokio::test]
async fn registration_rejects_changed_original_before_database_creation()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"WEBVTT\n\n00:01.000 --> 00:02.000\nhello\n";
    let source_path = directory.path().join("source.vtt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(
        &source_path,
        b"WEBVTT\n\n00:01.000 --> 00:02.000\nchanged\n",
    )?;
    let (link, intent) = records("vtt", source);
    let registrar = TranslateRunRegistrar::new(database_path.clone());
    assert!(
        registrar
            .ensure_run(&source_path, &link, &intent)
            .await
            .is_err()
    );
    assert!(!database_path.exists());
    Ok(())
}

#[tokio::test]
async fn webvtt_registration_keeps_optional_cue_identity_and_rejects_changed_profile()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"WEBVTT\n\nNOTE provenance\nsynthetic\n\n00:01.000 --> 00:02.000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
    let source_path = directory.path().join("source.vtt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(&source_path, source)?;
    let (link, intent) = records("vtt", source);
    let registrar = TranslateRunRegistrar::new(database_path.clone());
    registrar.ensure_run(&source_path, &link, &intent).await?;
    let database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    let translation_id = TranslationId::new(link.translation_id).ok_or("nil ID")?;
    let segments = database.segments(translation_id)?;
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].cue_label, None);
    drop(database);

    let mut changed = intent.clone();
    changed.profile_manifest_json.push(' ');
    assert!(
        registrar
            .ensure_run(&source_path, &link, &changed)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&source_path)?, source);
    Ok(())
}

#[tokio::test]
async fn local_model_run_commits_a_result_and_reuses_it_without_another_server_call()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
    let source_path = directory.path().join("source.srt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(&source_path, source)?;
    let (link, intent) = records("srt", source);
    TranslateRunRegistrar::new(database_path.clone())
        .ensure_run(&source_path, &link, &intent)
        .await?;
    let control = TranslateRunControl::new(database_path.clone());
    let requested = control.snapshot(&link).await?;
    assert_eq!(requested.phase, TranslationRunPhase::Requested);
    assert_eq!((requested.saved_blocks, requested.total_blocks), (0, 1));
    control.request_pause(&link).await?;
    assert_eq!(
        control.snapshot(&link).await?.phase,
        TranslationRunPhase::Paused
    );
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}/", listener.local_addr()?);
    let server = std::thread::spawn(move || respond_once(listener));
    let executor = TranslateRunExecutor::experimental_unchecked(database_path.clone());
    let job_id = JobId::new();
    let (progress_sender, mut progress_receiver) = tokio::sync::mpsc::unbounded_channel();
    let result_id = executor
        .execute_for_host(
            &source_path,
            &link,
            &intent,
            &endpoint,
            ports::host_translation_execution::HostTranslationExecution {
                job_id: job_id.clone(),
                start_guard: None,
                progress: progress_sender,
            },
        )
        .await?;
    let reported: Vec<_> = std::iter::from_fn(|| progress_receiver.try_recv().ok())
        .map(|progress| {
            (
                progress.run_id,
                progress.saved_blocks,
                progress.total_blocks,
            )
        })
        .collect();
    assert_eq!(reported, vec![(intent.run_id, 0, 1), (intent.run_id, 1, 1)]);
    server
        .join()
        .map_err(|_| "mock server panicked")?
        .map_err(|error| error.to_string())?;
    let database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    let run_id = RunId::new(intent.run_id).ok_or("nil run ID")?;
    assert_eq!(database.checkpoints(run_id)?.len(), 1);
    assert_eq!(database.result_for_run(run_id)?.result_id.get(), result_id);
    drop(database);
    let stored_host: Option<String> = rusqlite::Connection::open(&database_path)?.query_row(
        "SELECT host_job_id FROM run_attempts WHERE run_id = ?1",
        [intent.run_id.to_string()],
        |row| row.get(0),
    )?;
    assert_eq!(stored_host.as_deref(), Some(job_id.to_string().as_str()));
    let completed = control.snapshot(&link).await?;
    assert_eq!(completed.phase, TranslationRunPhase::Validated);
    assert_eq!((completed.saved_blocks, completed.total_blocks), (1, 1));
    assert_eq!(completed.result_id, Some(result_id));
    assert_eq!(completed.needs_review, Some(true));
    assert!(control.request_pause(&link).await.is_err());
    let rebuilt = TranslateResultReader::new(database_path.clone())
        .verified_result(&source_path, &link, intent.run_id, result_id)
        .await?;
    assert!(String::from_utf8(rebuilt.output_bytes)?.contains("Привет."));
    assert_eq!(
        executor
            .execute(&source_path, &link, &intent, "http://127.0.0.1:1/")
            .await?,
        result_id
    );
    assert!(
        TranslateRunExecutor::new(database_path)
            .execute(&source_path, &link, &intent, "http://127.0.0.1:1/")
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&source_path)?, source);
    Ok(())
}

#[tokio::test]
async fn running_pause_request_is_visible_without_changing_another_project()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"1\n00:00:01,000 --> 00:00:02,000\nhello\n";
    let source_path = directory.path().join("source.srt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(&source_path, source)?;
    let (link, intent) = records("srt", source);
    TranslateRunRegistrar::new(database_path.clone())
        .ensure_run(&source_path, &link, &intent)
        .await?;
    let run_id = RunId::new(intent.run_id).ok_or("nil run ID")?;
    let mut database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    let run = database.run(run_id)?;
    let attempt = database.begin_initial_attempt(&run, None)?;
    let control = TranslateRunControl::new(database_path);
    let mut foreign = link.clone();
    foreign.project_id = ProjectId::new();
    assert!(control.request_pause(&foreign).await.is_err());
    assert!(!control.snapshot(&link).await?.pause_requested);
    control.request_pause(&link).await?;
    let running = control.snapshot(&link).await?;
    assert_eq!(running.phase, TranslationRunPhase::Running);
    assert!(running.pause_requested);
    database.stop_attempt(run_id, attempt, RunStop::Paused, "pause acknowledged")?;
    let paused = control.snapshot(&link).await?;
    assert_eq!(paused.phase, TranslationRunPhase::Paused);
    assert!(!paused.pause_requested);
    assert_eq!(std::fs::read(source_path)?, source);
    Ok(())
}

#[tokio::test]
async fn executor_acknowledges_pause_without_waiting_for_a_model_response()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"1\n00:00:01,000 --> 00:00:02,000\nhello\n";
    let source_path = directory.path().join("source.srt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(&source_path, source)?;
    let (link, intent) = records("srt", source);
    TranslateRunRegistrar::new(database_path.clone())
        .ensure_run(&source_path, &link, &intent)
        .await?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}/", listener.local_addr()?);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let server = std::thread::spawn(move || -> Result<(), String> {
        listener
            .set_nonblocking(true)
            .map_err(|cause| cause.to_string())?;
        let started = Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(cause) if cause.kind() == std::io::ErrorKind::WouldBlock => {
                    if started.elapsed() > Duration::from_secs(5) {
                        return Err("request timeout".into());
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(cause) => return Err(cause.to_string()),
            }
        };
        stream
            .set_nonblocking(false)
            .map_err(|cause| cause.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|cause| cause.to_string())?;
        read_request(&mut stream).map_err(|cause| cause.to_string())?;
        ready_tx.send(()).map_err(|_| "receiver closed")?;
        let mut byte = [0];
        match stream.read(&mut byte) {
            Ok(0) => Ok(()),
            Err(cause) if cause.kind() == std::io::ErrorKind::ConnectionReset => Ok(()),
            other => Err(format!(
                "paused model request remained connected: {other:?}"
            )),
        }
    });
    let job_id = JobId::new();
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let executor = TranslateRunExecutor::experimental_unchecked(database_path.clone());
    let (work_source, work_link, work_intent, work_job) = (
        source_path.clone(),
        link.clone(),
        intent.clone(),
        job_id.clone(),
    );
    let work = tokio::spawn(async move {
        executor
            .execute_for_host(
                &work_source,
                &work_link,
                &work_intent,
                &endpoint,
                ports::host_translation_execution::HostTranslationExecution {
                    job_id: work_job,
                    start_guard: None,
                    progress: progress_tx,
                },
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), ready_rx).await??;
    TranslateRunControl::new(database_path.clone())
        .request_pause(&link)
        .await?;
    let result = tokio::time::timeout(Duration::from_secs(3), work).await??;
    assert!(matches!(result, Err(PortError::Cancelled)));
    server.join().map_err(|_| "server panicked")??;
    let database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    let run_id = RunId::new(intent.run_id).ok_or("nil run ID")?;
    assert_eq!(
        database.run_state(run_id)?,
        auralis_translation::RunState::Paused
    );
    assert!(!database.pause_requested(run_id)?);
    assert!(database.checkpoints(run_id)?.is_empty());
    assert!(database.result_for_run(run_id).is_err());
    let saved: Vec<_> = std::iter::from_fn(|| progress_rx.try_recv().ok())
        .map(|progress| progress.saved_blocks)
        .collect();
    assert_eq!(saved, [0]);
    let attempt: (Option<String>, Option<i64>, Option<String>) =
        rusqlite::Connection::open(&database_path)?.query_row(
            "SELECT host_job_id, ended_at, stop_reason FROM run_attempts WHERE run_id = ?1",
            [intent.run_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    assert_eq!(attempt.0, Some(job_id.to_string()));
    assert!(attempt.1.is_some());
    assert_eq!(attempt.2.as_deref(), Some("pause requested"));
    assert_eq!(link.active_run_id, Some(intent.run_id));
    assert_eq!(std::fs::read(source_path)?, source);
    Ok(())
}

fn respond_once(listener: TcpListener) -> Result<(), Box<dyn Error + Send + Sync>> {
    listener.set_nonblocking(true)?;
    let started = Instant::now();
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if started.elapsed() > Duration::from_secs(10) {
                    return Err("timed out waiting for model request".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    };
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    read_request(&mut stream)?;
    let body = r#"{"choices":[{"message":{"content":"Привет."},"finish_reason":"stop"}]}"#;
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}

fn read_request(stream: &mut TcpStream) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut raw = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Err("model request ended early".into());
        }
        raw.extend_from_slice(&buffer[..count]);
        if let Some(header_end) = raw.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&raw[..header_end])?;
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .ok_or("missing model request length")?;
            if raw.len() >= header_end + 4 + length {
                return Ok(());
            }
        }
    }
}
