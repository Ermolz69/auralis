"""Technical model probe using only an installed standalone PUZA bundle."""

from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import sys
import time


SHORT_TEXT = "Добрый вечер. Мы проверяем ясность русской речи."
LONG_TEXT = (
    "За окном медленно темнеет, а на столе остывает чай. "
    "Мы сохраним точный текст каждой реплики и проверим, как звучат длинные фразы. "
    "После остановки работа продолжится с сохранённого места."
)


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main() -> None:
    if os.environ.get("PUZA_PROBE_ADMITTED") != "1":
        raise RuntimeError("Shared model admission is required")
    case_id = sys.argv[sys.argv.index("--case-id") + 1]
    engine_id, sample = case_id.rsplit("-", 1)
    if engine_id not in {"voxcpm2", "espeech"} or sample not in {"short", "long"}:
        raise ValueError("Unknown installed-bundle probe")
    data_root = Path(os.environ["AURALIS_TTS_BUNDLE_DATA_ROOT"]).resolve(strict=True)
    config_path = data_root / "tts-runtime.json"
    config = json.loads(config_path.read_text(encoding="utf-8"))
    if Path(config["python_executable"]).resolve() != Path(sys.executable).resolve():
        raise ValueError("The installed bundle Python is not executing")
    runtime = Path(sys.executable).parent.parent
    if Path(sys.base_prefix).resolve() != runtime / "base":
        raise ValueError("The bundle is using an external Python base")
    if not all(Path(config[key]).resolve().is_relative_to(data_root) for key in
               ("package_root", "voice_store", "profile_root", "model_cache")):
        raise ValueError("Installed runtime escaped its data root")
    os.environ.update(HF_HOME=config["model_cache"],
                      HF_HUB_CACHE=str(Path(config["model_cache"]) / "hub"),
                      HF_HUB_OFFLINE="1", TTS_REQUIRE_PRELOADED_MODELS="1",
                      TTS_VOXCPM_REVISION=config["voxcpm_revision"])
    sys.path.insert(0, config["package_root"])
    import numpy as np
    import soundfile as sf
    from voxscript_contract.voices import load_voice_manifest
    from src.tts.schema import RenderJob, RenderSegment
    from src.tts.engine.voxcpm2_engine import VoxCPM2Engine
    from src.tts.engine.espeech_engine import ESpeechEngine

    voice_id = "marina_elevenlabs_ru"
    manifest_path = Path(config["voice_store"]) / voice_id / "voice.json"
    manifest = load_voice_manifest(manifest_path)
    reference = next(item for item in manifest.references
                     if item.reference_id == f"ref-{engine_id}-neutral")
    profile_path = Path(config["profile_root"]) / f"native-plan-{engine_id}.json"
    profile = json.loads(profile_path.read_text(encoding="utf-8"))
    root = data_root / "tts-evidence" / case_id
    root.mkdir(parents=True, exist_ok=True)
    output = root / "sample.wav"
    outcome = root / "outcome.json"
    if output.exists() or outcome.exists():
        raise FileExistsError(f"Probe result already exists: {case_id}")
    text = SHORT_TEXT if sample == "short" else LONG_TEXT
    reference_path = manifest.path.parent / reference.audio.path
    frozen = {
        "case_id": case_id, "engine_id": engine_id, "voice_id": voice_id,
        "reference_id": reference.reference_id,
        "qualification_status": reference.qualification.status,
        "text": text, "text_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
        "manifest_sha256": sha(manifest_path), "reference_sha256": sha(reference_path),
        "profile_sha256": sha(profile_path),
        "config_sha256": sha(config_path),
        "bundle_sha256": json.loads((data_root / "tts/receipt.json").read_text(encoding="utf-8"))["bundle_sha256"],
        "model_assets_sha256": config["integrity_sha256"],
        "model_revision": config["voxcpm_revision"] if engine_id == "voxcpm2" else
                          "f582b6e5897fe8a5835059405a8439d13bdf7684",
        "python": sys.version,
    }
    job = RenderJob(input_path=None, out_file=output, engine=engine_id,
                    mode="clone", settings=profile["settings"])
    segment = RenderSegment(id=case_id, kind="text", text=text,
                            reference_audio=reference_path, reference_text=reference.transcript,
                            reference_id=reference.reference_id, voice=voice_id,
                            resolved_voice=voice_id, emotion="neutral", resolved_emotion="neutral")
    engine = None
    started = time.monotonic()
    try:
        engine = VoxCPM2Engine(job) if engine_id == "voxcpm2" else ESpeechEngine(job)
        result = engine.synthesize(segment, output)
        info = sf.info(output)
        audio, rate = sf.read(output, dtype="float32", always_2d=True)
        rms = float(np.sqrt(np.mean(audio * audio)))
        if info.format != "WAV" or info.channels != 1 or info.frames <= 0 or rate != result.sample_rate or not np.isfinite(audio).all() or rms <= 0:
            raise ValueError("Installed bundle generated invalid WAV")
        record = {"version": 1, "status": "completed", "input": frozen,
                  "finished_at": datetime.now(timezone.utc).isoformat(),
                  "elapsed_ms": round((time.monotonic() - started) * 1000),
                  "wav_sha256": sha(output), "sample_rate": rate, "channels": info.channels,
                  "frames": info.frames, "duration_ms": round(info.frames * 1000 / rate),
                  "rms": rms, "peak": float(np.max(np.abs(audio))),
                  "effective_seed": segment.effective_seed,
                  "engine_input_text": segment.engine_input_text,
                  "hidden_chunks": segment.hidden_chunks}
    except BaseException as error:
        record = {"version": 1, "status": "failed", "input": frozen,
                  "elapsed_ms": round((time.monotonic() - started) * 1000),
                  "error": f"{type(error).__name__}: {error}"}
        raise
    finally:
        if engine is not None:
            engine.close()
        outcome.write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"{case_id}: {record['duration_ms']} ms, SHA-256 {record['wav_sha256']}")


if __name__ == "__main__":
    main()
