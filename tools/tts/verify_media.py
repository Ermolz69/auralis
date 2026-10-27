"""Read-only audit of ready native media projects and independent copies."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import sys

parser = argparse.ArgumentParser()
parser.add_argument("data_root", type=Path)
args = parser.parse_args()
root = args.data_root.resolve()
runtime = json.loads((root / "project-runtime.json").read_text(encoding="utf-8"))
package = Path(runtime["package_root"])
for item in runtime["files"]:
    file = package / item["path"]
    with file.open("rb") as stream:
        assert hashlib.file_digest(stream, "sha256").hexdigest() == item["sha256"], file
sys.path.insert(0, str(package))
from voxscript_contract.media_project import validate_media_project

with sqlite3.connect((root / "auralis.sqlite").as_uri() + "?mode=ro", uri=True) as database:
    assert database.execute("PRAGMA user_version").fetchone()[0] == 22
    receipts = database.execute("SELECT project_id,manifest_json,manifest_sha256 FROM media_project_receipts WHERE state='ready'").fetchall()
    assets = {}
    for project, raw, checksum in receipts:
        project_root = root / "projects" / project
        manifest_file = project_root / "auralis-project.json"
        assert hashlib.sha256(manifest_file.read_bytes()).hexdigest() == checksum, project
        manifest = json.loads(raw)
        assert json.loads(manifest_file.read_bytes()) == manifest, project
        validate_media_project(project_root)
        for asset in manifest["assets"]:
            assets[asset["id"]] = (project_root / asset["path"], asset)
    copies = []
    for file, asset in assets.values():
        source = assets.get(asset["source_asset_id"])
        if source is None:
            continue
        original, origin = source
        assert original != file
        assert original.stat().st_ino != file.stat().st_ino, "Copy shares its file identity"
        assert asset["source_sha256"] == origin["sha256"] == asset["sha256"]
        copies.append(dict(asset=asset["id"], source=asset["source_asset_id"], bytes=asset["bytes"], sha256=asset["sha256"]))
    print(json.dumps(dict(schema=22, ready_projects=len(receipts), independent_copies=copies), indent=2))
