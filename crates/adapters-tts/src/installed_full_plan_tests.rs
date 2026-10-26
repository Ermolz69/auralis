use crate::{TtsRuntimeConfig, TtsWorkerClient};
use domain::project::ProjectId;
use ports::puza_contract::PuzaContractPort;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use uuid::Uuid;

mod data;
mod guard_control;
mod inputs;
mod lifecycle;
mod proof;

const COUNT: usize = 4004;
type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn guarded_script(root: &Path, body: &str) -> String {
    let guard = root.join("metadata-guard.py");
    if guard.is_file() {
        format!("import runpy; runpy.run_path({});\n{body}", json!(guard))
    } else {
        body.into()
    }
}

pub(crate) fn configure_guard(command: &mut tokio::process::Command, root: &Path) {
    if root.join("metadata-guard.py").is_file() {
        for name in ["TMP", "TEMP", "TMPDIR"] {
            command.env(name, root.join("metadata-tmp"));
        }
    }
}

fn save(root: &Path, name: &str, value: &Value) -> TestResult<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    Ok(())
}

fn ensure(condition: bool, message: &str) -> TestResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

async fn engine_control(client: &TtsWorkerClient, root: &Path, engine: &str) -> TestResult<Value> {
    let project = ProjectId::new();
    let run = Uuid::new_v4();
    let (draft, bindings, source) = data::draft(client, &project, run, engine)?;
    let directory = root.join(engine);
    std::fs::create_dir(&directory)?;
    save(&directory, "frozen-draft.json", &draft)?;
    save(
        &directory,
        "bindings.json",
        &serde_json::to_value(&bindings)?,
    )?;
    let started = Instant::now();
    let contract = client
        .prepare_project(&project, Some(source.clone()), bindings.clone())
        .await?;
    let contract_ms = started.elapsed().as_millis();
    ensure(
        contract.contract_version == "0.2.0",
        "installed CONTRACT version changed",
    )?;
    ensure(
        contract.source_text.as_deref() == Some(source.as_str()),
        "CONTRACT source text changed",
    )?;
    let cast: Value = serde_json::from_str(&contract.cast_json)?;
    ensure(
        cast["roles"]
            .as_object()
            .ok_or("missing CONTRACT roles")?
            .len()
            == COUNT + 1,
        "CONTRACT cast coverage differs",
    )?;
    ensure(
        serde_json::from_str::<Value>(&contract.bindings_json)? == serde_json::to_value(bindings)?,
        "CONTRACT binding identity/order changed",
    )?;
    save(
        &directory,
        "contract.json",
        &serde_json::to_value(contract)?,
    )?;
    let plan_path = directory.join("plan.json");
    let profile = format!("native-plan-{engine}");
    let started = Instant::now();
    let composed = client
        .compose(
            &project,
            &run.to_string(),
            "prepare",
            &directory.join("frozen-draft.json"),
            &plan_path,
            &profile,
        )
        .await?;
    let compose_ms = started.elapsed().as_millis();
    let bytes = std::fs::read(&plan_path)?;
    let mut plan: Value = serde_json::from_slice(&bytes)?;
    let digest = plan
        .as_object_mut()
        .ok_or("plan is not an object")?
        .remove("plan_digest")
        .ok_or("missing digest")?;
    ensure(
        plan == draft,
        "installed compose changed reviewed utterances/cast/order",
    )?;
    ensure(
        digest.as_str() == Some(composed.plan_digest.as_str()),
        "composed digest differs",
    )?;
    ensure(
        composed.utterance_count == COUNT
            && composed.chunk_count == COUNT
            && composed.engine == engine,
        "composed full-plan coverage differs",
    )?;
    let started = Instant::now();
    let validated = client
        .validate_plan(&project, &run.to_string(), "validate", &plan_path, &profile)
        .await?;
    let validate_ms = started.elapsed().as_millis();
    ensure(
        validated.utterance_count == COUNT
            && validated.chunk_count == COUNT
            && validated.engine == engine,
        "validated full-plan coverage differs",
    )?;
    ensure(
        validated.plan_digest == composed.plan_digest,
        "validated plan digest changed",
    )?;
    ensure(
        std::fs::read(&plan_path)? == bytes,
        "validation changed immutable persisted plan bytes",
    )?;
    let result = json!({"engine":engine,"utterances":COUNT,"chunks":COUNT,"bindings":COUNT+1,
        "plan_digest":composed.plan_digest,"plan_sha256":inputs::hash(&bytes),
        "draft_sha256":inputs::file_hash(&directory.join("frozen-draft.json"))?,
        "source_sha256":inputs::hash(source.as_bytes()),"profile_sha256":draft["profile"]["profile_sha256"],
        "technical_reference_permission":draft["review"]["technical_reference_permission"],
        "contract_ms":contract_ms,"compose_ms":compose_ms,"validate_ms":validate_ms,
        "per_operation_deadline_seconds":45,"models_loaded":false,"synthesis_called":false});
    save(&directory, "result.json", &result)?;
    Ok(result)
}

#[tokio::test]
#[ignore = "requires a new metadata case and pinned installed runtime; no synthesis or model access"]
async fn installed_four_thousand_cues_compose_and_validate_without_models() -> TestResult<()> {
    run_case(false).await
}

#[tokio::test]
#[ignore = "requires a new guard-only case and pinned runtime; inert negative controls only"]
async fn installed_guard_only_rejects_threaded_engine_actions() -> TestResult<()> {
    run_case(true).await
}

