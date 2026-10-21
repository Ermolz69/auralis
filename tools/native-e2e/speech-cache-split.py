"""Prove that the native changed-text fixture invalidates one installed TTS chunk."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from src.tts.pipeline.speech_segmentation import split_speech_text


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    profile_bytes = args.profile.read_bytes()
    settings = json.loads(profile_bytes)["settings"]
    source = (
        "Мы сохраняем проверенный текст и читаем его спокойно. "
        "Голос остаётся нейтральным, а каждое слово должно звучать ясно. "
    ) * 9
    changed = source.replace("каждое слово должно", "каждый текст должен", 1)
    options = {
        "preferred_chars": settings["speech_preferred_chars"],
        "hard_chars": settings["speech_hard_chars"],
        "hard_words": settings["speech_hard_words"],
    }
    original_chunks = split_speech_text(source, **options)
    changed_chunks = split_speech_text(changed, **options)
    changed_indices = [
        index
        for index, (original, edited) in enumerate(zip(original_chunks, changed_chunks))
        if original != edited
    ]
    if len(original_chunks) != 6 or len(changed_chunks) != 6 or changed_indices != [0]:
        raise SystemExit("Native changed-text fixture no longer invalidates exactly chunk 0")
    result = {
        "version": 1,
        "status": "installed_split_policy_changes_only_chunk_zero",
        "python": sys.version,
        "executable": sys.executable,
        "script_sha256": sha256(Path(__file__).read_bytes()),
        "config_sha256": sha256(args.config.read_bytes()),
        "profile_sha256": sha256(profile_bytes),
        "profile_path": str(args.profile),
        "split_settings": options,
        "original_text": source,
        "original_text_sha256": sha256(source.encode("utf-8")),
        "changed_text": changed,
        "changed_text_sha256": sha256(changed.encode("utf-8")),
        "original_chunk_sha256": [sha256(chunk.encode("utf-8")) for chunk in original_chunks],
        "changed_chunk_sha256": [sha256(chunk.encode("utf-8")) for chunk in changed_chunks],
        "changed_indices": changed_indices,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes((json.dumps(result, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
    print(json.dumps({"status": result["status"], "chunks": len(original_chunks), "changed_indices": changed_indices}))


if __name__ == "__main__":
    main()
