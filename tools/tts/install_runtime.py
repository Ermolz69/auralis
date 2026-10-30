"""Install a locally pinned Windows CUDA wheel closure without loading models."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
from email.parser import BytesParser
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import zipfile


HEX = re.compile(r"[0-9a-f]{64}\Z")
WHEEL_NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.+-]*\.whl\Z")
REQUIRED = {"torch": "2.8.0+cu128", "torchaudio": "2.8.0+cu128",
            "voxcpm": "2.0.3", "f5-tts": "1.1.21",
            "accelerate": "1.14.0", "safetensors": "0.8.0",
            "huggingface-hub": "1.29.0", "ruaccent": "1.5.8.3",
            "jsonschema": "4.26.0", "portalocker": "4.4.0",
            "psutil": "7.2.2", "aiohttp": "3.14.1", "rich": "13.9.4",
            "puza-tts": "0.3.0", "voxscript-contract": "0.2.0"}
COMPONENT_PINS = {
    "puza_tts-0.3.0-py3-none-any.whl": (57290, "1b0fa4ea0e7b8f3e055194edf202bb24ff826e1a54bcf354f5036ebbd9a3cca6"),
    "voxscript_contract-0.2.0-py3-none-any.whl": (75499, "c8dcb3ecfbd64f821c56347ee8df039c8bff1d486c8d29f5967ac26a270e5381"),
}


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def pinned_file(root: Path, entry: dict) -> Path:
    if set(entry) != {"filename", "size_bytes", "sha256"}:
        raise ValueError("File pin must contain filename, size_bytes and sha256")
    name, size, expected = entry["filename"], entry["size_bytes"], entry["sha256"]
    if (not isinstance(name, str) or name in {".", ".."} or
            Path(name).name != name or "/" in name or "\\" in name or ":" in name or
            not isinstance(size, int) or isinstance(size, bool) or size <= 0 or
            not isinstance(expected, str) or not HEX.fullmatch(expected)):
        raise ValueError("Invalid pinned file identity")
    file = root / name
    if file.is_symlink() or not file.is_file() or file.stat().st_size != size or sha256(file) != expected:
        raise ValueError(f"Pinned file differs: {name}")
    return file


def inspect_wheel(path: Path, supported_tags: set) -> tuple[str, str]:
    from pip._vendor.packaging.utils import parse_wheel_filename
    from pip._vendor.packaging.utils import canonicalize_name

    if not WHEEL_NAME.fullmatch(path.name):
        raise ValueError(f"Invalid wheel filename: {path.name}")
    name, version, _, tags = parse_wheel_filename(path.name)
    if not tags & supported_tags:
        raise ValueError(f"Wheel is incompatible with CPython/Windows: {path.name}")
    with zipfile.ZipFile(path) as archive:
        members = archive.namelist()
        if len(members) != len(set(members)) or any(
            item.startswith("/") or "\\" in item or ":" in item or
            any(part in {"", ".", ".."} for part in item.rstrip("/").split("/"))
            for item in members
        ) or any(stat.S_IFMT(info.external_attr >> 16) == stat.S_IFLNK
                 for info in archive.infolist()):
            raise ValueError(f"Unsafe wheel entries: {path.name}")
        # Distribution spelling may differ while its canonical name remains equal.
        files = {item for item in members if not item.endswith("/")}
        infos = [item for item in files if item.count("/") == 1 and item.endswith(".dist-info/METADATA")]
        if len(infos) != 1:
            raise ValueError(f"Wheel metadata is incomplete: {path.name}")
        prefix = infos[0][:-len("METADATA")]
        if prefix + "WHEEL" not in files or prefix + "RECORD" not in files:
            raise ValueError(f"Wheel control files are incomplete: {path.name}")
        metadata = BytesParser().parsebytes(archive.read(prefix + "METADATA"))
        if canonicalize_name(metadata["Name"] or "") != name or metadata["Version"] != str(version):
            raise ValueError(f"Wheel identity differs: {path.name}")
        import base64
        import csv
        import io
        records = list(csv.reader(io.StringIO(archive.read(prefix + "RECORD").decode("utf-8"))))
        if len(records) != len(files) or {row[0] for row in records} != files:
            raise ValueError(f"Wheel RECORD coverage differs: {path.name}")
        for record in records:
            if len(record) != 3:
                raise ValueError(f"Malformed wheel RECORD: {path.name}")
            filename, digest, size = record
            if filename == prefix + "RECORD":
                if digest or size:
                    raise ValueError(f"Wheel RECORD self entry differs: {path.name}")
                continue
            checksum = hashlib.sha256()
            length = 0
            with archive.open(filename) as stream:
                while block := stream.read(1024 * 1024):
                    length += len(block)
                    checksum.update(block)
            expected = "sha256=" + base64.urlsafe_b64encode(checksum.digest()).rstrip(b"=").decode()
            if digest != expected or size != str(length):
                raise ValueError(f"Wheel RECORD integrity differs: {path.name}")
    return name, str(version)


def verify_inputs(manifest: dict, wheelhouse: Path, tools: Path, supported_tags: set) -> list[Path]:
    if set(manifest) != {"version", "platform", "wheels", "windows_tools", "source_archives"} or manifest["version"] != 1 or manifest["platform"] != "cp312-win_amd64":
        raise ValueError("Unsupported Windows runtime manifest")
    pins = manifest["wheels"]
    if not isinstance(pins, list) or not pins or len(pins) > 512:
        raise ValueError("Expected a bounded complete wheel set")
    names = set()
    versions = {}
    files = []
    for pin in pins:
        file = pinned_file(wheelhouse, pin)
        if file.name in names:
            raise ValueError("Duplicate wheel filename")
        names.add(file.name)
        name, version = inspect_wheel(file, supported_tags)
        if name in versions:
            raise ValueError(f"Multiple versions of {name}")
        versions[name] = version
        files.append(file)
    for name, version in REQUIRED.items():
        if versions.get(name) != version:
            raise ValueError(f"Required wheel {name}=={version} is absent")
    for filename, (size, digest) in COMPONENT_PINS.items():
        if not any(pin["filename"] == filename and pin["size_bytes"] == size and
                   pin["sha256"] == digest for pin in pins):
            raise ValueError(f"Reviewed component wheel differs: {filename}")
    if set(manifest["windows_tools"]) != {"ffmpeg", "ffprobe"}:
        raise ValueError("Pin both Windows media tools")
    for name, pin in manifest["windows_tools"].items():
        if pin.get("filename", "").lower() != name + ".exe":
            raise ValueError(f"Invalid {name} executable pin")
        pinned_file(tools, pin)
    if not isinstance(manifest["source_archives"], list):
        raise ValueError("Invalid source archive inventory")
    for pin in manifest["source_archives"]:
        if not pin.get("filename", "").endswith(".tar.gz"):
            raise ValueError("Expected a pinned source archive")
        pinned_file(wheelhouse, pin)
    return files


def capture(args: argparse.Namespace) -> Path:
    """Make a local candidate inventory; review its source before installing."""
    if sys.platform != "win32" or sys.version_info[:2] != (3, 12) or sys.maxsize <= 2**32:
        raise ValueError("Use 64-bit native Windows CPython 3.12 to capture compatible wheels")
    wheelhouse = args.wheelhouse.resolve(strict=True)
    tools = args.windows_tools.resolve(strict=True)
    output = args.output.resolve()
    if output.exists():
        raise FileExistsError(f"Candidate manifest already exists: {output}")
    wheels = sorted(wheelhouse.glob("*.whl"))
    if not wheels:
        raise ValueError("Wheelhouse contains no wheels")
    pin = lambda file: {"filename": file.name, "size_bytes": file.stat().st_size, "sha256": sha256(file)}
    manifest = {"version": 1, "platform": "cp312-win_amd64",
                "wheels": [pin(file) for file in wheels],
                "source_archives": [pin(file) for file in sorted(wheelhouse.glob("*.tar.gz"))],
                "windows_tools": {name: pin(tools / (name + ".exe"))
                                  for name in ("ffmpeg", "ffprobe")}}
    from pip._vendor.packaging.tags import sys_tags
    verify_inputs(manifest, wheelhouse, tools, set(sys_tags()))
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return output


def run(*args: str, env: dict | None = None) -> str:
    result = subprocess.run(args, text=True, capture_output=True, env=env, timeout=3600)
    if result.returncode:
        raise RuntimeError(f"{Path(args[0]).name} exited {result.returncode}: {result.stderr[-4000:]}")
    return result.stdout.strip()


def install(args: argparse.Namespace) -> Path:
    if sys.platform != "win32":
        raise ValueError("This installer requires native Windows")
    if sys.version_info[:2] != (3, 12) or sys.maxsize <= 2**32:
        raise ValueError("Use 64-bit CPython 3.12 to run the installer")
    for name in ("vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"):
        if not (Path(os.environ["SystemRoot"]) / "System32" / name).is_file():
            raise ValueError(f"Missing Microsoft Visual C++ x64 runtime: {name}")
    from pip._vendor.packaging.tags import sys_tags

    manifest_path = args.manifest.resolve(strict=True)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest_hash = sha256(manifest_path)
    wheelhouse = args.wheelhouse.resolve(strict=True)
    tools = args.windows_tools.resolve(strict=True)
    wheels = verify_inputs(manifest, wheelhouse, tools, set(sys_tags()))
    root = args.data_root.resolve()
    target = root / "tts/runtime" / manifest_hash[:16]
    receipt = target / "runtime-installation.json"
    if target.exists():
        raise FileExistsError(f"Runtime destination already exists; inspect before retry: {target}")
    target.mkdir(parents=True)
    marker = target / ".installing"
    marker.write_text("installation incomplete\n", encoding="utf-8")
    python = target / "venv/Scripts/python.exe"
    clean_env = {key: value for key, value in os.environ.items()
                 if key.upper() not in {"PYTHONHOME", "PYTHONPATH", "PIP_INDEX_URL", "PIP_EXTRA_INDEX_URL"}}
    clean_env.update(PIP_NO_INDEX="1", PYTHONDONTWRITEBYTECODE="1")
    clean_env["NUMBA_CACHE_DIR"] = str(target / "numba-cache")
    run(sys.executable, "-I", "-m", "venv", str(target / "venv"), env=clean_env)
    run(str(python), "-I", "-m", "pip", "install", "--no-index", "--no-deps", "--no-compile",
        *map(str, wheels), env=clean_env)
    run(str(python), "-I", "-m", "pip", "check", env=clean_env)
    probe = '''import importlib.metadata as m, json, sys, torch, torchaudio
import voxcpm, f5_tts, puza_tts, voxscript_contract, soundfile
from pathlib import Path
names = %r
assert sys.version_info[:2] == (3, 12) and sys.maxsize > 2**32
assert all(m.version(k) == v for k, v in names.items())
assert torch.version.cuda == "12.8" and torch.__version__ == torchaudio.__version__ == "2.8.0+cu128"
assert torch.cuda.is_available(), "CUDA device is unavailable"
print(json.dumps({"python": sys.version, "torch": torch.__version__, "torchaudio": torchaudio.__version__, "cuda_runtime": torch.version.cuda, "gpu": torch.cuda.get_device_name(0)}))
''' % REQUIRED
    compatibility = json.loads(run(str(python), "-I", "-c", probe, env=clean_env))
    bin_dir = target / "bin"
    bin_dir.mkdir()
    for pin in manifest["windows_tools"].values():
        shutil.copy2(tools / pin["filename"], bin_dir / pin["filename"])
        pinned_file(bin_dir, pin)
    for name in ("ffmpeg", "ffprobe"):
        run(str(bin_dir / (name + ".exe")), "-version", env=clean_env)
    data = {"version": 1, "installed_at_utc": datetime.now(timezone.utc).isoformat(),
            "status": "verified_local_runtime", "manifest_sha256": manifest_hash,
            "python_executable": str(python), "python_sha256": sha256(python),
            "base_python_sha256": sha256(Path(sys.executable)),
            "wheels": manifest["wheels"], "windows_tools": manifest["windows_tools"],
            "compatibility": compatibility, "models_loaded": False}
    temporary = target / "runtime-installation.json.tmp"
    temporary.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    os.replace(temporary, receipt)
    marker.unlink()
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", action="store_true", help="Write a candidate manifest; do not install")
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--wheelhouse", type=Path, required=True)
    parser.add_argument("--windows-tools", type=Path, required=True)
    parser.add_argument("--data-root", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.capture and (not args.output or args.manifest or args.data_root):
        parser.error("--capture requires --output and excludes --manifest/--data-root")
    if not args.capture and (not args.manifest or not args.data_root or args.output):
        parser.error("installation requires --manifest and --data-root and excludes --output")
    print(capture(args) if args.capture else install(args))


if __name__ == "__main__":
    main()