async fn run_case(guard_only: bool) -> TestResult<()> {
    let case = std::env::var("AURALIS_METADATA_CASE")?;
    assert!(
        !case.is_empty()
            && case.len() <= 64
            && case
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    );
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace missing")?
        .canonicalize()?;
    let cache = workspace.join(".cache").canonicalize()?;
    assert!(cache.starts_with(&workspace));
    let base = cache.join("backend-metadata");
    std::fs::create_dir_all(&base)?;
    assert!(base.canonicalize()?.starts_with(&workspace));
    let root = base.join(&case);
    std::fs::create_dir(&root)?;
    let config_file = PathBuf::from(std::env::var("AURALIS_TEST_PUZA_RUNTIME")?);
    let config_bytes = std::fs::read(&config_file)?;
    let value: Value = serde_json::from_slice(&config_bytes)?;
    let expected_inventory = std::env::var("AURALIS_METADATA_INVENTORY_SHA")?;
    ensure(
        expected_inventory.len() == 64 && expected_inventory.bytes().all(|c| c.is_ascii_hexdigit()),
        "pin an exact metadata inventory hash",
    )?;
    ensure(
        value["integrity_sha256"].as_str() == Some(expected_inventory.as_str()),
        "metadata inventory differs from frozen input",
    )?;
    let managed = root.join("managed");
    for name in ["metadata-audit", "metadata-tmp"] {
        std::fs::create_dir_all(managed.join(name))?;
    }
    let runtime = inputs::runtime(&value, managed.clone())?;
    save(&root, "tts-runtime.json", &value)?;
    let before = inputs::snapshot(&runtime, &config_file)?;
    save(&root, "inputs-before.json", &serde_json::to_value(&before)?)?;
    save(
        &root,
        "frozen-spec.json",
        &json!({"version":1,"case_id":case,"source":if guard_only {"inert_guard_controls"}else{"synthetic4004_metadata_control"},
        "test_executable_sha256":inputs::file_hash(&std::env::current_exe()?)?,
        "fresh_runtime_config_sha256":inputs::file_hash(&root.join("tts-runtime.json"))?,
        "runtime_config_sha256":inputs::hash(&config_bytes),"integrity_sha256":expected_inventory,
        "engines":if guard_only {json!([])}else{json!(["voxcpm2","espeech"])},"utterances":if guard_only {0}else{COUNT},"allow_unverified_neutral":!guard_only,
        "per_operation_deadline_seconds":45,"allow_execute":false,"allow_model_access":false}),
    )?;
    std::fs::write(
        managed.join("metadata-guard.py"),
        include_str!("installed_full_plan_tests/guard.py"),
    )?;
    save(
        &managed,
        "metadata-guard.json",
        &json!({"write_root":root,"model_cache":runtime.model_cache,
        "package_root":runtime.package_root,"audit_root":managed.join("metadata-audit")}),
    )?;
    let client = TtsWorkerClient::new(runtime.clone())?;
    client.verify_integrity(None).await?;
    let lease = common::model_admission::ModelAdmissionLease::acquire_model_free()?;
    let result: TestResult<Value> = async {
        let guard_control = guard_control::run(&runtime, &root).await?;
        let mut engines = Vec::new();
        if !guard_only {
            for engine in ["voxcpm2", "espeech"] {
                engines.push(engine_control(&client, &managed, engine).await?);
            }
        }
        let audits = if guard_only {
            Vec::new()
        } else {
            proof::audit_reports(&managed.join("metadata-audit"))?
        };
        Ok(json!({"engines":engines,"audits":audits,"inert_guard_control":guard_control}))
    }
    .await;
    let admission_retained = lifecycle::uncertain(&result);
    let release = lifecycle::release_candidate(lease, &result)
        .map(common::model_admission::ModelAdmissionLease::release);
    let cleanup_confirmed = release.as_ref().is_some_and(|result| result.is_ok());
    let after = inputs::snapshot(&runtime, &config_file);
    let unchanged = after.as_ref().is_ok_and(|snapshot| snapshot == &before);
    if let Ok(snapshot) = &after {
        lifecycle::preserve(
            save(&root, "inputs-after.json", &serde_json::to_value(snapshot)?),
            admission_retained,
        )?;
    }
    lifecycle::preserve(
        save(
            &root,
            "report.json",
            &json!({"version":1,"case_id":case,"status":if result.is_ok()&&cleanup_confirmed&&unchanged {"passed"}else{"failed"},
        "guard_only":guard_only,"scope":if guard_only {"Inert guard controls only; no full plan/component acceptance"}else{"Installed metadata-only compatibility; worker-persisted plan JSON, no UI/database, inference, audio or language acceptance"},
        "model_execution_authorized":false,"models_loaded":if result.is_ok() {json!(false)} else {Value::Null},
        "reused_input_inventory_unchanged":unchanged,"hashed_input_bytes_unchanged":unchanged,
        "input_scope":"Package/runtime/reference byte hashes; unused weight-like assets metadata only; model cache left untraversed",
        "cleanup_confirmed":cleanup_confirmed,"admission_retained":admission_retained,"result":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string),
        "release_error":release.as_ref().and_then(|result|result.as_ref().err()).map(ToString::to_string)}),
        ),
        admission_retained,
    )?;
    if admission_retained {
        result?;
        return Err(
            lifecycle::unconfirmed("metadata admission retained after unconfirmed exit").into(),
        );
    }
    if let Some(release) = release {
        release?;
    }
    after?;
    assert!(
        unchanged,
        "reused input bytes changed; originals were not restored"
    );
    result?;
    Ok(())
}
