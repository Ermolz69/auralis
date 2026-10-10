"""Synthetic process responses for host framing and owned-tree acceptance."""

import json
import subprocess
import sys
import time
from pathlib import Path


pin = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
mode = sys.argv[2]
pid_file = Path(sys.argv[3]) if len(sys.argv) > 3 else None
sys.stderr.write("synthetic diagnostic on stderr\n")


def event(request, sequence, kind, payload):
    response = {
        "protocol_version": 1,
        "request_id": request["request_id"],
        "project_id": request["project_id"],
        "run_id": request["run_id"],
        "attempt_id": request["attempt_id"],
        "sequence": sequence,
        "kind": kind,
        "payload": payload,
    }
    sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
    sys.stdout.flush()


for sequence, line in enumerate(sys.stdin, start=1):
    request = json.loads(line)
    if request["operation"] == "capabilities":
        event(request, sequence, "capabilities", {
            "package_version": pin["package_version"],
            "protocol_versions": [1], "speech_plan_versions": [1],
            "segment_result_versions": [1], "synthesis_result_versions": [1],
            "operations": ["capabilities", "validate"], "synthesis_available": False,
            "schema_sha256": pin["schema_sha256"],
        })
        continue
    if mode == "eof":
        break
    if mode == "partial":
        sys.stdout.write("{\"partial\":true}")
        sys.stdout.flush()
        break
    if mode == "malformed":
        sys.stdout.write("not-json\n")
        sys.stdout.flush()
        break
    if mode == "tree":
        child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"],
                                 stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                 stderr=subprocess.DEVNULL)
        if pid_file is not None:
            pid_file.write_text(str(child.pid), encoding="ascii")
    payload = {"report": {"version": 1, "scope": "speech_plan", "valid": True, "errors": []}}
    kind = "validation"
    if mode == "unknown":
        kind = "completed"
    response = {
        "protocol_version": 1, "request_id": request["request_id"],
        "project_id": request["project_id"], "run_id": request["run_id"],
        "attempt_id": "stale-attempt" if mode == "stale" else request["attempt_id"],
        "sequence": sequence, "kind": kind, "payload": payload,
    }
    sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
    sys.stdout.flush()
    if mode == "tree":
        time.sleep(30)
