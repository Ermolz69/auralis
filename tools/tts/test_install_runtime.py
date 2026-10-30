"""Model-free controls for the fresh Windows runtime installer."""

from __future__ import annotations

import base64
import csv
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import install_runtime as runtime
import setup_local


def wheel(path: Path, *, corrupt: bool = False, tag: str = "py3-none-any") -> None:
    prefix = "example-1.0.dist-info/"
    files = {
        "example/__init__.py": b"value = 1\n",
        prefix + "METADATA": b"Metadata-Version: 2.3\nName: example\nVersion: 1.0\n",
        prefix + "WHEEL": f"Wheel-Version: 1.0\nRoot-Is-Purelib: true\nTag: {tag}\n".encode(),
    }
    rows = []
    for name, content in files.items():
        digest = base64.urlsafe_b64encode(hashlib.sha256(content).digest()).rstrip(b"=").decode()
        rows.append([name, "sha256=" + digest, str(len(content))])
    rows.append([prefix + "RECORD", "", ""])
    output = io.StringIO()
    csv.writer(output, lineterminator="\n").writerows(rows)
    files[prefix + "RECORD"] = output.getvalue().encode()
    if corrupt:
        files["example/__init__.py"] = b"value = 2\n"
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in files.items():
            archive.writestr(name, content)


class RuntimeInputTests(unittest.TestCase):
    def test_wheel_record_and_platform_are_verified(self):
        from pip._vendor.packaging.tags import sys_tags

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            file = root / "example-1.0-py3-none-any.whl"
            wheel(file)
            self.assertEqual(runtime.inspect_wheel(file, set(sys_tags())), ("example", "1.0"))
            wheel(file, corrupt=True)
            with self.assertRaisesRegex(ValueError, "RECORD integrity"):
                runtime.inspect_wheel(file, set(sys_tags()))
            other = root / "example-1.0-cp311-cp311-win_amd64.whl"
            wheel(other, tag="cp311-cp311-win_amd64")
            with self.assertRaisesRegex(ValueError, "incompatible"):
                runtime.inspect_wheel(other, set(sys_tags()))

    def test_file_pins_reject_changed_bytes_and_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            file = root / "ffmpeg.exe"
            file.write_bytes(b"sample")
            pin = {"filename": file.name, "size_bytes": 6,
                   "sha256": hashlib.sha256(b"sample").hexdigest()}
            self.assertEqual(runtime.pinned_file(root, pin), file)
            file.write_bytes(b"tamper")
            with self.assertRaisesRegex(ValueError, "differs"):
                runtime.pinned_file(root, pin)
            pin["filename"] = "../ffmpeg.exe"
            with self.assertRaisesRegex(ValueError, "Invalid pinned"):
                runtime.pinned_file(root, pin)

    def test_incomplete_closure_cannot_start_install(self):
        from pip._vendor.packaging.tags import sys_tags

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            file = root / "example-1.0-py3-none-any.whl"
            wheel(file)
            pin = {"filename": file.name, "size_bytes": file.stat().st_size,
                   "sha256": runtime.sha256(file)}
            for name in ("ffmpeg", "ffprobe"):
                (root / f"{name}.exe").write_bytes(b"sample")
            tools = {name: {"filename": name + ".exe", "size_bytes": 6,
                            "sha256": hashlib.sha256(b"sample").hexdigest()}
                     for name in ("ffmpeg", "ffprobe")}
            manifest = {"version": 1, "platform": "cp312-win_amd64", "wheels": [pin],
                        "windows_tools": tools, "source_archives": []}
            with self.assertRaisesRegex(ValueError, "Required wheel"):
                runtime.verify_inputs(manifest, root, root, set(sys_tags()))

    def test_fresh_receipt_binds_python_and_media_tool_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "venv"
            python = source / "Scripts/python.exe"
            python.parent.mkdir(parents=True)
            python.write_bytes(b"python")
            tools = root / "bin"
            tools.mkdir()
            media = {}
            for name in ("ffmpeg", "ffprobe"):
                file = tools / (name + ".exe")
                file.write_bytes(name.encode())
                media[name] = {"filename": file.name, "size_bytes": file.stat().st_size,
                               "sha256": runtime.sha256(file)}
            receipt = root / "runtime-installation.json"
            receipt.write_text(json.dumps({
                "status": "verified_local_runtime", "python_executable": str(python),
                "python_sha256": runtime.sha256(python), "windows_tools": media,
                "manifest_sha256": "a" * 64,
                "compatibility": {"torch": "2.8.0+cu128", "torchaudio": "2.8.0+cu128",
                                  "cuda_runtime": "12.8"},
            }), encoding="utf-8")
            with patch.object(setup_local.sys, "executable", str(python)):
                self.assertEqual(setup_local.verified_fresh_runtime(source), runtime.sha256(receipt))
                (tools / "ffmpeg.exe").write_bytes(b"changed")
                with self.assertRaisesRegex(ValueError, "media tool differs"):
                    setup_local.verified_fresh_runtime(source)


if __name__ == "__main__":
    unittest.main()
