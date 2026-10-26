import hashlib
import importlib.metadata
import json
from pathlib import Path
import sys
import tempfile

sys.path.insert(0, sys.argv[1])
from voxscript_contract.api import load_project, load_voice_manifest
from voxscript_contract.json_codec import canonical_json_v1
from voxscript_contract.voice_qualification import require_qualified_reference


def run(request):
    version = importlib.metadata.version("voxscript-contract")
    if version != "0.2.0":
        raise ValueError("CONTRACT 0.2.0 is required")
    source = request.get("source_text")
    if source is not None and (not isinstance(source, str) or not source.strip()):
        raise ValueError("Project source text is empty")
    configuration = {"version": 1, "documents": []}
    cast = {"version": 2, "roles": {"narrator": {"display_name": "Рассказчик", "voice_id": None}}}
    bindings = request.get("bindings", [])
    if len(bindings) > request["max_bindings"]:
        raise ValueError("Too many voice assignments")
    seen = set()
    voices = {}
    for binding in bindings:
        role = binding["role_id"]
        if role in seen:
            raise ValueError("Duplicate voice role")
        seen.add(role)
        voice_id = binding["voice_id"]
        if not voice_id or any(c not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-" for c in voice_id):
            raise ValueError("Invalid voice identity")
        if voice_id not in voices:
            voices[voice_id] = load_voice_manifest(Path(sys.argv[2]) / voice_id / "voice.json")
        voice = voices[voice_id]
        if voice.voice_id != voice_id:
            raise ValueError("Voice manifest identity differs")
        reference = next((item for item in voice.references
                          if item.reference_id == binding["reference_id"]
                          and item.engine_id == binding["engine_id"]), None)
        if reference is None:
            raise ValueError("Voice reference is unavailable for the selected engine")
        technical = (reference.emotion_id == "neutral"
                     and reference.qualification.status == "unverified"
                     and binding.get("technical_reference_permission"))
        if not technical:
            require_qualified_reference(voice, binding["reference_id"], binding["engine_id"], reference.emotion_id)
        cast["roles"][role] = {"display_name": binding["display_name"], "voice_id": voice_id}
    if source is not None:
        configuration["documents"] = [{"document_id": "document-000001", "input": "source.txt"}]
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        (root / "project.json").write_bytes(canonical_json_v1(configuration))
        (root / "cast.json").write_bytes(canonical_json_v1(cast))
        if source is not None:
            (root / "source.txt").write_bytes(source.encode("utf-8"))
        project = load_project(root)
        if len(project.documents) != (1 if source is not None else 0):
            raise ValueError("CONTRACT project coverage differs")
    result = {
        "contract_version": version,
        "project_json": canonical_json_v1(configuration).decode("utf-8"),
        "cast_json": canonical_json_v1(cast).decode("utf-8"),
        "source_text": source,
        "bindings_json": canonical_json_v1(bindings).decode("utf-8"),
    }
    result["sha256"] = hashlib.sha256(canonical_json_v1(result)).hexdigest()
    return result


try:
    request = json.loads(sys.stdin.buffer.read(2_097_153))
    result = run(request)
    print(json.dumps({"request_id": request["request_id"], "project_id": request["project_id"], "result": result}, ensure_ascii=False))
except Exception as error:
    print(str(error), file=sys.stderr)
    raise SystemExit(1)
