"""Hold Caster's Windows file-lock primitive for the Rust admission interop test."""

import sys
import time
from pathlib import Path

import portalocker


lock = Path(sys.argv[1])
ready = Path(sys.argv[2])
lock.parent.mkdir(parents=True, exist_ok=True)
with lock.open("a+") as stream:
    portalocker.lock(stream, portalocker.LOCK_EX | portalocker.LOCK_NB)
    ready.write_text("ready", encoding="ascii")
    time.sleep(30)
