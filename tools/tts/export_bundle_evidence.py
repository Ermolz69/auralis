"""Compare installed standalone model probes with frozen source probes."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import wave


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    workspace = Path(__file__).resolve().parents[3]
    parser.add_argument("--data-root", type=Path, required=True)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--original", type=Path, default=workspace / "CONTRACT/VOXSCRIPT_CONTRACT/tests/evidence/NEW_VOICES_20261021.json")
    parser.add_argument("--output", type=Path, default=workspace / "CONTRACT/VOXSCRIPT_CONTRACT/tests/evidence/STANDALONE_BUNDLE_20261021.json")
    args = parser.parse_args()
    data_root = args.data_root.resolve(strict=True)
    bundle = args.bundle.resolve(strict=True)
    original = json.loads(args.original.read_text(encoding="utf-8"))
    source_cases = {item["input"]["case_id"]: item for item in original["cases"]}
    config_path = data_root / "tts-runtime.json"
    config = json.loads(config_path.read_text(encoding="utf-8"))
    receipt = json.loads((data_root / "tts/receipt.json").read_text(encoding="utf-8"))
    if any(not Path(config[key]).resolve().is_relative_to(data_root) for key in
           ("python_executable", "package_root", "voice_store", "profile_root", "model_cache")):
        raise ValueError("Standalone runtime refers outside its data root")
    if receipt["bundle_sha256"] != sha(bundle / "bundle.json"):
        raise ValueError("Installed bundle manifest changed")
    cases = []
    for engine_id in ("voxcpm2", "espeech"):
        for sample in ("short", "long"):
            case_id = f"{engine_id}-{sample}"
            outcome_path = data_root / "tts-evidence" / case_id / "outcome.json"
            result = json.loads(outcome_path.read_text(encoding="utf-8"))
            wav = outcome_path.parent / "sample.wav"
            source = source_cases[f"{engine_id}-marina_elevenlabs_ru-{sample}"]
            frozen = source["input"]
            match = (
                result["status"] == "completed"
                and result["input"]["text"] == frozen["text"]
                and result["input"]["text_sha256"] == frozen["text_sha256"]
                and result["input"]["manifest_sha256"] == frozen["manifest_sha256"]
                and result["input"]["reference_sha256"] == frozen["reference_audio_sha256"]
                and result["input"]["profile_sha256"] == frozen["profile_sha256"]
                and result["input"]["model_revision"] == frozen["model_revision"]
                and result["wav_sha256"] == source["outcome"]["output_sha256"]
                and result["duration_ms"] == source["outcome"]["duration_ms"]
                and result["input"]["qualification_status"] == "unverified"
                and sha(wav) == result["wav_sha256"]
            )
            with wave.open(str(wav), "rb") as audio:
                match = match and audio.getnchannels() == 1 and audio.getframerate() == result["sample_rate"] and audio.getnframes() == result["frames"]
            if not match:
                raise ValueError(f"Standalone probe differs from frozen technical source: {case_id}")
            cases.append({"case_id": case_id, "outcome": result,
                          "outcome_sha256": sha(outcome_path),
                          "source_case_id": frozen["case_id"]})
    evidence = {
        "version": 1,
        "status": "four_standalone_model_probes_byte_identical_to_frozen_source",
        "qualification": "human_review_pending",
        "native_auralis_run": "blocked_on_unverified_voice",
        "clean_windows_verified": False,
        "bundle_manifest_sha256": receipt["bundle_sha256"],
        "runtime_config_sha256": sha(config_path),
        "runtime_config": config,
        "wheel_sha256": receipt["wheels"],
        "source_commits": {key: receipt[key] for key in ("tts_commit", "contract_commit", "caster_commit")},
        "cases": cases,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Verified {len(cases)} byte-identical standalone technical WAVs: {args.output}")


if __name__ == "__main__":
    main()
