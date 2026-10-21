"""Real, technical renderer recovery probe from the installed standalone bundle.

These samples use an unverified reference for engineering checks only. They do
not qualify the voice or bypass the production speech-plan validation gate.
"""

from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import sys
import time


TEXTS = (
    "Добрый вечер. Мы проверяем ясность русской речи.",
    "Сегодня работа сохраняет каждую фразу.",
)
CHANGED_TEXT = "Доброе утро. Мы проверяем новую русскую реплику."
CANCEL_TEXTS = (
    "Первый сохранённый фрагмент переживёт остановку.",
    "Вторая фраза будет создана после возобновления.",
)
PHASES = {"baseline", "repeat", "corrupt", "changed", "cancel", "resume"}


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def text_sha(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def load_bundle():
    if os.environ.get("PUZA_PROBE_ADMITTED") != "1":
        raise RuntimeError("Shared model admission is required")
    data_root = Path(os.environ["AURALIS_TTS_BUNDLE_DATA_ROOT"]).resolve(strict=True)
    config_path = data_root / "tts-runtime.json"
    config = json.loads(config_path.read_text(encoding="utf-8"))
    if Path(config["python_executable"]).resolve() != Path(sys.executable).resolve():
        raise ValueError("The installed bundle Python is not executing")
    if Path(sys.base_prefix).resolve() != Path(sys.executable).parent.resolve():
        raise ValueError("The bundle is using an external Python base")
    for key in ("package_root", "voice_store", "profile_root", "model_cache"):
        if not Path(config[key]).resolve().is_relative_to(data_root):
            raise ValueError(f"Installed runtime {key} escaped its data root")
    os.environ.update(
        HF_HOME=config["model_cache"],
        HF_HUB_CACHE=str(Path(config["model_cache"]) / "hub"),
        HF_HUB_OFFLINE="1",
        TTS_REQUIRE_PRELOADED_MODELS="1",
        TTS_VOXCPM_REVISION=config["voxcpm_revision"],
    )
    sys.path.insert(0, config["package_root"])
    return data_root, config_path, config


def main() -> None:
    case_id = sys.argv[sys.argv.index("--case-id") + 1]
    engine_id, phase = case_id.rsplit("-", 1)
    if engine_id not in {"voxcpm2", "espeech"} or phase not in PHASES:
        raise ValueError(f"Unknown renderer probe: {case_id}")
    data_root, config_path, config = load_bundle()

    import soundfile as sf
    from voxscript_contract import SpeechPlan, SpeechProfile, SpeechUtterance
    from voxscript_contract.voices import load_voice_manifest
    from src.tts.lifecycle import CheckpointJournal, RenderCancelled, RenderControl
    from src.tts.models import ESPEECH_REVISION, VOXCPM_REVISION
    from src.tts.speech_plan import render_speech_plan
    from src.tts.worker import verify_complete_result

    voice_id = "marina_elevenlabs_ru"
    manifest_path = Path(config["voice_store"]) / voice_id / "voice.json"
    voice = load_voice_manifest(manifest_path)
    reference = next(item for item in voice.references
                     if item.reference_id == f"ref-{engine_id}-neutral")
    if reference.qualification.status != "unverified":
        raise ValueError("This evaluation is frozen to the unverified reference state")
    reference_path = voice.path.parent / reference.audio.path
    profile_path = Path(config["profile_root"]) / f"native-plan-{engine_id}.json"
    model_revision = VOXCPM_REVISION if engine_id == "voxcpm2" else ESPEECH_REVISION
    root = data_root / "tts-evidence" / "renderer-cache" / engine_id
    phase_root = root / phase
    outcome_path = phase_root / "outcome.json"
    if phase_root.exists():
        raise FileExistsError(f"Probe result already exists: {phase_root}")
    phase_root.mkdir(parents=True)

    texts = CANCEL_TEXTS if phase in {"cancel", "resume"} else TEXTS
    if phase == "changed":
        texts = (CHANGED_TEXT, TEXTS[1])
    utterances = tuple(SpeechUtterance(
        utterance_id=f"utterance-{index}", cue_id=f"cue-{index}", text_range=None,
        text=value, role_id="narrator", voice_id=voice_id,
        reference_id=reference.reference_id, reference_audio=reference_path,
        reference_audio_sha256=sha(reference_path),
        reference_text=reference.transcript, voice_manifest_sha256=sha(manifest_path),
        emotion_id="neutral", start_ms=None, end_ms=None, speed=1.0,
        volume=1.0, seed_key=f"renderer-probe-{index}",
    ) for index, value in enumerate(texts, start=1))
    plan_digest = text_sha(json.dumps({
        "case_family": "renderer-cache-technical-v1", "engine_id": engine_id,
        "texts": texts, "voice_manifest_sha256": sha(manifest_path),
        "reference_sha256": sha(reference_path), "profile_sha256": sha(profile_path),
    }, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
    plan = SpeechPlan(
        plan_id="renderer-cache-technical-v1", project_id="technical-evaluation",
        mode="subtitle_utterances", source_sha256=text_sha("\n".join(texts)),
        text_revision="technical-evaluation-v1",
        review_decision_id="technical-evaluation-no-human-approval",
        profile=SpeechProfile(profile_path.stem, sha(profile_path), engine_id, model_revision),
        plan_digest=plan_digest, utterances=utterances,
    )
    cache_dir = root / ("recovery-cache" if phase in {"cancel", "resume"} else "cache")
    checkpoint_root = root / "checkpoints"
    run_id = "recovery" if phase in {"cancel", "resume"} else "cache-matrix"
    journal = CheckpointJournal(checkpoint_root, run_id=run_id,
                                attempt_id=phase, plan_digest=plan_digest)
    cancel_flag = False
    def saved(_record):
        nonlocal cancel_flag
        if phase == "cancel":
            cancel_flag = True
    control = RenderControl(journal, cancel_requested=lambda: cancel_flag, on_saved=saved)
    wav_path = phase_root / "output.wav"
    result_manifest_path = phase_root / "manifest.json"
    baseline_manifest_path = root / "baseline" / "manifest.json"
    frozen = {
        "case_id": case_id, "engine_id": engine_id, "phase": phase,
        "voice_id": voice_id, "reference_id": reference.reference_id,
        "qualification_status": reference.qualification.status,
        "texts": list(texts), "text_sha256": [text_sha(value) for value in texts],
        "cue_ids": [item.cue_id for item in utterances],
        "seed_keys": [item.seed_key for item in utterances],
        "technical_plan_digest": plan_digest,
        "voice_manifest_sha256": sha(manifest_path),
        "reference_sha256": sha(reference_path), "profile_sha256": sha(profile_path),
        "config_sha256": sha(config_path), "model_revision": model_revision,
        "model_assets_sha256": config["integrity_sha256"],
        "bundle_sha256": json.loads((data_root / "tts/receipt.json").read_text(encoding="utf-8"))["bundle_sha256"],
        "python": sys.version,
    }
    started = time.monotonic()
    record = {"version": 1, "input": frozen, "status": "failed"}
    try:
        if phase in {"repeat", "corrupt", "changed"} and not baseline_manifest_path.is_file():
            raise FileNotFoundError("Baseline manifest is required")
        baseline = json.loads(baseline_manifest_path.read_text(encoding="utf-8")) if baseline_manifest_path.is_file() else None
        if phase == "corrupt":
            damaged = Path(baseline["segments"][0]["audio"])
            if not damaged.resolve().is_relative_to(cache_dir.resolve()):
                raise ValueError("Cannot corrupt a cache file outside this probe")
            damaged.write_bytes(b"corrupt-technical-probe")
            frozen["corrupted_cache_path"] = str(damaged)
            frozen["corrupted_cache_sha256"] = sha(damaged)
        try:
            render_speech_plan(plan, profile_path=profile_path, out_file=wav_path,
                               cache_dir=cache_dir, manifest_path=result_manifest_path,
                               control=control)
        except RenderCancelled:
            if phase != "cancel":
                raise
            checkpoints = sorted(journal.directory.glob("*.json"))
            if len(checkpoints) != 1 or result_manifest_path.exists() or wav_path.exists():
                raise AssertionError("Cancellation did not stop after one durable checkpoint")
            record.update(status="cancelled", checkpoints=[json.loads(path.read_text(encoding="utf-8")) for path in checkpoints],
                          checkpoint_sha256=[sha(path) for path in checkpoints])
        else:
            if phase == "cancel":
                raise AssertionError("Expected renderer cancellation")
            verified = verify_complete_result(plan, wav_path, result_manifest_path)
            rendered = json.loads(result_manifest_path.read_text(encoding="utf-8"))
            segments = rendered["segments"]
            statuses = [item["status"] for item in segments]
            expected = {
                "baseline": ["generated", "generated"],
                "repeat": ["cached", "cached"],
                "corrupt": ["generated", "cached"],
                "changed": ["generated", "cached"],
                "resume": ["cached", "generated"],
            }[phase]
            if statuses != expected:
                raise AssertionError(f"Unexpected cache statuses: {statuses} != {expected}")
            if len(segments) != 2:
                raise AssertionError("Technical probe unexpectedly split an utterance")
            if phase in {"repeat", "corrupt"} and verified["output_sha256"] != baseline["output_sha256"]:
                raise AssertionError("Cache recovery changed the final WAV")
            if phase == "changed":
                if (segments[0]["cache_key"] == baseline["segments"][0]["cache_key"]
                        or segments[1]["cache_key"] != baseline["segments"][1]["cache_key"]):
                    raise AssertionError("Changed text invalidated the wrong cache entries")
            if phase == "resume":
                previous = sorted((checkpoint_root / "recovery" / "cancel").glob("*.json"))
                if len(previous) != 1:
                    raise AssertionError("No durable cancellation checkpoint to resume")
                prior = json.loads(previous[0].read_text(encoding="utf-8"))
                if (prior["audio_sha256"] != segments[0]["audio_sha256"]
                        or prior["cache_key"] != segments[0]["cache_key"]):
                    raise AssertionError("Resume did not reuse the saved first segment")
            info = sf.info(str(wav_path))
            checkpoints = sorted(journal.directory.glob("*.json"))
            if len(checkpoints) != 2 or info.format != "WAV" or info.frames <= 0:
                raise AssertionError("Completed renderer output or checkpoints are invalid")
            record.update(status="completed", verified=verified,
                          wav_sha256=sha(wav_path), manifest_sha256=sha(result_manifest_path),
                          duration_ms=round(info.frames * 1000 / info.samplerate),
                          sample_rate=info.samplerate, channels=info.channels, frames=info.frames,
                          segments=[{key: item.get(key) for key in
                                     ("utterance_id", "cue_id", "text", "status", "cache_key", "audio_sha256", "duration_ms", "audio")}
                                    for item in segments],
                          checkpoints=[json.loads(path.read_text(encoding="utf-8")) for path in checkpoints],
                          checkpoint_sha256=[sha(path) for path in checkpoints])
    except BaseException as error:
        record["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        record["elapsed_ms"] = round((time.monotonic() - started) * 1000)
        record["finished_at"] = datetime.now(timezone.utc).isoformat()
        outcome_path.write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"{case_id}: {record['status']}, {record['elapsed_ms']} ms")


if __name__ == "__main__":
    main()
