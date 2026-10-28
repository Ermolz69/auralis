import argparse
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parent))
from component_wheels import configuration, validate_wheel
from packaging.requirements import Requirement
from packaging.utils import canonicalize_name

EPOCH = "1791639837"


def git(root, *arguments):
    return subprocess.check_output(["git", "-C", str(root), *arguments], text=True).strip()


def source_identity(root, revision):
    if not re.fullmatch(r"[a-f0-9]{40}", revision):
        raise ValueError("Pin a full component source revision")
    root = root.resolve(strict=True)
    if Path(git(root, "rev-parse", "--show-toplevel")).resolve() != root:
        raise ValueError("Component source must be an explicit repository root")
    if git(root, "rev-parse", "HEAD") != revision:
        raise ValueError("Component source revision differs")
    if git(root, "status", "--porcelain", "--untracked-files=normal"):
        raise ValueError("Component source must be clean")
    return root


def snapshot(root, revision, package, destination):
    modes = git(root, "ls-tree", "-r", revision, "--", package)
    if not modes or any(line.split()[0] not in {"100644", "100755"} for line in modes.splitlines()):
        raise ValueError("Component snapshot requires regular tracked source files")
    archive = destination / (package + ".zip")
    subprocess.run(
        ["git", "-C", str(root), "archive", "--format=zip", f"--output={archive}", revision, package],
        check=True,
    )
    with zipfile.ZipFile(archive) as entries:
        if any(name.startswith("/") or ".." in name.split("/") for name in entries.namelist()):
            raise ValueError("Invalid component snapshot path")
        entries.extractall(destination)
    return destination / package


def sha256(file):
    value = hashlib.sha256()
    with file.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def build(args):
    if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]{0,120}", args.id):
        raise ValueError("Invalid component build identity")
    if not re.fullmatch(r"[a-f0-9]{64}", args.tts_wheel_sha256):
        raise ValueError("Pin the complete TTS wheel SHA-256")
    root = args.worktree_root.resolve(strict=True)
    destination = root / ".cache/ui-e2e/wheels" / args.id
    if not destination.parent.resolve().is_relative_to(root):
        raise ValueError("Component outputs must remain in the owned worktree")
    if destination.exists():
        raise ValueError("Retained component builds cannot be overwritten")
    contract = source_identity(args.contract_root, args.contract_commit)
    tts = source_identity(args.tts_root, args.tts_commit)
    wheel = args.tts_wheel.resolve(strict=True)
    if args.tts_wheel.is_symlink() or not wheel.is_file() or wheel.stat().st_size > 64 * 1024 * 1024:
        raise ValueError("Expected a bounded regular TTS wheel")
    if sha256(wheel) != args.tts_wheel_sha256:
        raise ValueError("TTS wheel SHA-256 differs")
    tts_project = validate_wheel(tts / "PUZA_TTS", wheel, "puza-tts")
    destination.mkdir(parents=True, exist_ok=False)
    staging = destination / ".build"
    staging.mkdir()
    source = snapshot(contract, args.contract_commit, "VOXSCRIPT_CONTRACT", staging)
    configuration(source, "voxscript-contract")
    environment = {
        **os.environ,
        "SOURCE_DATE_EPOCH": EPOCH,
        "PYTHONHASHSEED": "0",
        "UV_CACHE_DIR": str(staging / "uv-cache"),
        "UV_PYTHON_DOWNLOADS": "never",
        "UV_LINK_MODE": "copy",
        "PYTHONDONTWRITEBYTECODE": "1",
    }
    subprocess.run(
        [args.uv, "build", "--offline", "--wheel", "--no-build-isolation", "--python",
         sys.executable, "--out-dir", str(destination), str(source)],
        env=environment,
        check=True,
    )
    built = list(destination.glob("*.whl"))
    if len(built) != 1:
        raise ValueError("Expected exactly one built CONTRACT wheel")
    contract_project = validate_wheel(source, built[0], "voxscript-contract")
    binding = [
        Requirement(value) for value in tts_project["dependencies"]
        if canonicalize_name(Requirement(value).name) == "voxscript-contract"
    ]
    if len(binding) != 1 or contract_project["version"] not in binding[0].specifier:
        raise ValueError("CONTRACT version does not satisfy the verified TTS wheel")
    with wheel.open("rb") as origin, (destination / wheel.name).open("xb") as output:
        shutil.copyfileobj(origin, output)
    if sha256(destination / wheel.name) != args.tts_wheel_sha256:
        raise ValueError("TTS wheel changed while retaining verified inputs")
    source_identity(contract, args.contract_commit)
    source_identity(tts, args.tts_commit)
    files = [
        dict(path=f".cache/ui-e2e/wheels/{args.id}/{file.name}",
             size_bytes=file.stat().st_size, sha256=sha256(file))
        for file in sorted(destination.glob("*.whl"))
    ]
    receipt = dict(
        version=2,
        contract_commit=args.contract_commit,
        tts_commit=args.tts_commit,
        contract_tree=git(contract, "rev-parse", f"{args.contract_commit}:VOXSCRIPT_CONTRACT"),
        tts_tree=git(tts, "rev-parse", f"{args.tts_commit}:PUZA_TTS"),
        contract_version=contract_project["version"],
        source_date_epoch=int(EPOCH),
        python=sys.version.split()[0],
        setuptools=importlib.metadata.version("setuptools"),
        files=files,
        models_loaded=False,
        voices_modified=False,
    )
    pending = destination / "components.pending.json"
    with pending.open("x", encoding="utf-8") as output:
        json.dump(receipt, output, indent=2)
        output.write("\n")
        output.flush()
        os.fsync(output.fileno())
    os.link(pending, destination / "components.json")
    pending.unlink()
    print("Built and retained two verified offline component wheels; no models or voice modifications")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    for option in ["id", "uv", "contract-commit", "tts-commit", "tts-wheel-sha256"]:
        parser.add_argument("--" + option, required=True)
    for option in ["worktree-root", "contract-root", "tts-root", "tts-wheel"]:
        parser.add_argument("--" + option, type=Path, required=True)
    try:
        build(parser.parse_args())
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        print(f"Component build rejected: {error}", file=sys.stderr)
        sys.exit(1)
