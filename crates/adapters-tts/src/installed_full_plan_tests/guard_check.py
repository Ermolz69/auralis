import json
import sys
import threading
from pathlib import Path

sys.path.insert(0, sys.argv[1])
data = json.loads(sys.argv[2])
guard_profile = sys.getprofile()


class NeverLoadFramework:
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "torch" or fullname.startswith("torch."):
            raise AssertionError("audit failed before the fallback import fence")
        return None


sys.meta_path.insert(0, NeverLoadFramework())


def expect_denial(action):
    try:
        action()
    except RuntimeError as error:
        if not str(error).startswith("Metadata-only guard:"):
            raise
    else:
        raise AssertionError("metadata guard accepted a prohibited inert control")
    finally:
        sys.setprofile(guard_profile)


expect_denial(lambda: __import__("torch"))
expect_denial(lambda: open(Path(data["root"]) / "absent.safetensors", "rb"))
expect_denial(lambda: open(Path(data["model_cache"]) / "metadata-control-absent.txt", "rb"))
fake_engine = {"__name__": "src.tts.engine.inert_guard_control"}
exec("def __init__(self):\n    raise AssertionError('inert engine body ran')", fake_engine)
expect_denial(lambda: fake_engine["__init__"](object()))
fake_render = {"__name__": "src.tts.speech_plan"}
exec("def render_speech_plan():\n    raise AssertionError('inert render body ran')", fake_render)
expect_denial(fake_render["render_speech_plan"])
expect_denial(lambda: open(data["outside_owned_guard"], "wb"))


def threaded_denial(action):
    failures = []
    def target():
        try:
            expect_denial(action)
        except BaseException as error:
            failures.append(error)
    thread = threading.Thread(target=target)
    thread.start()
    thread.join(2)
    if thread.is_alive():
        raise AssertionError("inert guard thread did not terminate")
    if failures:
        raise failures[0]


threaded_denial(lambda: fake_engine["__init__"](object()))
threaded_denial(fake_render["render_speech_plan"])
