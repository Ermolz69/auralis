"""Stage and install a self-contained pinned Auralis speech bundle on Windows."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def base_home(runtime: Path) -> None:
    config = runtime / "venv/pyvenv.cfg"
    lines = config.read_text(encoding="utf-8").splitlines()
    if sum(line.lower().startswith("home =") for line in lines) != 1:
        raise ValueError("Pinned venv has no unique base Python")
    config.write_text("\n".join(
        f"home = {runtime / 'base'}" if line.lower().startswith("home =") else line
        for line in lines
    ) + "\n", encoding="utf-8")


def all_files(root: Path) -> list[Path]:
    return sorted((path for path in root.rglob("*") if path.is_file()),
                  key=lambda path: path.relative_to(root).as_posix())


def stage(source_root: Path, bundle: Path) -> None:
    source_root = source_root.resolve(strict=True)
    if bundle.exists():
        raise FileExistsError(f"Bundle destination already exists: {bundle}")
    config = json.loads((source_root / "tts-runtime.json").read_text(encoding="utf-8"))
    receipt = json.loads((source_root / "tts/receipt.json").read_text(encoding="utf-8"))
    source_python = Path(config["python_executable"])
    runtime = source_python.parent.parent if source_python.parent.name == "base" else source_python.parents[2]
    packages = Path(config["package_root"])
    voices = Path(config["voice_store"])
    if not all(path.is_relative_to(source_root) for path in (runtime, packages, voices)):
        raise ValueError("The installed source does not own its runtime, packages, and voices")
    source_integrity = json.loads((packages / "integrity.json").read_text(encoding="utf-8"))
    wheels = source_root / "tts/wheels" / hashlib.sha256(
        (receipt["tts_commit"] + receipt["contract_commit"] + receipt["caster_commit"]).encode()
    ).hexdigest()[:16]
    bundle.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=bundle.parent, prefix=".tts-bundle-") as temporary:
        staged = Path(temporary) / bundle.name
        staged.mkdir()
        for source, relative in [
            (runtime, Path("runtime") / runtime.name),
            (packages, Path("packages") / packages.name),
            (voices, Path("voice-stores") / voices.name),
            (wheels, Path("wheels") / wheels.name),
        ]:
            shutil.copytree(source, staged / relative, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        bundled_runtime = staged / "runtime" / runtime.name
        base_home(bundled_runtime)
        for item in source_integrity["model_files"]:
            relative = Path(item["path"])
            source = Path(config["model_cache"]) / relative
            if digest(source) != item["sha256"]:
                raise ValueError(f"Pinned model asset changed: {relative}")
            target = staged / "model-cache" / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        shutil.copy2(Path(__file__), staged / "install.py")
        (staged / "install.ps1").write_text(
            "param([Parameter(Mandatory=$true)][string]$DataRoot)\n"
            "$ErrorActionPreference = 'Stop'\n"
            "$bundle = Split-Path -Parent $MyInvocation.MyCommand.Path\n"
            "$manifest = Get-Content -LiteralPath (Join-Path $bundle 'bundle.json') -Raw | ConvertFrom-Json\n"
            "$python = Join-Path $bundle ('runtime/' + $manifest.runtime_id + '/base/python.exe')\n"
            "& $python -I (Join-Path $bundle 'install.py') install --bundle $bundle --data-root $DataRoot\n"
            "if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n",
            encoding="utf-8",
        )
        runtime_python = bundled_runtime / "base/python.exe"
        subprocess.run([str(runtime_python), "-I", "-c",
                        "import sys, pathlib, torch, torchaudio, voxcpm, f5_tts, voxscript_contract, puza_tts; "
                        "r=pathlib.Path(sys.argv[1]).resolve(); "
                        "assert pathlib.Path(sys.base_prefix).resolve()==r; "
                        "assert pathlib.Path(torch.__file__).resolve().is_relative_to(r.parent)",
                        str(bundled_runtime / "base")],
                       check=True, env={key: value for key, value in os.environ.items()
                                        if key not in {"PYTHONPATH", "PYTHONHOME"}})
        entries = [{"path": path.relative_to(staged).as_posix(), "size": path.stat().st_size,
                    "sha256": digest(path)} for path in all_files(staged)]
        manifest = {
            "version": 1,
            "source_receipt": receipt,
            "runtime_id": runtime.name,
            "package_id": packages.name,
            "voice_set": voices.name,
            "wheel_set": wheels.name,
            "model_files": source_integrity["model_files"],
            "dependency_relative_paths": {
                item["name"]: str(Path(item["path"]).relative_to(runtime))
                for item in source_integrity["dependencies"]
            },
            "files": entries,
        }
        write_json(staged / "bundle.json", manifest)
        staged.rename(bundle)
    base_home(bundle / "runtime" / runtime.name)
    config_path = bundle / "runtime" / runtime.name / "venv/pyvenv.cfg"
    for item in manifest["files"]:
        if item["path"] == config_path.relative_to(bundle).as_posix():
            item["size"] = config_path.stat().st_size
            item["sha256"] = digest(config_path)
            break
    else:
        raise ValueError("Bundled Python configuration is absent from file manifest")
    write_json(bundle / "bundle.json", manifest)
    print(f"Staged standalone speech bundle with {len(entries)} hashed files: {bundle}")


def install(bundle: Path, data_root: Path) -> None:
    bundle = bundle.resolve(strict=True)
    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    if manifest["version"] != 1:
        raise ValueError("Unsupported speech bundle format")
    for item in manifest["files"]:
        source = bundle / item["path"]
        if not source.is_file() or source.stat().st_size != item["size"] or digest(source) != item["sha256"]:
            raise ValueError(f"Speech bundle file changed: {item['path']}")
    data_root.mkdir(parents=True, exist_ok=True)
    target_roots = {
        "runtime": data_root / "tts/runtime" / manifest["runtime_id"],
        "packages": data_root / "tts/packages" / manifest["package_id"],
        "voice-stores": data_root / "tts/voice-stores" / manifest["voice_set"],
        "wheels": data_root / "tts/wheels" / manifest["wheel_set"],
        "model-cache": data_root / "tts/model-cache",
    }
    for kind, target in target_roots.items():
        marker = target.parent / f".{target.name}.installing"
        if marker.exists():
            if not target.resolve().is_relative_to(data_root.resolve()):
                raise ValueError("Partial installation escaped the selected data root")
            if target.exists():
                shutil.rmtree(target)
            marker.unlink()
        if not target.exists():
            target.parent.mkdir(parents=True, exist_ok=True)
            marker.write_text("copying\n", encoding="utf-8")
            source = bundle / kind
            if kind != "model-cache":
                source = source / target.name
            shutil.copytree(source, target)
            marker.unlink()
    runtime = target_roots["runtime"]
    base_home(runtime)
    python = runtime / "base/python.exe"
    bridge = runtime / "base/Lib/site-packages/auralis-native-venv.pth"
    bridge.write_text("../../../venv/Lib/site-packages\n", encoding="utf-8")
    wheels = sorted(target_roots["wheels"].glob("*.whl"))
    if len(wheels) != 2:
        raise ValueError("Standalone bundle has no complete component wheel set")
    wheel_receipt = {"version": 1, "wheels": {path.name: digest(path) for path in wheels}}
    component_receipt = runtime / "auralis-components.json"
    if not component_receipt.is_file() or json.loads(component_receipt.read_text(encoding="utf-8")) != wheel_receipt:
        subprocess.run([str(python), "-m", "pip", "install", "--break-system-packages", "--no-index", "--no-deps",
                        "--force-reinstall", *map(str, wheels)], check=True)
        write_json(component_receipt, wheel_receipt)
    packages = target_roots["packages"]
    integrity_path = packages / "integrity.json"
    integrity = json.loads(integrity_path.read_text(encoding="utf-8"))
    for item in integrity["dependencies"]:
        item["path"] = str(runtime / manifest["dependency_relative_paths"][item["name"]])
        if digest(Path(item["path"])) != item["sha256"]:
            raise ValueError(f"Installed native package changed: {item['name']}")
    for item in integrity["model_files"]:
        if digest(target_roots["model-cache"] / item["path"]) != item["sha256"]:
            raise ValueError(f"Installed model asset changed: {item['path']}")
    write_json(integrity_path, integrity)
    subprocess.run([str(python), "-I", "-c",
                    "import sys, pathlib, torch, torchaudio, voxcpm, f5_tts, voxscript_contract, puza_tts; "
                    "r=pathlib.Path(sys.argv[1]).resolve(); "
                    "assert pathlib.Path(sys.base_prefix).resolve()==r; "
                    "assert pathlib.Path(torch.__file__).resolve().is_relative_to(r.parent)",
                    str(runtime / "base")],
                   check=True, env={key: value for key, value in os.environ.items()
                                    if key not in {"PYTHONPATH", "PYTHONHOME"}})
    config = {
        "version": 1,
        "python_executable": str(python),
        "package_root": str(packages),
        "voice_store": str(target_roots["voice-stores"]),
        "profile_root": str(packages / "share/puza-tts/profiles"),
        "model_cache": str(target_roots["model-cache"]),
        "voxcpm_revision": "32279effe8c19989596f05d353d1447f51d9e915",
        "integrity_sha256": digest(integrity_path),
    }
    write_json(data_root / "tts-runtime.json", config)
    receipt = dict(manifest["source_receipt"])
    receipt["runtime"] = config
    receipt["bundle_sha256"] = digest(bundle / "bundle.json")
    write_json(data_root / "tts/receipt.json", receipt)
    print(f"Installed standalone speech runtime: {data_root / 'tts-runtime.json'}")


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    prepare = sub.add_parser("stage")
    prepare.add_argument("--source-root", type=Path, default=Path(os.environ["APPDATA"]) / "com.auralis.desktop")
    prepare.add_argument("--bundle", type=Path, required=True)
    deploy = sub.add_parser("install")
    deploy.add_argument("--bundle", type=Path, default=Path(__file__).resolve().parent)
    deploy.add_argument("--data-root", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "stage":
        stage(args.source_root, args.bundle)
    else:
        install(args.bundle, args.data_root.resolve())


if __name__ == "__main__":
    main()
