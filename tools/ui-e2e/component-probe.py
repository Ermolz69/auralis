import hashlib
import importlib.metadata
import json
from pathlib import Path
import sys

configuration = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
sys.path.insert(0, configuration["package_root"])
from voxscript_contract.api import load_voice_manifest
from voxscript_contract.speech_plan import speech_plan_digest, validate_speech_plan


def digest(file):
    with Path(file).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


results = []
for voice_id in ("marina_elevenlabs_ru", "alex_analyst_elevenlabs_ru"):
    file = Path(configuration["voice_store"]) / voice_id / "voice.json"
    original = file.read_bytes()
    voice = load_voice_manifest(file)
    for engine, revision in (("voxcpm2", "32279effe8c19989596f05d353d1447f51d9e915"),
                             ("espeech", "f582b6e5897fe8a5835059405a8439d13bdf7684")):
        reference = next(item for item in voice.references
                         if item.engine_id == engine and item.emotion_id == "neutral")
        if reference.qualification.status != "unverified":
            raise ValueError("The technical probe requires the unchanged unverified reference")
        text = "Техническая проверка контракта без синтеза."
        profile = Path(configuration["profile_root"]) / f"native-plan-{engine}.json"
        plan = {
            "version": 1, "plan_id": "component-probe", "project_id": "component-probe",
            "mode": "continuous", "source": {"source_sha256": hashlib.sha256(text.encode()).hexdigest(),
                "text_revision": "probe-1", "reviewed_text": text},
            "review": {"decision_id": "probe-review", "status": "approved", "technical_reference_permission": "technical-component-probe"},
            "profile": {"profile_id": f"native-plan-{engine}", "profile_sha256": digest(profile),
                "engine_id": engine, "model_revision": revision},
            "cast": [{"role_id": "narrator", "voice_id": voice_id, "manifest_sha256": digest(file)}],
            "utterances": [{"utterance_id": "one", "cue_id": None, "text_range": {"start": 0, "end": len(text)},
                "text": text, "role_id": "narrator", "reference_id": reference.reference_id, "emotion_id": "neutral",
                "start_ms": None, "end_ms": None, "speed": 1.0, "volume": 1.0, "seed_key": "probe-1"}],
        }
        plan["plan_digest"] = speech_plan_digest(plan)
        validated = validate_speech_plan(plan, voice_store=Path(configuration["voice_store"]))
        assert len(validated.utterances) == 1
        assert file.read_bytes() == original
        results.append({"voice_id": voice_id, "engine_id": engine, "reference_id": reference.reference_id,
                        "qualification": reference.qualification.status, "plan_digest": validated.plan_digest})

result = {"contract_version": importlib.metadata.version("voxscript-contract"),
          "tts_version": importlib.metadata.version("puza-tts"), "checks": results,
          "models_loaded": False, "voices_modified": False,
          "scope": "Installed component validation only; no application UI, inference or quality acceptance"}
Path(sys.argv[2]).write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
