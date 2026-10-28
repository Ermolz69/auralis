"""Validate the reviewed component closure before updating active source pins."""

from __future__ import annotations

import argparse
import ast
from email.parser import BytesParser
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib
import zipfile


SOURCES = {
    "tts": ("49662267df9ee467e61a0d4e6a4caf7955f54a7d", "Timbuhtuk/puza-tts", "PUZA_TTS", "0eb71b3933d08c38161177ffafd19644e248a0ed", "puza-tts", "0.3.0"),
    "speech": ("13f4383c7ca8b8b7b575eb64fdf9c54856a9204a", "Timbuhtuk/puza-contract", "VOXSCRIPT_CONTRACT", "deab23554bc79faae0f5e402e1ab0f57adec8096", "voxscript-contract", "0.2.0"),
    "media": ("6ee129cfcf534a7ca865c17ca5fa8195e8a028be", "Timbuhtuk/puza-contract", "VOXSCRIPT_CONTRACT", "74a9d249a6546473e57f0d045ce32a4fc46eeacd", "voxscript-contract", "0.3.0"),
    "caster": ("2dbab64abeb4d3e7eee7bc819a5578a7008b3c5d", "Timbuhtuk/puza-caster", "src", "c9cb634c4671f363dbc069560ea243c6e079b8fc", "puza-voice-editor", "0.2.0"),
}
WHEELS = {
    "tts": ("1b0fa4ea0e7b8f3e055194edf202bb24ff826e1a54bcf354f5036ebbd9a3cca6", 57290),
    "speech": ("c8dcb3ecfbd64f821c56347ee8df039c8bff1d486c8d29f5967ac26a270e5381", 75499),
    "media": ("1d7760d6784fc89a143c7d9fd0c7bf230f6315691a082bfa44823c920d32a20e", 78567),
}
OLD = {"tts": "22dcbb12a37bc1e54d0d8cf7f52adb9668de36a7", "speech": "4ab50d5dffbe611dc7d60d0fe292614f6c47cd8a", "caster": "2dbab64abeb4d3e7eee7bc819a5578a7008b3c5d"}
OLD_MEDIA = ("0cc6898abb149eeb983c767655e5c88c3f3f854a", "b632d82674717533a65f44536307b4bf0313cabc")


def git(root: Path, *args: str) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True, timeout=30)
    if result.returncode or result.stderr.strip():
        raise ValueError("Component Git validation failed")
    return result.stdout.strip()


def origin_repository(url: str) -> str:
    for prefix in ("https://github.com/", "git@github.com:"):
        if url.startswith(prefix):
            return url[len(prefix):].removesuffix(".git").casefold()
    raise ValueError("Component origin is not a documented GitHub repository")


def validate_source(role: str, root: Path, revision: str) -> dict:
    expected, repository, subtree, tree, name, version = SOURCES[role]
    if not re.fullmatch(r"[0-9a-f]{40}", revision) or revision != expected:
        raise ValueError(f"{role}: expected the full reviewed revision")
    if root.is_symlink() or not root.is_dir():
        raise ValueError(f"{role}: expected an ordinary repository root")
    root = root.resolve(strict=True)
    if Path(git(root, "rev-parse", "--show-toplevel")).resolve() != root:
        raise ValueError(f"{role}: supplied path is not the repository root")
    if git(root, "rev-parse", "HEAD") != revision:
        raise ValueError(f"{role}: source revision differs")
    if origin_repository(git(root, "remote", "get-url", "origin")) != repository.casefold():
        raise ValueError(f"{role}: foreign repository origin")
    if git(root, "status", "--porcelain=v1", "--untracked-files=all"):
        raise ValueError(f"{role}: source is dirty")
    if git(root, "rev-parse", f"HEAD:{subtree}") != tree:
        raise ValueError(f"{role}: package tree differs")
    metadata = root / ("pyproject.toml" if role == "caster" else f"{subtree}/pyproject.toml")
    project = tomllib.loads(metadata.read_text(encoding="utf-8"))["project"]
    if project["name"] != name or project["version"] != version:
        raise ValueError(f"{role}: package name or version differs")
    if role == "tts" and "scipy==1.18.1" not in project.get("dependencies", []):
        raise ValueError("tts: required base SciPy dependency is absent")
    return {"revision": revision, "tree": tree, "package": name, "version": version}


def validate_wheel(role: str, path: Path) -> str:
    checksum, size = WHEELS[role]
    if path.is_symlink() or not path.is_file() or path.stat().st_size != size:
        raise ValueError(f"{role}: wheel size or file type differs")
    with path.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha256").hexdigest()
    if actual != checksum:
        raise ValueError(f"{role}: wheel SHA-256 differs")
    with zipfile.ZipFile(path) as archive:
        records = [name for name in archive.namelist() if name.endswith(".dist-info/METADATA")]
        if len(records) != 1:
            raise ValueError(f"{role}: expected one wheel metadata record")
        metadata = BytesParser().parsebytes(archive.read(records[0]))
    if (metadata["Name"], metadata["Version"]) != SOURCES[role][4:6]:
        raise ValueError(f"{role}: wheel package identity differs")
    return actual


