"""Install pinned local PUZA components without loading a speech model."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import importlib.metadata


TTS_COMMIT = "0da6c6f05075f372cbae08c3351df8a569e6d4b1"
CONTRACT_COMMIT = "d0a09a97ca51b282b682ec766a016ea783cd3e48"
VOX_REVISION = "32279effe8c19989596f05d353d1447f51d9e915"


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def atomic_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    data = (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as stream:
        temporary = Path(stream.name)
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)


def main() -> None:
    parser = argparse.ArgumentParser()
    workspace = Path(__file__).resolve().parents[3]
    parser.add_argument("--tts-root", type=Path, default=workspace / "TTS")
    parser.add_argument("--contract-root", type=Path, default=workspace / ".integration/CONTRACT")
    parser.add_argument("--voice-source", type=Path, default=workspace / "CASTER/voices")
    parser.add_argument("--data-root", type=Path, default=Path(os.environ["APPDATA"]) / "com.auralis.desktop")
    parser.add_argument("--model-cache", type=Path, default=Path.home() / ".cache/huggingface")
    args = parser.parse_args()
    if sys.version_info[:2] != (3, 12):
        raise ValueError("The local GPU runtime requires Python 3.12")
    root = args.data_root.resolve()
    wheels = root / "tts/wheels"
    wheels.mkdir(parents=True, exist_ok=True)
    for repository, expected in [(args.tts_root, TTS_COMMIT), (args.contract_root, CONTRACT_COMMIT)]:
        actual = subprocess.check_output(
            ["git", "-c", f"safe.directory={repository.as_posix()}", "-C", str(repository), "rev-parse", "HEAD"], text=True,
        ).strip()
        if actual != expected:
            raise ValueError(f"Component revision differs from the pinned candidate: {repository}")
        component = "PUZA_TTS" if repository == args.tts_root else "VOXSCRIPT_CONTRACT"
        subprocess.run(["git", "-c", f"safe.directory={repository.as_posix()}", "-C", str(repository), "diff", "--exit-code", "HEAD", "--", component], check=True)
    wheel_files = [wheels / "voxscript_contract-0.2.0-py3-none-any.whl", wheels / "puza_tts-0.3.0-py3-none-any.whl"]
    previous_path = root / "tts/receipt.json"
    if previous_path.exists():
        previous = json.loads(previous_path.read_text(encoding="utf-8"))
        if previous["tts_commit"] != TTS_COMMIT or previous["contract_commit"] != CONTRACT_COMMIT:
            raise ValueError("Existing installation has different pinned component revisions")
        for path in wheel_files:
            if digest(path) != previous["wheels"][path.name]:
                raise ValueError(f"Retained component wheel changed: {path.name}")
    else:
        build_env = dict(os.environ, SOURCE_DATE_EPOCH="1791676800")
        for source in [args.contract_root / "VOXSCRIPT_CONTRACT", args.tts_root / "PUZA_TTS"]:
            subprocess.run([sys.executable, "-m", "pip", "wheel", "--no-deps", "--no-build-isolation", "--wheel-dir", str(wheels), str(source)], check=True, env=build_env)
    identity = hashlib.sha256("".join(digest(path) for path in wheel_files).encode()).hexdigest()
    packages = root / "tts/packages" / identity
    if not (packages / "installation.json").is_file():
        packages.mkdir(parents=True, exist_ok=True)
        subprocess.run([sys.executable, "-m", "pip", "install", "--no-index", "--no-deps", "--upgrade", "--target", str(packages), *map(str, wheel_files)], check=True)
        atomic_json(packages / "installation.json", {"version": 1, "wheels": {path.name: digest(path) for path in wheel_files}})
    sys.path.insert(0, str(packages))
    from voxscript_contract.voices import import_legacy_voice, load_voice_manifest
    voice_store = root / "tts/voices"
    voice_store.mkdir(parents=True, exist_ok=True)
    for voice in ["ciri", "geralt"]:
        for engine in ["voxcpm2", "espeech"]:
            voice_id = f"{voice}_{engine}"
            destination = voice_store / voice_id
            if not destination.exists():
                import_legacy_voice(args.voice_source / voice, destination, engine_id=engine, voice_id=voice_id, display_name=voice.capitalize(), description=f"Local Auralis {engine} reference copy")
            loaded = load_voice_manifest(destination / "voice.json")
            if loaded.voice_id != voice_id:
                raise ValueError(f"Voice identity differs: {voice_id}")
    profiles = packages / "share/puza-tts/profiles"
    if not profiles.is_dir():
        raise ValueError("The installed wheel did not include its engine profiles")
    integrity_path = packages / "integrity.json"
    if not integrity_path.exists():
        package_files = [{"path": str(path.relative_to(packages)), "sha256": digest(path)}
            for path in sorted(packages.rglob("*")) if path.is_file()
            and path.suffix != ".pyc" and "__pycache__" not in path.parts
            and path.name != "installation.json"]
        model_files = []
        for engine, repository, revision in [
            ("voxcpm2", "openbmb--VoxCPM2", VOX_REVISION),
            ("espeech", "ESpeech--ESpeech-TTS-1_RL-V2", "f582b6e5897fe8a5835059405a8439d13bdf7684"),
            ("espeech", "charactr--vocos-mel-24khz", "0feb3fdd929bcd6649e0e7c5a688cf7dd012ef21"),
        ]:
            snapshot = args.model_cache / "hub" / f"models--{repository}" / "snapshots" / revision
            files = sorted(path for path in snapshot.rglob("*") if path.is_file())
            if not files: raise ValueError(f"Pinned local model snapshot is missing: {snapshot}")
            model_files.extend({"engine": engine, "path": str(path.relative_to(args.model_cache)), "sha256": digest(path)} for path in files)
        dependencies = []
        for name in ["torch", "torchaudio", "transformers", "voxcpm", "numpy", "soundfile", "vocos", "setuptools"]:
            distribution = importlib.metadata.distribution(name)
            metadata = distribution.locate_file(f"{distribution._path.name}/METADATA")
            dependencies.append({"name": name, "version": distribution.version, "path": str(metadata), "sha256": digest(metadata)})
        atomic_json(integrity_path, {"version": 1, "python": sys.version, "package_files": package_files, "model_files": model_files, "dependencies": dependencies})
    config = {
        "version": 1,
        "python_executable": sys.executable,
        "package_root": str(packages),
        "voice_store": str(voice_store),
        "profile_root": str(profiles),
        "model_cache": str(args.model_cache.resolve()),
        "voxcpm_revision": VOX_REVISION,
        "integrity_sha256": digest(integrity_path),
    }
    atomic_json(root / "tts-runtime.json", config)
    atomic_json(root / "tts/receipt.json", {"version": 1, "tts_commit": TTS_COMMIT, "contract_commit": CONTRACT_COMMIT, "installation_id": identity, "wheels": {path.name: digest(path) for path in wheel_files}, "runtime": config})
    print(f"Installed local TTS runtime: {root / 'tts-runtime.json'}")
    print("Voice copies retain their real qualification state. No model was loaded.")


if __name__ == "__main__":
    main()
