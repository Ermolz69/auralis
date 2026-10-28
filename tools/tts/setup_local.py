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
import shutil


TTS_COMMIT = "49662267df9ee467e61a0d4e6a4caf7955f54a7d"
CONTRACT_COMMIT = "13f4383c7ca8b8b7b575eb64fdf9c54856a9204a"
CASTER_COMMIT = "2dbab64abeb4d3e7eee7bc819a5578a7008b3c5d"
VOICE_IDS = (
    "marina_elevenlabs_ru",
    "alex_analyst_elevenlabs_ru",
    "anastasiya_kim_elevenlabs_ru",
    "artur_denophine_elevenlabs_ru",
    "denis_elevenlabs_ru",
    "nastya_elevenlabs_ru",
    "rina_elevenlabs_ru",
    "stanislav_elevenlabs_ru",
)
NATIVE_PACKAGES = (
    "torch", "torchaudio", "transformers", "voxcpm", "f5-tts",
    "huggingface-hub", "accelerate", "safetensors", "numpy",
    "soundfile", "scipy", "vocos", "setuptools",
)
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


def installed_python(source: Path, destination_root: Path) -> Path:
    """Relocate the pinned native environment so runtime never opens a checkout Python."""
    source = source.resolve(strict=True)
    lines = (source / "pyvenv.cfg").read_text(encoding="utf-8").splitlines()
    homes = [line.partition("=")[2].strip() for line in lines if line.lower().startswith("home =")]
    if len(homes) != 1:
        raise ValueError("Native Python has no single pinned base interpreter")
    base = Path(homes[0]).resolve(strict=True)
    source_python = source / "Scripts/python.exe"
    base_python = base / "python.exe"
    evidence = [digest(source_python), digest(base_python), digest(source / "pyvenv.cfg")]
    for name in NATIVE_PACKAGES:
        distribution = importlib.metadata.distribution(name)
        evidence.extend((distribution.version, digest(source / "Lib/site-packages" /
                                                      distribution._path.name / "METADATA")))
    runtime_id = hashlib.sha256("".join(evidence).encode()).hexdigest()[:16]
    target = destination_root / runtime_id
    python = target / "base/python.exe"
    if not python.exists():
        destination_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=destination_root, prefix=".install-") as temporary:
            staged = Path(temporary) / runtime_id
            shutil.copytree(base, staged / "base", ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
            shutil.copytree(source, staged / "venv", ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
            (staged / "base/Lib/site-packages/auralis-native-venv.pth").write_text(
                "../../../venv/Lib/site-packages\n", encoding="utf-8"
            )
            site = staged / "venv/Lib/site-packages"
            for pth in site.glob("*.pth"):
                entries = pth.read_text(encoding="utf-8").splitlines()
                if any(Path(line.strip()).is_absolute() for line in entries if line.strip() and not line.startswith("import ")):
                    pth.unlink()
            config = staged / "venv/pyvenv.cfg"
            config.write_text("\n".join(
                f"home = {staged / 'base'}" if line.lower().startswith("home =") else line
                for line in lines
            ) + "\n", encoding="utf-8")
            probe = staged / "base/python.exe"
            subprocess.run([str(probe), "-I", "-c",
                            "import sys, pathlib, torch, torchaudio, soundfile, f5_tts, voxcpm; "
                            "r=pathlib.Path(sys.argv[1]).resolve(); "
                            "assert pathlib.Path(sys.base_prefix).resolve()==r; "
                            "assert pathlib.Path(torch.__file__).resolve().is_relative_to(r.parent); "
                            "print('Pinned native Python ready')", str(staged / "base")],
                           check=True, env={key: value for key, value in os.environ.items()
                                            if key not in {"PYTHONPATH", "PYTHONHOME"}})
            staged.rename(target)
            # pyvenv.cfg must point at its final parent after the atomic rename.
            config = target / "venv/pyvenv.cfg"
            config.write_text("\n".join(
                f"home = {target / 'base'}" if line.lower().startswith("home =") else line
                for line in lines
            ) + "\n", encoding="utf-8")
    if not python.is_file():
        raise ValueError("Pinned native Python installation is incomplete")
    bridge = target / "base/Lib/site-packages/auralis-native-venv.pth"
    if not bridge.exists() or bridge.read_text(encoding="utf-8").strip() != "../../../venv/Lib/site-packages":
        bridge.write_text("../../../venv/Lib/site-packages\n", encoding="utf-8")
    subprocess.run([str(python), "-I", "-c",
                    "import sys, pathlib, torch, torchaudio, soundfile, f5_tts, voxcpm; "
                    "r=pathlib.Path(sys.argv[1]).resolve(); "
                    "assert pathlib.Path(sys.base_prefix).resolve()==r; "
                    "assert pathlib.Path(torch.__file__).resolve().is_relative_to(r.parent); "
                    "print('Pinned native Python ready')", str(target / "base")], check=True)
    return python


def main() -> None:
    parser = argparse.ArgumentParser()
    workspace = Path(__file__).resolve().parents[3]
    parser.add_argument("--tts-root", type=Path, default=workspace / "TTS")
    parser.add_argument("--contract-root", type=Path, default=workspace / "CONTRACT")
    parser.add_argument("--voice-source", type=Path, default=workspace / "CASTER/voices")
    parser.add_argument("--data-root", type=Path, default=Path(os.environ["APPDATA"]) / "com.auralis.desktop")
    parser.add_argument("--model-cache", type=Path, default=Path.home() / ".cache/huggingface")
    parser.add_argument("--runtime-source", type=Path, default=workspace / "TTS/PUZA_TTS/.venv_gpu")
    args = parser.parse_args()
    if sys.version_info[:2] != (3, 12):
        raise ValueError("The local GPU runtime requires Python 3.12")
    root = args.data_root.resolve()
    pin = hashlib.sha256((TTS_COMMIT + CONTRACT_COMMIT + CASTER_COMMIT).encode()).hexdigest()[:16]
    wheels = root / "tts/wheels" / pin
    wheels.mkdir(parents=True, exist_ok=True)
    for repository, expected in [(args.tts_root, TTS_COMMIT), (args.contract_root, CONTRACT_COMMIT), (args.voice_source.parent, CASTER_COMMIT)]:
        actual = subprocess.check_output(
            ["git", "-c", f"safe.directory={repository.as_posix()}", "-C", str(repository), "rev-parse", "HEAD"], text=True,
        ).strip()
        component = "PUZA_TTS" if repository == args.tts_root else "VOXSCRIPT_CONTRACT" if repository == args.contract_root else "voices"
        package_paths = [component]
        if repository == args.contract_root:
            package_paths.append(":(exclude)VOXSCRIPT_CONTRACT/tests/evidence")
        if actual != expected:
            subprocess.run(["git", "-c", f"safe.directory={repository.as_posix()}", "-C", str(repository),
                            "merge-base", "--is-ancestor", expected, "HEAD"], check=True)
            subprocess.run(["git", "-c", f"safe.directory={repository.as_posix()}", "-C", str(repository),
                            "diff", "--exit-code", expected, "HEAD", "--", *package_paths], check=True)
        subprocess.run(["git", "-c", f"safe.directory={repository.as_posix()}", "-C", str(repository),
                        "diff", "--exit-code", "HEAD", "--", *package_paths], check=True)
    wheel_files = [wheels / "voxscript_contract-0.2.0-py3-none-any.whl", wheels / "puza_tts-0.3.0-py3-none-any.whl"]
    previous_path = root / "tts/receipt.json"
    if previous_path.exists():
        previous = json.loads(previous_path.read_text(encoding="utf-8"))
        if previous.get("tts_commit") == TTS_COMMIT and previous.get("contract_commit") == CONTRACT_COMMIT:
            for path in wheel_files:
                if not path.is_file() or digest(path) != previous["wheels"][path.name]:
                    raise ValueError(f"Retained component wheel changed: {path.name}")
    if not any(path.exists() for path in wheel_files):
        build_env = dict(os.environ, SOURCE_DATE_EPOCH="1791676800")
        for source in [args.contract_root / "VOXSCRIPT_CONTRACT", args.tts_root / "PUZA_TTS"]:
            subprocess.run([sys.executable, "-m", "pip", "wheel", "--no-deps", "--no-build-isolation", "--wheel-dir", str(wheels), str(source)], check=True, env=build_env)
    if not all(path.is_file() for path in wheel_files):
        raise ValueError("Pinned wheel set is incomplete")
    runtime_python = installed_python(args.runtime_source, root / "tts/runtime")
    identity = hashlib.sha256("".join(digest(path) for path in wheel_files).encode()).hexdigest()
    runtime_receipt = runtime_python.parent.parent / "auralis-components.json"
    wanted_components = {"version": 1, "wheels": {path.name: digest(path) for path in wheel_files}}
    if not runtime_receipt.is_file() or json.loads(runtime_receipt.read_text(encoding="utf-8")) != wanted_components:
        subprocess.run([str(runtime_python), "-m", "pip", "install", "--break-system-packages", "--no-index", "--no-deps",
                        "--force-reinstall", *map(str, wheel_files)], check=True)
        atomic_json(runtime_receipt, wanted_components)
    packages = root / "tts/packages" / identity
    if not (packages / "installation.json").is_file():
        packages.mkdir(parents=True, exist_ok=True)
        subprocess.run([sys.executable, "-m", "pip", "install", "--no-index", "--no-deps", "--upgrade", "--target", str(packages), *map(str, wheel_files)], check=True)
        atomic_json(packages / "installation.json", {"version": 1, "wheels": {path.name: digest(path) for path in wheel_files}})
    sys.path.insert(0, str(packages))
    from voxscript_contract.voices import load_voice_manifest
    voice_set = hashlib.sha256("".join(digest(args.voice_source / voice_id / "voice.json")
                                       for voice_id in VOICE_IDS).encode()).hexdigest()[:16]
    voice_store = root / "tts/voice-stores" / voice_set
    voice_store.mkdir(parents=True, exist_ok=True)
    for voice_id in VOICE_IDS:
        source = args.voice_source / voice_id
        manifest = load_voice_manifest(source / "voice.json")
        if manifest.voice_id != voice_id:
            raise ValueError(f"Voice identity differs: {voice_id}")
        destination = voice_store / voice_id
        if not destination.exists():
            with tempfile.TemporaryDirectory(dir=voice_store, prefix=f".{voice_id}-") as temporary:
                staged = Path(temporary) / voice_id
                staged.mkdir()
                shutil.copy2(source / "voice.json", staged / "voice.json")
                for reference in manifest.references:
                    source_audio = source / reference.audio.path
                    target_audio = staged / reference.audio.path
                    target_audio.parent.mkdir(parents=True, exist_ok=True)
                    if not target_audio.exists():
                        shutil.copy2(source_audio, target_audio)
                    if reference.qualification.report is not None:
                        report_source = source / reference.qualification.report.path
                        report_target = staged / reference.qualification.report.path
                        report_target.parent.mkdir(parents=True, exist_ok=True)
                        if not report_target.exists():
                            shutil.copy2(report_source, report_target)
                load_voice_manifest(staged / "voice.json")
                staged.rename(destination)
        installed = load_voice_manifest(destination / "voice.json")
        if installed.voice_id != voice_id or digest(destination / "voice.json") != digest(source / "voice.json"):
            raise ValueError(f"Installed voice differs from pinned source: {voice_id}")
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
        for name in NATIVE_PACKAGES:
            distribution = importlib.metadata.distribution(name)
            metadata = runtime_python.parent.parent / "venv/Lib/site-packages" / distribution._path.name / "METADATA"
            dependencies.append({"name": name, "version": distribution.version, "path": str(metadata), "sha256": digest(metadata)})
        atomic_json(integrity_path, {"version": 1, "python": sys.version, "package_files": package_files, "model_files": model_files, "dependencies": dependencies})
    config = {
        "version": 1,
        "python_executable": str(runtime_python),
        "package_root": str(packages),
        "voice_store": str(voice_store),
        "profile_root": str(profiles),
        "model_cache": str(args.model_cache.resolve()),
        "voxcpm_revision": VOX_REVISION,
        "integrity_sha256": digest(integrity_path),
    }
    atomic_json(root / "tts-runtime.json", config)
    atomic_json(root / "tts/receipt.json", {"version": 1, "tts_commit": TTS_COMMIT, "contract_commit": CONTRACT_COMMIT, "caster_commit": CASTER_COMMIT, "voice_ids": list(VOICE_IDS), "installation_id": identity, "wheels": {path.name: digest(path) for path in wheel_files}, "runtime": config})
    print(f"Installed local TTS runtime: {root / 'tts-runtime.json'}")
    print("Voice copies retain their real qualification state. No model was loaded.")


if __name__ == "__main__":
    main()
