"""Read-only verification of the pinned streaming policy; never import engines."""
from __future__ import annotations

import hashlib
import importlib.util
import io
import json
import math
from pathlib import Path
import sys
import time
import wave


def digest(file: Path) -> str:
    with file.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify(data: dict) -> dict:
    deadline = time.monotonic() + data["max_wall_seconds"]

    def check() -> None:
        if time.monotonic() >= deadline:
            raise TimeoutError("Read-only PCM verification deadline exhausted")

    for entry in data["library_files"]:
        check()
        if digest(Path(entry["path"])) != entry["sha256"]:
            raise ValueError("Pinned numerical library bytes changed")
    sys.path.insert(0, data["site_packages"])
    import numpy as np
    import scipy
    import soundfile as sf

    helper = Path(data["helper_path"])
    if digest(helper) != data["helper_sha256"]:
        raise ValueError("Pinned streaming helper changed")
    spec = importlib.util.spec_from_file_location("auralis_ui_pcm_policy", helper)
    policy = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = policy
    spec.loader.exec_module(policy)
    if policy.AUDIO_ASSEMBLY_POLICY != data["policy"] or data["policy"] != "pcm16_stream_fade10ms_peak089125_kaiser5_v1":
        raise ValueError("Unsupported deterministic PCM conversion policy")

    sources = data["chunks"]
    peak, total = 0.0, 0
    for source in sources:
        check()
        if digest(Path(source["path"])) != source["sha256"]:
            raise ValueError("Published technical chunk changed")
        with sf.SoundFile(source["path"]) as stream:
            if stream.subtype != "PCM_16" or stream.channels != data["channels"]:
                raise ValueError("Unsupported technical chunk format")
            frames = math.ceil(stream.frames * 48000 / stream.samplerate)
            total += frames
            for start, audio in policy._decoded_blocks(stream, 48000, check):
                if not np.isfinite(audio).all():
                    raise ValueError("Non-finite technical chunk PCM")
                policy._fade_block(audio, start, frames, 48000)
                peak = max(peak, float(np.max(np.abs(audio))))

    expected_digest = hashlib.sha256()
    actual_digest = hashlib.sha256()
    with wave.open(data["raw_wav"], "rb") as aggregate:
        if (aggregate.getframerate(), aggregate.getnchannels(), aggregate.getsampwidth(), aggregate.getnframes()) != (48000, data["channels"], 2, total):
            raise ValueError("Aggregate format or complete frame coverage differs")
        for utterance in data["utterances"]:
            check()
            aggregate.setpos(utterance["first_frame"])
            left, signal = utterance["frame_count"], False
            while left:
                block = aggregate.readframes(min(65536, left))
                frames = len(block) // (data["channels"] * 2)
                if frames == 0:
                    raise ValueError("Aggregate utterance range was truncated")
                signal = signal or bool(np.any(np.frombuffer(block, dtype="<i2")))
                left -= frames
            if utterance["text"].strip() and not signal:
                raise ValueError("Nonblank approved utterance has no PCM signal")
        aggregate.rewind()
        gain = 0.89125 / peak if peak > 0.0 else None
        frame = 0
        for source in sources:
            with sf.SoundFile(source["path"]) as stream:
                frames = math.ceil(stream.frames * 48000 / stream.samplerate)
                for start, audio in policy._decoded_blocks(stream, 48000, check):
                    policy._fade_block(audio, start, frames, 48000)
                    if gain is not None:
                        audio = np.clip(audio * gain, -1.0, 1.0)
                    # Encode only into memory. No replacement WAV or publication.
                    memory = io.BytesIO()
                    with sf.SoundFile(memory, mode="w", samplerate=48000, channels=data["channels"], format="RAW", subtype="PCM_16", endian="LITTLE") as output:
                        output.write(audio)
                    expected = memory.getvalue()
                    actual = aggregate.readframes(len(audio))
                    if expected != actual:
                        raise ValueError(f"Aggregate PCM differs from verified chunks at frame {frame}")
                    expected_digest.update(expected)
                    actual_digest.update(actual)
                    frame += len(audio)
        if aggregate.readframes(1) or frame != total:
            raise ValueError("Aggregate has extra or missing PCM")
    forbidden = [name for name in sys.modules if name.split(".")[0] in {"torch", "transformers", "voxcpm", "espeech", "f5_tts", "tts"}]
    if forbidden:
        raise ValueError("PCM verifier imported an engine or model module")
    return {"policy": data["policy"], "frames": total, "utterances": len(data["utterances"]), "chunks": len(sources), "pcm_sha256": actual_digest.hexdigest(), "expected_pcm_sha256": expected_digest.hexdigest(), "models_loaded": False, "helper_sha256": data["helper_sha256"], "libraries": {"numpy": np.__version__, "scipy": scipy.__version__, "soundfile": sf.__version__, "libsndfile": sf.__libsndfile_version__}}


if __name__ == "__main__":
    data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    result = verify(data)
    with Path(sys.argv[2]).open("x", encoding="utf-8") as output:
        json.dump(result, output, ensure_ascii=False, indent=2)
