"""Retain exact inputs and outcomes of the eight-voice technical probe set."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
from pathlib import Path
import sys


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    workspace = Path(__file__).resolve().parents[3]
    parser.add_argument("--probe-root", type=Path, default=workspace / "TTS/PUZA_TTS/.tmp_audio/new-voices-20261021")
    parser.add_argument("--output", type=Path, default=workspace / "CONTRACT/VOXSCRIPT_CONTRACT/tests/evidence/NEW_VOICES_20261021.json")
    args = parser.parse_args()
    probe_root = args.probe_root.resolve(strict=True)
    registry_path = probe_root / "cases.json"
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    cases = []
    for frozen in registry["cases"]:
        case_id = frozen["case_id"]
        outcome_path = probe_root / case_id / "outcome.json"
        outcome = json.loads(outcome_path.read_text(encoding="utf-8"))
        wav_path = probe_root / case_id / "sample.wav"
        if outcome["input"] != frozen or outcome["status"] != "completed":
            raise ValueError(f"Frozen input or technical outcome changed: {case_id}")
        if sha256(wav_path) != outcome["output_sha256"]:
            raise ValueError(f"Probe WAV changed: {case_id}")
        cases.append({"input": frozen, "outcome": outcome, "outcome_sha256": sha256(outcome_path)})
    if len(cases) != 18:
        raise ValueError(f"Expected 18 probes, received {len(cases)}")
    reports = {}
    for path in sorted((probe_root / "qualification-reports").glob("*.awaiting.json")):
        report = json.loads(path.read_text(encoding="utf-8"))
        if report.get("status") != "awaiting_review":
            raise ValueError(f"Human decision already exists; do not export as unreviewed: {path}")
        reports[path.name] = sha256(path)
    if len(reports) != 8:
        raise ValueError(f"Expected eight awaiting reports, received {len(reports)}")
    wheels = workspace / "TTS/PUZA_TTS/.tmp_audio/wheels"
    evidence = {
        "version": 1,
        "technical_status": "18_completed_wav_probes_verified",
        "human_status": "awaiting_review",
        "python_version": sys.version,
        "package_versions": {name: importlib.metadata.version(name) for name in
                             ("torch", "torchaudio", "voxcpm", "f5-tts", "soundfile")},
        "source_commits": {
            "tts": "22dcbb12a37bc1e54d0d8cf7f52adb9668de36a7",
            "contract_worker": "4ab50d5dffbe611dc7d60d0fe292614f6c47cd8a",
            "caster": "2dbab64abeb4d3e7eee7bc819a5578a7008b3c5d",
        },
        "registry_sha256": sha256(registry_path),
        "listening_index_sha256": sha256(probe_root / "INDEX.html"),
        "wheel_sha256": {path.name: sha256(path) for path in sorted(wheels.glob("*.whl"))},
        "awaiting_report_sha256": reports,
        "cases": cases,
        "clean_windows_verified": False,
        "native_auralis_qualified_voice_run_verified": False,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Retained {len(cases)} exact technical outcomes: {args.output}")


if __name__ == "__main__":
    main()
