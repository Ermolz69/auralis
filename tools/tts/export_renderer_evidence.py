"""Verify and freeze all twelve installed-bundle renderer recovery outcomes."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


PHASES = ("baseline", "repeat", "corrupt", "changed", "cancel", "resume")
EXPECTED = {
    "baseline": ("generated", "generated"),
    "repeat": ("cached", "cached"),
    "corrupt": ("generated", "cached"),
    "changed": ("generated", "cached"),
    "resume": ("cached", "generated"),
}


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def verify(data_root: Path) -> dict:
    receipt_path = data_root / "tts" / "receipt.json"
    config_path = data_root / "tts-runtime.json"
    receipt, config = read(receipt_path), read(config_path)
    root = data_root / "tts-evidence" / "renderer-cache"
    cases = []
    for engine in ("voxcpm2", "espeech"):
        family = root / engine
        baseline = read(family / "baseline" / "outcome.json")
        for phase in PHASES:
            phase_root = family / phase
            outcome_path = phase_root / "outcome.json"
            outcome = read(outcome_path)
            assert outcome["input"]["case_id"] == f"{engine}-{phase}"
            assert outcome["input"]["engine_id"] == engine
            assert outcome["input"]["qualification_status"] == "unverified"
            assert outcome["input"]["bundle_sha256"] == receipt["bundle_sha256"]
            assert outcome["input"]["model_assets_sha256"] == config["integrity_sha256"]
            assert outcome["input"]["config_sha256"] == sha(config_path)
            assert outcome["elapsed_ms"] > 0
            checkpoints = family / "checkpoints" / ("recovery" if phase in {"cancel", "resume"} else "cache-matrix") / phase
            checkpoint_files = sorted(checkpoints.glob("*.json"))
            assert [sha(path) for path in checkpoint_files] == outcome["checkpoint_sha256"]
            assert [read(path) for path in checkpoint_files] == outcome["checkpoints"]
            if phase == "cancel":
                assert outcome["status"] == "cancelled" and len(checkpoint_files) == 1
                assert not (phase_root / "manifest.json").exists()
                assert not (phase_root / "output.wav").exists()
            else:
                assert outcome["status"] == "completed" and len(checkpoint_files) == 2
                wav_path, manifest_path = phase_root / "output.wav", phase_root / "manifest.json"
                manifest = read(manifest_path)
                assert sha(wav_path) == outcome["wav_sha256"] == manifest["output_sha256"]
                assert sha(manifest_path) == outcome["manifest_sha256"]
                assert outcome["duration_ms"] == outcome["verified"]["duration_ms"]
                assert [item["status"] for item in manifest["segments"]] == list(EXPECTED[phase])
                assert [item["status"] for item in outcome["segments"]] == list(EXPECTED[phase])
                for segment in manifest["segments"]:
                    assert sha(Path(segment["audio"])) == segment["audio_sha256"]
                if phase in {"repeat", "corrupt"}:
                    assert outcome["wav_sha256"] == baseline["wav_sha256"]
                if phase == "changed":
                    assert outcome["segments"][0]["cache_key"] != baseline["segments"][0]["cache_key"]
                    assert outcome["segments"][1]["cache_key"] == baseline["segments"][1]["cache_key"]
                if phase == "resume":
                    first = read(family / "checkpoints" / "recovery" / "cancel" / "00000001.json")
                    assert outcome["segments"][0]["cache_key"] == first["cache_key"]
                    assert outcome["segments"][0]["audio_sha256"] == first["audio_sha256"]
            cases.append({"case_id": f"{engine}-{phase}", "outcome": outcome,
                          "outcome_sha256": sha(outcome_path)})
    source = Path(__file__).resolve().parent
    return {
        "version": 1,
        "status": "twelve_sequential_real_renderer_recovery_probes_verified",
        "qualification": "human_review_pending",
        "native_auralis_run": "blocked_on_unverified_voice",
        "clean_windows_verified": False,
        "source_sha256": {
            "probe_renderer_cache.py": sha(source / "probe_renderer_cache.py"),
            "run_bundle_probe.ps1": sha(source / "run_bundle_probe.ps1"),
        },
        "runtime_config_sha256": sha(config_path),
        "bundle_manifest_sha256": receipt["bundle_sha256"],
        "model_assets_sha256": config["integrity_sha256"],
        "installed_source_commits": {key: receipt[key] for key in ("tts_commit", "contract_commit", "caster_commit")},
        "wheel_sha256": receipt["wheels"],
        "cases": cases,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    evidence = verify(args.data_root.resolve(strict=True))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes((json.dumps(evidence, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
    print(f"Verified {len(evidence['cases'])} renderer outcomes: {args.output}")


if __name__ == "__main__":
    main()
