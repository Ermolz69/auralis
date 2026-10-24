"""Install the media CONTRACT alongside the unchanged, pinned speech runtime."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract-root", type=Path, required=True)
    parser.add_argument("--data-root", type=Path, default=Path(os.environ["APPDATA"]) / "com.auralis.desktop")
    args = parser.parse_args()
    source = args.contract_root.resolve()
    lock = json.loads(Path(__file__).with_name("media-contract.lock.json").read_text(encoding="utf-8"))
    tree = subprocess.check_output(["git", "-c", f"safe.directory={source.as_posix()}", "-C", str(source), "rev-parse", "HEAD:VOXSCRIPT_CONTRACT"], text=True).strip()
    if tree != lock["source_tree"]:
        raise SystemExit(f"Expected frozen CONTRACT source from {lock['source_commit']}")
    revision = subprocess.check_output(["git", "-c", f"safe.directory={source.as_posix()}", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    changes = subprocess.check_output(["git", "-c", f"safe.directory={source.as_posix()}", "-C", str(source), "status", "--porcelain", "--", "VOXSCRIPT_CONTRACT"], text=True).strip()
    if changes:
        raise SystemExit("Commit CONTRACT source before installing a reproducible runtime")
    root = args.data_root.resolve() / "project-contract"
    wheels = root / "wheels"
    wheels.mkdir(parents=True, exist_ok=True)
    subprocess.run([sys.executable, "-m", "pip", "wheel", "--no-deps", "--no-build-isolation", "--wheel-dir", str(wheels), str(source / "VOXSCRIPT_CONTRACT")], check=True, env=dict(os.environ, SOURCE_DATE_EPOCH="1792800000"))
    wheel = wheels / "voxscript_contract-0.3.0-py3-none-any.whl"
    checksum = digest(wheel)
    if checksum != lock["wheel_sha256"]:
        raise SystemExit("CONTRACT wheel differs from the pinned SHA-256")
    package = root / "packages" / checksum
    if not package.exists():
        subprocess.run([sys.executable, "-m", "pip", "install", "--no-deps", "--no-index", "--target", str(package), str(wheel)], check=True)
    receipt = dict(version=1, python_executable=sys.executable, package_root=str(package), package_version="0.3.0",
                   files=[dict(path=p.relative_to(package).as_posix(), sha256=digest(p)) for p in sorted(package.rglob("*")) if p.is_file() and p.suffix != ".pyc" and "__pycache__" not in p.parts])
    target = args.data_root / "project-runtime.json"
    temporary = target.with_suffix(".pending")
    temporary.write_text(json.dumps(receipt, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(temporary, target)
    revision = subprocess.check_output(["git", "-c", f"safe.directory={source.as_posix()}", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    (root / "receipt.json").write_text(json.dumps(dict(commit=revision, wheel=str(wheel), sha256=checksum), indent=2), encoding="utf-8")
    print(f"Media CONTRACT installed: {target}; wheel SHA-256 {checksum}")


if __name__ == "__main__":
    main()
