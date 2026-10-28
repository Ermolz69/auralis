"""Prepare an isolated wheel builder from an existing CPython interpreter."""
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys


def main():
    if os.name != "nt":
        raise ValueError("The private component builder supports Windows")
    if platform.python_implementation() != "CPython" or sys.version_info[:2] != (3, 12):
        raise ValueError("Select an existing CPython 3.12 interpreter")
    if len(sys.argv) != 2 or not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]{0,120}", sys.argv[1]):
        raise ValueError("Select a safe private build identity")
    root = Path(__file__).resolve().parents[2]
    target = root / ".cache/ui-e2e/build-python" / sys.argv[1]
    if not target.parent.resolve().is_relative_to(root):
        raise ValueError("Build interpreter must remain in the owned worktree")
    target.mkdir(parents=True, exist_ok=False)
    temporary = target / ".prepare"
    temporary.mkdir()
    environment = {key: value for key, value in os.environ.items() if not key.startswith("PIP_")}
    environment.update(TEMP=str(temporary), TMP=str(temporary), PIP_CONFIG_FILE=os.devnull,
                       PIP_CACHE_DIR=str(temporary / "pip-cache"), PYTHONDONTWRITEBYTECODE="1")
    subprocess.run([sys.executable, "-I", "-m", "venv", str(target)], check=True, env=environment)
    python = target / "Scripts/python.exe"
    pins = {"setuptools": "84.0.0", "wheel": "0.45.1", "packaging": "26.3"}
    subprocess.run([str(python), "-I", "-m", "pip", "install", "--disable-pip-version-check",
                    "--no-input", *[f"{name}=={version}" for name, version in pins.items()]],
                   check=True, env=environment)
    installed = {item.metadata["Name"]: item.version for item in importlib.metadata.distributions(
        path=[str(target / "Lib/site-packages")])}
    if any(installed.get(name) != version for name, version in pins.items()) or "pip" not in installed:
        raise ValueError("Private build tooling does not match the complete pins")
    receipt = dict(version=1, python=sys.version.split()[0], packages=installed,
                   directory=target.relative_to(root).as_posix(), models_loaded=False)
    with (target / "build-python.json").open("x", encoding="utf-8") as output:
        json.dump(receipt, output, indent=2)
    print(f"Prepared private CPython 3.12 builder {sys.argv[1]} with pinned model-free tools")


if __name__ == "__main__":
    main()
