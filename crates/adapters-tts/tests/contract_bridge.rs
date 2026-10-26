use ports::speech_run::MAX_SPEECH_UTTERANCES;
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

// Exercises the shipped Python bridge with fixture CONTRACT metadata and API.
// It never imports TTS, accesses installed voices or loads a model.
const API: &str = r#"
import json
from types import SimpleNamespace as NS
seen = set()
def load_voice_manifest(path):
    if path in seen:
        raise ValueError('Voice manifests must be cached per request')
    seen.add(path)
    value = json.loads(path.read_text(encoding='utf-8'))
    return NS(voice_id=value['voice_id'], references=[NS(**{**ref,
        'qualification':NS(status=ref['qualification'])}) for ref in value['references']])
def load_project(root):
    return NS(documents=json.loads((root/'project.json').read_text())['documents'])
"#;
const QUALIFICATION: &str = r#"
def require_qualified_reference(voice, reference_id, engine_id, emotion_id):
    reference = next(ref for ref in voice.references if ref.reference_id == reference_id
        and ref.engine_id == engine_id and ref.emotion_id == emotion_id)
    if reference.qualification.status != 'qualified':
        raise ValueError('Human qualification required')
"#;

fn probe(
    count: usize,
    emotion: &str,
    status: &str,
    permission: bool,
    engine: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let packages = root.path().join("packages");
    let contract = packages.join("voxscript_contract");
    let metadata = packages.join("voxscript_contract-0.2.0.dist-info");
    let voice = root.path().join("voices/fixture");
    for folder in [&contract, &metadata, &voice] {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(contract.join("__init__.py"), "")?;
    std::fs::write(contract.join("api.py"), API)?;
    std::fs::write(contract.join("voice_qualification.py"), QUALIFICATION)?;
    std::fs::write(
        contract.join("json_codec.py"),
        "import json\ndef canonical_json_v1(value):\n    return json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode('utf-8')\n",
    )?;
    std::fs::write(
        metadata.join("METADATA"),
        "Name: voxscript-contract\nVersion: 0.2.0\n",
    )?;
    std::fs::write(
        voice.join("voice.json"),
        serde_json::to_vec(&json!({
            "voice_id":"fixture", "references":[{"reference_id":"selected", "engine_id":engine,
                "emotion_id":emotion,"qualification":status}]
        }))?,
    )?;
    let bindings: Vec<_> = (0..count)
        .map(|index| {
            json!({
                "role_id":format!("role-{index}"),"display_name":"Fixture",
                "voice_id":"fixture","reference_id":"selected","engine_id":engine,
                "technical_reference_permission":permission.then_some("explicit-plan-permission")
            })
        })
        .collect();
    let request = json!({"request_id":"fixture-request", "project_id":"fixture-project",
        "source_text":"Reviewed fixture", "bindings":bindings,
        "max_bindings":MAX_SPEECH_UTTERANCES + 1});
    let mut child = Command::new(
        std::env::var_os("AURALIS_CONTRACT_TEST_PYTHON")
            .ok_or("Set AURALIS_CONTRACT_TEST_PYTHON using task rs:test:contract-bridge")?,
    )
    .args([
        "-I",
        "-B",
        "-X",
        "utf8",
        "-c",
        include_str!("../src/contract_worker.py"),
    ])
    .arg(packages)
    .arg(root.path().join("voices"))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing worker input")?
        .write_all(&serde_json::to_vec(&request)?)?;
    let output = child.wait_with_output()?;
    if output.status.success() {
        let result: Value = serde_json::from_slice(&output.stdout)?;
        let bindings: Vec<Value> = serde_json::from_str(
            result["result"]["bindings_json"]
                .as_str()
                .ok_or("missing bindings")?,
        )?;
        assert_eq!(bindings.len(), count);
        assert_eq!(result["request_id"], request["request_id"]);
    }
    Ok(output.status.success())
}

#[test]
#[ignore = "requires an explicitly selected Python interpreter; fixture API only, no models"]
fn long_cast_and_reference_qualification_preserve_the_existing_permission_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    for engine in ["voxcpm2", "espeech"] {
        assert!(probe(4005, "neutral", "unverified", true, engine)?);
        assert!(probe(1, "joy", "qualified", false, engine)?);
        assert!(!probe(1, "joy", "unverified", true, engine)?);
        assert!(!probe(1, "neutral", "unverified", false, engine)?);
        assert!(!probe(1, "neutral", "rejected", true, engine)?);
    }
    assert!(!probe(
        MAX_SPEECH_UTTERANCES as usize + 2,
        "neutral",
        "qualified",
        false,
        "voxcpm2"
    )?);
    Ok(())
}
