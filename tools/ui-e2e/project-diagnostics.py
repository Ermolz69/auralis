import hashlib
import importlib.metadata
import json
from pathlib import Path
import sys

profile = Path(sys.argv[1])
config = json.loads((profile / "tts-runtime.json").read_text(encoding="utf-8"))
package = Path(config["package_root"])
inventory_file = package / "integrity.json"
assert hashlib.sha256(inventory_file.read_bytes()).hexdigest() == config["integrity_sha256"]
inventory = json.loads(inventory_file.read_text(encoding="utf-8"))
for item in inventory["package_files"]:
    assert hashlib.sha256((package / item["path"]).read_bytes()).hexdigest() == item["sha256"], item["path"]
for item in inventory["dependencies"]:
    assert hashlib.sha256(Path(item["path"]).read_bytes()).hexdigest() == item["sha256"], item["path"]
sys.path.insert(0, str(package))
from voxscript_contract.api import load_project, load_voice_manifest
from voxscript_contract.json_codec import canonical_json_v1
from voxscript_contract.voice_qualification import require_qualified_reference
print(json.dumps({"operation": "read-only imports and hashes", "contract_version": importlib.metadata.version("voxscript-contract"), "model_inference": False}))
