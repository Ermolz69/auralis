"""Metadata-control instrumentation installed before the unmodified worker body."""
import atexit
import ctypes
from ctypes import wintypes
import importlib.metadata
import json
import os
from pathlib import Path
import sys
import threading

root = Path(__file__).parent
config = json.loads((root / "metadata-guard.json").read_text(encoding="utf-8"))
write_root = os.path.normcase(os.path.realpath(config["write_root"]))
model_root = os.path.normcase(os.path.realpath(config["model_cache"]))
package_root = os.path.normcase(os.path.realpath(config["package_root"]))
blocked_modules = ("torch", "torchaudio", "transformers", "voxcpm", "f5_tts", "modelscope", "vocos", "accelerate", "onnxruntime", "safetensors")
blocked_calls = {"render_speech_plan", "render_prepared_segments", "render_job", "synthesize", "_execute", "download_engine_models"}
state = {"pid": os.getpid(), "imports": [], "violations": [], "model_file_opens": 0, "engine_calls": 0,
         "chunk_previews": [],
         "scope": "Thin adapter definitions may import; model implementations and execution are prohibited"}
kernel = ctypes.WinDLL("kernel32", use_last_error=True)
kernel.GetCurrentProcess.restype = wintypes.HANDLE
kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
times = [wintypes.FILETIME() for _ in range(4)]
if not kernel.GetProcessTimes(kernel.GetCurrentProcess(), *(ctypes.byref(item) for item in times)):
    raise ctypes.WinError(ctypes.get_last_error())
state["creation_time"] = (times[0].dwHighDateTime << 32) | times[0].dwLowDateTime


def normalized(value):
    return os.path.normcase(os.path.realpath(os.fsdecode(value)))


def inside(value, directory):
    return value == directory or value.startswith(directory + os.sep)


def denied(reason):
    state["violations"].append(reason)
    raise RuntimeError("Metadata-only guard: " + reason)


def audit(event, args):
    if event == "import":
        name = args[0]
        state["imports"].append(name)
        if any(name == item or name.startswith(item + ".") for item in blocked_modules):
            denied("model/framework import " + name)
    elif event == "open" and isinstance(args[0], (str, bytes, os.PathLike)):
        path = normalized(args[0])
        if inside(path, model_root) or Path(path).suffix.lower() in {".pt", ".pth", ".bin", ".safetensors", ".gguf", ".onnx", ".ckpt"}:
            state["model_file_opens"] += 1
            denied("model/weight file open " + path)
        flags = args[2]
        if flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND) and not inside(path, write_root):
            denied("write outside owned output " + path)
    elif event in {"os.mkdir", "os.remove", "os.rmdir", "os.chmod", "os.truncate"}:
        if isinstance(args[0], (str, bytes, os.PathLike)) and not inside(normalized(args[0]), write_root):
            denied("mutation outside owned output " + event)
    elif event in {"os.rename", "os.link", "os.symlink"}:
        if any(not inside(normalized(value), write_root) for value in args[:2]):
            denied("relocation outside owned output " + event)
    elif event in {"subprocess.Popen", "os.system", "os.exec", "socket.connect", "socket.bind"}:
        denied("metadata operation attempted " + event)


def profile(frame, event, returned):
    if event == "return" and frame.f_code.co_name == "prepare_speech_plan" and frame.f_globals.get("__name__") == "src.tts.speech_plan" and returned is not None:
        job, segments = returned
        state["chunk_previews"].append({"engine": job.engine, "expected_utterance_ids": list(job.expected_utterance_ids),
            "segments": [{"utterance_id": item.utterance_id, "chunk_id": item.chunk_id, "cue_id": item.cue_id,
                "text": item.text, "role_id": item.voice, "voice_id": item.resolved_voice, "reference_id": item.reference_id,
                "start_ms": item.source_start_ms, "end_ms": item.source_end_ms} for item in segments]})
    if event != "call":
        return
    name = frame.f_code.co_name
    if name != "__init__" and name not in blocked_calls:
        return
    module = frame.f_globals.get("__name__", "")
    if not module.startswith("src.tts."):
        return
    engine_init = name == "__init__" and module.startswith("src.tts.engine.")
    if engine_init or name in blocked_calls:
        state["engine_calls"] += 1
        denied("engine construction/render/execute call " + name)


def finish():
    state["profile_active_before_exit"] = sys.getprofile() is profile
    state["thread_profile_active_before_exit"] = threading.getprofile() is profile
    sys.setprofile(None)
    state["imports"] = sorted(set(state["imports"]) | set(sys.modules))
    state["thin_adapter_imports"] = [name for name in state["imports"] if name.startswith("src.tts.engine")]
    state["framework_modules_loaded"] = [name for name in sys.modules if any(name == item or name.startswith(item + ".") for item in blocked_modules)]
    state["contract_version"] = importlib.metadata.version("voxscript-contract")
    state["tts_version"] = importlib.metadata.version("puza-tts")
    destination = Path(config["audit_root"]) / (str(os.getpid()) + "-" + str(state["creation_time"]) + ".json")
    with destination.open("x", encoding="utf-8") as stream:
        json.dump(state, stream, ensure_ascii=False, indent=2)


atexit.register(finish)
sys.addaudithook(audit)
for name in list(sys.modules):
    if any(name == item or name.startswith(item + ".") for item in blocked_modules):
        denied("model/framework already loaded " + name)
sys.setprofile(profile)
threading.setprofile(profile)
