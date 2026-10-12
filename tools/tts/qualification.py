"""Retain real probe evidence and apply only explicit human scores through CONTRACT."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

WORKSPACE = Path(__file__).resolve().parents[3]
PROBES = WORKSPACE / "TTS/PUZA_TTS/.tmp_audio/auralis-local-20261011"

def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["prepare", "apply"])
    parser.add_argument("--data-root", type=Path, default=Path(os.environ["APPDATA"]) / "com.auralis.desktop")
    parser.add_argument("--decisions", type=Path)
    args = parser.parse_args()
    config = json.loads((args.data_root / "tts-runtime.json").read_text(encoding="utf-8"))
    sys.path.insert(0, config["package_root"])
    sys.path.insert(0, str(WORKSPACE / "TTS/ci/tts"))
    os.environ["AURALIS_QUALIFICATION_ROOT"] = str(PROBES)
    os.chdir(WORKSPACE / "TTS/PUZA_TTS")
    from inspect_qualification_case import inspect
    from voxscript_contract.voices import load_voice_manifest
    from voxscript_contract.voice_qualification import validate_voice_preparation_report, qualify_voice
    voices = Path(config["voice_store"])
    reports = args.data_root / "tts/qualification"
    reports.mkdir(parents=True, exist_ok=True)
    registry_path = PROBES / "qualification-cases.json"
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    decisions = None
    if args.action == "apply":
        if args.decisions is None:
            raise ValueError("An explicit human decisions file is required")
        decisions = json.loads(args.decisions.read_text(encoding="utf-8"))
    for case in registry["cases"]:
        inspect(case["case_id"])
        voice_id = f"{case['voice_id']}_{case['engine_id']}"
        voice_root = voices / voice_id
        voice_path = voice_root / "voice.json"
        voice_bytes = voice_path.read_bytes()
        outcome = json.loads((PROBES / case["case_id"] / "attempt-outcome.json").read_text(encoding="utf-8"))
        report_path = reports / f"{voice_id}.awaiting.json"
        report = {
            "version": 1, "status": "awaiting_review", "voice_id": voice_id,
            "corpus_id": "auralis_tts_neutral_probe", "corpus_version": f"20261011-{digest(registry_path)[:12]}",
            "input_hashes": {"voice_manifest_sha256": digest(voice_path), "corpus_sha256": digest(registry_path)},
            "engines": [case["engine_id"]], "seeds": [case["expected_effective_seed"]],
            "references": [{"reference_id": f"{case['engine_id']}-neutral-01", "audio_sha256": case["reference_audio_sha256"],
                "cases": [{"case_id": case["case_id"], "engine_id": case["engine_id"], "seed": case["expected_effective_seed"], "output_sha256": outcome["output_sha256"]}],
                "scores": None, "notes": None, "decision": None, "human_reviewed": False}],
        }
        if decisions is None:
            validate_voice_preparation_report(report, load_voice_manifest(voice_path), voice_bytes)
            data = (json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")
            if report_path.exists() and report_path.read_bytes() != data:
                raise ValueError(f"Awaiting review evidence differs: {voice_id}")
            if not report_path.exists(): report_path.write_bytes(data)
            print(f"Awaiting human review: {voice_id}; WAV {case['output']}")
        else:
            decision = decisions[voice_id]
            if set(decision) != {"scores", "notes", "decision", "human_reviewed"} or decision["human_reviewed"] is not True:
                raise ValueError(f"Explicit human scores and decision are required: {voice_id}")
            report = json.loads(report_path.read_text(encoding="utf-8"))
            report["status"] = "reviewed"
            report["references"][0].update(decision)
            reviewed_path = reports / f"{voice_id}.reviewed.json"
            data = (json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")
            if reviewed_path.exists() and reviewed_path.read_bytes() != data:
                raise ValueError("A retained human decision cannot be overwritten")
            reviewed_path.write_bytes(data)
            result = qualify_voice(voice_root, reviewed_path)
            print(f"Human decision recorded through CONTRACT: {result}")

if __name__ == "__main__":
    main()
