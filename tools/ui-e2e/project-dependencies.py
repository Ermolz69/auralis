import importlib.metadata
import json
from pathlib import Path
import re
import sys
from packaging.requirements import Requirement

source = Path(sys.argv[1]).resolve()
config = json.loads((source / "tts-runtime.json").read_text(encoding="utf-8"))
runtime = Path(config["python_executable"]).parent.parent
normalize = lambda name: re.sub(r"[-_.]+", "-", name).lower()
distributions = {normalize(item.metadata["Name"]): item for item in importlib.metadata.distributions(path=[str(Path(config["package_root"])), str(runtime / "venv/Lib/site-packages")])}
pending = ["voxscript-contract"]
seen = set()
files = set()
versions = {}
while pending:
    name = pending.pop()
    if name in seen:
        continue
    if name in {"torch", "torchaudio", "transformers", "voxcpm", "vocos", "numpy", "huggingface-hub"}:
        raise RuntimeError(f"Engine dependency is forbidden in the metadata seed: {name}")
    distribution = distributions[name]
    seen.add(name)
    versions[name] = distribution.version
    for requirement in distribution.requires or []:
        requirement = Requirement(requirement)
        if requirement.marker is None or requirement.marker.evaluate({"extra": ""}):
            pending.append(normalize(requirement.name))
    for file in distribution.files or []:
        resolved = Path(distribution.locate_file(file)).resolve()
        if resolved.is_file():
            files.add(resolved.relative_to(source).as_posix())
print(json.dumps({"versions": versions, "files": sorted(files)}))
