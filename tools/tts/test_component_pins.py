from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import component_pins as pins


class ComponentPinTests(unittest.TestCase):
    def setUp(self):
        temporary_root = Path(__file__).resolve().parents[2] / ".cache/component-pin-tests"
        temporary_root.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="auralis-pin-test-", dir=temporary_root)
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.target = self.base / "auralis"
        (self.target / "tools/tts").mkdir(parents=True)
        self.sources, self.selections, self.wheels, self.wheel_pins = {}, {}, {}, {}
        for role, (_, _, subtree, _, name, version) in pins.SOURCES.items():
            root = self.base / role
            root.mkdir()
            self.run_git(root, "init", "-q")
            repository = f"fixture/{role}"
            self.run_git(root, "remote", "add", "origin", f"https://github.com/{repository}.git")
            package = root if role == "caster" else root / subtree
            package.mkdir(parents=True, exist_ok=True)
            (package / "pyproject.toml").write_text(
                f'[project]\nname = "{name}"\nversion = "{version}"\ndependencies = ["scipy==1.18.1"]\n',
                encoding="utf-8",
            )
            (package / "src").mkdir(exist_ok=True)
            (package / "src/module.py").write_text(f'ROLE = "{role}"\n', encoding="utf-8")
            self.run_git(root, "add", ".")
            self.run_git(root, "commit", "-qm", f"Fixture {role}")
            revision = self.run_git(root, "rev-parse", "HEAD")
            tree = self.run_git(root, "rev-parse", f"HEAD:{subtree}")
            self.sources[role] = revision, repository, subtree, tree, name, version
            self.selections[role] = root, revision
            if role != "caster":
                wheel = self.base / f"{role}.whl"
                with zipfile.ZipFile(wheel, "w") as archive:
                    archive.writestr(f"{role}.dist-info/METADATA", f"Metadata-Version: 2.1\nName: {name}\nVersion: {version}\n")
                self.wheels[role] = wheel
                self.wheel_pins[role] = hashlib.sha256(wheel.read_bytes()).hexdigest(), wheel.stat().st_size
        self.addCleanup(patch.stopall)
        patch.object(pins, "SOURCES", self.sources).start()
        patch.object(pins, "WHEELS", self.wheel_pins).start()
        self.setup = self.target / "tools/tts/setup_local.py"
        self.lock = self.target / "tools/tts/media-contract.lock.json"
        self.setup.write_text(
            f'TTS_COMMIT = "{pins.OLD["tts"]}"\nCONTRACT_COMMIT = "{pins.OLD["speech"]}"\nCASTER_COMMIT = "{pins.OLD["caster"]}"\n'
            'NATIVE_PACKAGES = (\n    "soundfile", "setuptools",\n)\n'
            'raise RuntimeError("The installer must never execute during pin validation")\n',
            encoding="utf-8",
        )
        self.lock.write_text(json.dumps({"package_version": "0.3.0", "source_commit": pins.OLD_MEDIA[0],
                                         "source_tree": pins.OLD_MEDIA[1], "wheel_sha256": self.wheel_pins["media"][0]}) + "\n", encoding="utf-8")
        self.original = self.setup.read_bytes(), self.lock.read_bytes()

    @staticmethod
    def run_git(root, *args):
        env = dict(os.environ, GIT_AUTHOR_NAME="Fixture", GIT_AUTHOR_EMAIL="fixture@example.invalid",
                   GIT_COMMITTER_NAME="Fixture", GIT_COMMITTER_EMAIL="fixture@example.invalid")
        result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True, env=env, timeout=30)
        if result.returncode:
            raise RuntimeError(result.stderr)
        return result.stdout.strip()

    def update(self):
        return pins.update(self.target, self.selections, self.wheels)

    def unchanged_after_rejection(self):
        with self.assertRaises((ValueError, FileNotFoundError)):
            self.update()
        self.assertEqual((self.setup.read_bytes(), self.lock.read_bytes()), self.original)
        self.assertEqual(list(self.setup.parent.glob(".component-pins-*")), [])

    def test_complete_closure_updates_only_active_pins_and_is_repeatable(self):
        result = self.update()
        self.assertEqual(set(result["updated"]), {"tools/tts/setup_local.py", "tools/tts/media-contract.lock.json"})
        source = self.setup.read_text(encoding="utf-8")
        for role in ("tts", "speech", "caster"):
            self.assertIn(self.sources[role][0], source)
        self.assertEqual(source.count('"scipy"'), 1)
        media = json.loads(self.lock.read_bytes())
        self.assertEqual((media["source_commit"], media["source_tree"]), (self.sources["media"][0], self.sources["media"][3]))
        self.assertEqual(media["wheel_sha256"], self.wheel_pins["media"][0])
        before = self.setup.read_bytes(), self.lock.read_bytes(), self.setup.stat().st_mtime_ns, self.lock.stat().st_mtime_ns
        self.assertEqual(self.update()["updated"], [])
        self.assertEqual(before, (self.setup.read_bytes(), self.lock.read_bytes(), self.setup.stat().st_mtime_ns, self.lock.stat().st_mtime_ns))

    def test_swapped_roots_are_rejected_before_writes(self):
        self.selections["tts"] = self.selections["speech"][0], self.sources["tts"][0]
        self.unchanged_after_rejection()

    def test_short_or_unreviewed_revision_is_rejected(self):
        self.selections["tts"] = self.selections["tts"][0], self.sources["tts"][0][:7]
        self.unchanged_after_rejection()

    def test_dirty_last_source_is_rejected_before_either_target_changes(self):
        (self.selections["caster"][0] / "src/module.py").write_text("DIRTY = True\n", encoding="utf-8")
        self.unchanged_after_rejection()

    def test_untracked_source_file_is_not_silently_admitted(self):
        (self.selections["speech"][0] / "foreign.txt").write_text("untracked", encoding="utf-8")
        self.unchanged_after_rejection()

    def test_foreign_origin_and_nested_roots_are_rejected(self):
        root = self.selections["media"][0]
        self.run_git(root, "remote", "set-url", "origin", "https://github.com/foreign/component.git")
        self.unchanged_after_rejection()
        self.run_git(root, "remote", "set-url", "origin", f"https://github.com/{self.sources['media'][1]}.git")
        self.selections["media"] = root / "VOXSCRIPT_CONTRACT", self.sources["media"][0]
        self.unchanged_after_rejection()

    def test_only_documented_github_hosts_are_accepted(self):
        self.assertEqual(pins.origin_repository("git@github.com:Fixture/tts.git"), "fixture/tts")
        root = self.selections["tts"][0]
        self.run_git(root, "remote", "set-url", "origin", "git@local-alias:fixture/tts.git")
        self.unchanged_after_rejection()

    def test_wrong_tree_is_rejected(self):
        value = list(self.sources["speech"])
        value[3] = "0" * 40
        self.sources["speech"] = tuple(value)
        self.unchanged_after_rejection()

    def test_changed_or_swapped_wheels_are_rejected(self):
        original = self.wheels["tts"].read_bytes()
        self.wheels["tts"].write_bytes(original[:-1] + bytes([original[-1] ^ 1]))
        self.unchanged_after_rejection()
        self.wheels["tts"].write_bytes(original)
        self.wheels["media"] = self.wheels["speech"]
        self.unchanged_after_rejection()

    def test_wheel_metadata_is_verified(self):
        wheel = self.wheels["tts"]
        with zipfile.ZipFile(wheel, "w") as archive:
            archive.writestr("tts.dist-info/METADATA", "Name: foreign\nVersion: 0.3.0\n")
        self.wheel_pins["tts"] = hashlib.sha256(wheel.read_bytes()).hexdigest(), wheel.stat().st_size
        self.unchanged_after_rejection()

    def test_source_package_version_is_verified_independently(self):
        root, _ = self.selections["tts"]
        metadata = root / "PUZA_TTS/pyproject.toml"
        metadata.write_text(metadata.read_text(encoding="utf-8").replace('version = "0.3.0"', 'version = "9.0.0"'), encoding="utf-8")
        self.run_git(root, "add", ".")
        self.run_git(root, "commit", "-qm", "Different package version")
        revision = self.run_git(root, "rev-parse", "HEAD")
        source = list(self.sources["tts"])
        source[0], source[3] = revision, self.run_git(root, "rev-parse", "HEAD:PUZA_TTS")
        self.sources["tts"] = tuple(source)
        self.selections["tts"] = root, revision
        self.unchanged_after_rejection()

    def test_incomplete_closure_and_unknown_lock_are_rejected(self):
        original = self.wheels.pop("media")
        self.unchanged_after_rejection()
        self.wheels["media"] = original
        self.lock.write_text('{}\n', encoding="utf-8")
        with self.assertRaises(ValueError):
            self.update()
        self.assertEqual(self.setup.read_bytes(), self.original[0])

    def test_second_replace_failure_restores_both_originals(self):
        real_replace = os.replace
        calls = 0

        def fail_once(source, destination):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise OSError("Synthetic second-target replacement failure")
            return real_replace(source, destination)

        with patch.object(pins.os, "replace", side_effect=fail_once):
            with self.assertRaises(OSError):
                self.update()
        self.assertEqual((self.setup.read_bytes(), self.lock.read_bytes()), self.original)
        self.assertEqual(list(self.setup.parent.glob(".component-pins-*")), [])


if __name__ == "__main__":
    unittest.main()
