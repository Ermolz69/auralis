"""Synthetic read-only verifier regression; no engine imports or join_audio calls."""
from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import sys
import zipfile


def run(config: dict, directory: Path) -> dict:
    for entry in config["library_files"]:
        with Path(entry["path"]).open("rb") as stream:
            assert hashlib.file_digest(stream, "sha256").hexdigest() == entry["sha256"]
    sys.path.insert(0, config["site_packages"])
    import numpy as np
    import soundfile as sf
    from scipy.signal import firwin, resample_poly

    with zipfile.ZipFile(config["wheel"]) as wheel:
        names = [name for name in wheel.namelist() if name.endswith("tts/audio/joiner.py")]
        assert len(names) == 1, "Expected one pinned wheel streaming helper"
        helper = wheel.read(names[0])
    helper_path = directory / "pinned-joiner.py"
    helper_path.write_bytes(helper)
    config.update(helper_path=str(helper_path), helper_sha256=hashlib.sha256(helper).hexdigest())
    module = importlib.util.spec_from_file_location("pcm_verifier", Path(__file__).with_name("pcm-verify.py"))
    verifier = importlib.util.module_from_spec(module)
    module.loader.exec_module(verifier)

    # Build expected PCM independently with whole-array FIR resampling. Clips cross
    # streaming block boundaries and use both integer and fractional rate ratios.
    converted, chunks = [], []
    for index, rate in enumerate((16000, 44100, 48000)):
        samples = np.arange(rate * 2 + 317, dtype=np.float32)
        audio = (0.17 * np.sin(samples * np.float32(0.071 + index * 0.02))).reshape(-1, 1)
        chunk = directory / f"chunk-{index}.wav"
        sf.write(chunk, audio, rate, subtype="PCM_16")
        decoded, _ = sf.read(chunk, dtype="float32", always_2d=True)
        if rate != 48000:
            divisor = math.gcd(rate, 48000)
            up, down = 48000 // divisor, rate // divisor
            taps = firwin(20 * max(up, down) + 1, 1.0 / max(up, down), window=("kaiser", 5.0)).astype(np.float32)
            decoded = resample_poly(decoded, up, down, axis=0, window=taps)
        fade = min(len(decoded) // 2, 480)
        decoded[:fade] *= np.linspace(0, 1, fade, dtype=np.float32)[:, None]
        decoded[-fade:] *= np.linspace(1, 0, fade, dtype=np.float32)[:, None]
        converted.append(decoded)
        chunks.append({"path": str(chunk), "sha256": verifier.digest(chunk)})
    merged = np.concatenate(converted)
    expected = np.clip(merged * (0.89125 / float(np.max(np.abs(merged)))), -1, 1)
    aggregate = directory / "aggregate.wav"
    sf.write(aggregate, expected, 48000, subtype="PCM_16")
    config.update(policy="pcm16_stream_fade10ms_peak089125_kaiser5_v1", chunks=chunks,
                  channels=1, raw_wav=str(aggregate), max_wall_seconds=120,
                  utterances=[{"first_frame": 0, "frame_count": len(converted[0]), "text": "Первая фраза."},
                              {"first_frame": len(converted[0]), "frame_count": len(merged)-len(converted[0]), "text": "Вторая фраза."}])
    with (directory / "wrapper-inputs.json").open("x", encoding="utf-8") as stream:
        json.dump({key: config[key] for key in ("policy", "chunks", "channels", "raw_wav", "utterances")}, stream, ensure_ascii=False)
    accepted = verifier.verify(config)
    outcomes = {"correct_multi_chunk_pcm": "passed"}

    def rejects(name: str, data: dict, message: str) -> None:
        try:
            verifier.verify(data)
        except ValueError as error:
            assert message in str(error), str(error)
            outcomes[name] = "passed"
        else:
            raise AssertionError(f"False passed: {name}")

    changed = copy.deepcopy(config)
    changed["policy"] = "unknown"
    rejects("unknown_policy", changed, "policy")
    changed = copy.deepcopy(config)
    changed["helper_sha256"] = "0" * 64
    rejects("changed_helper", changed, "helper changed")
    changed = copy.deepcopy(config)
    changed["library_files"][0]["sha256"] = "0" * 64
    rejects("changed_library", changed, "library bytes changed")
    substituted = directory / "same-length-substitution.wav"
    sf.write(substituted, expected * np.float32(0.5), 48000, subtype="PCM_16")
    rejects("same_length_nonzero_substitution", {**config, "raw_wav": str(substituted)}, "PCM differs")
    silent = expected.copy()
    silent[len(converted[0]):] = 0
    substituted = directory / "silent-second-utterance.wav"
    sf.write(substituted, silent, 48000, subtype="PCM_16")
    rejects("silent_nonblank_utterance", {**config, "raw_wav": str(substituted)}, "no PCM signal")
    return {"status": "passed", "scope": "synthetic-read-only-pcm", "checks": outcomes, "proof": accepted}


if __name__ == "__main__":
    config = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    result = run(config, Path(sys.argv[2]))
    with (Path(sys.argv[2]) / "validation.json").open("x", encoding="utf-8") as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2)