def render_setup(source: str) -> str:
    for role, constant in (("tts", "TTS_COMMIT"), ("speech", "CONTRACT_COMMIT"), ("caster", "CASTER_COMMIT")):
        pattern = re.compile(rf'^{constant} = "([0-9a-f]{{40}})"(?=\r?$)', re.MULTILINE)
        matches = list(pattern.finditer(source))
        if len(matches) != 1 or matches[0][1] not in {OLD[role], SOURCES[role][0]}:
            raise ValueError(f"Unexpected active {constant}")
        source = pattern.sub(f'{constant} = "{SOURCES[role][0]}"', source)
    inventories = [node for node in ast.parse(source).body if isinstance(node, ast.Assign)
                   and any(isinstance(target, ast.Name) and target.id == "NATIVE_PACKAGES" for target in node.targets)]
    if len(inventories) != 1:
        raise ValueError("Expected one native dependency inventory")
    inventory = ast.literal_eval(inventories[0].value)
    if not isinstance(inventory, tuple) or len(inventory) != len(set(inventory)) or "soundfile" not in inventory:
        raise ValueError("Invalid native dependency inventory")
    if "scipy" not in inventory:
        lines = source.splitlines(keepends=True)
        start, end = inventories[0].lineno - 1, inventories[0].end_lineno
        fragment = "".join(lines[start:end])
        fragment = fragment.replace('"soundfile",', '"soundfile", "scipy",', 1)
        source = "".join(lines[:start]) + fragment + "".join(lines[end:])
        if "scipy" not in ast.literal_eval(next(node.value for node in ast.parse(source).body
                                               if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "NATIVE_PACKAGES" for t in node.targets))):
            raise ValueError("Cannot add the required SciPy inventory entry")
    return source


def replace_files(changes: dict[Path, bytes]) -> None:
    staged, backups, applied = {}, {}, []
    try:
        for path, data in changes.items():
            for content, table in ((data, staged), (path.read_bytes(), backups)):
                with tempfile.NamedTemporaryFile(prefix=".component-pins-", dir=path.parent, delete=False) as stream:
                    table[path] = Path(stream.name)
                    stream.write(content)
                    stream.flush()
                    os.fsync(stream.fileno())
        for path, temporary in staged.items():
            os.replace(temporary, path)
            applied.append(path)
    except BaseException:
        for path in reversed(applied):
            os.replace(backups[path], path)
        raise
    finally:
        for path in (*staged.values(), *backups.values()):
            path.unlink(missing_ok=True)


def update(root: Path, selections: dict[str, tuple[Path, str]], wheels: dict[str, Path]) -> dict:
    if set(selections) != set(SOURCES) or set(wheels) != set(WHEELS):
        raise ValueError("The complete reviewed component closure is required")
    checked = {role: validate_source(role, *selection) for role, selection in selections.items()}
    hashes = {role: validate_wheel(role, path) for role, path in wheels.items()}
    setup, lock = root / "tools/tts/setup_local.py", root / "tools/tts/media-contract.lock.json"
    if any(path.is_symlink() or not path.is_file() for path in (setup, lock)):
        raise ValueError("Active pin targets must be ordinary existing files")
    original = {path: path.read_bytes() for path in (setup, lock)}
    source = render_setup(original[setup].decode("utf-8"))
    media = json.loads(original[lock])
    if set(media) != {"package_version", "source_commit", "source_tree", "wheel_sha256"} or media["package_version"] != "0.3.0" or media["wheel_sha256"] != WHEELS["media"][0]:
        raise ValueError("Unexpected active media lock shape, version or wheel")
    target = SOURCES["media"][0], SOURCES["media"][3]
    if (media["source_commit"], media["source_tree"]) not in {OLD_MEDIA, target}:
        raise ValueError("Unexpected active media source pin")
    media.update(source_commit=target[0], source_tree=target[1])
    desired = {setup: source.encode("utf-8"), lock: (json.dumps(media, indent=2) + "\n").encode()}
    changes = {path: data for path, data in desired.items() if data != original[path]}
    replace_files(changes)
    return {"version": 1, "updated": [path.relative_to(root).as_posix() for path in changes],
            "components": checked, "wheels": hashes, "models_loaded": False, "runtime_modified": False}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for role in SOURCES:
        parser.add_argument(f"--{role}-root", type=Path, required=True)
        parser.add_argument(f"--{role}-revision", required=True)
    for role in WHEELS:
        parser.add_argument(f"--{role}-wheel", type=Path, required=True)
    args = vars(parser.parse_args())
    selections = {role: (args[f"{role}_root"], args[f"{role}_revision"]) for role in SOURCES}
    wheels = {role: args[f"{role}_wheel"] for role in WHEELS}
    print(json.dumps(update(Path(__file__).resolve().parents[2], selections, wheels), indent=2))


if __name__ == "__main__":
    main()
