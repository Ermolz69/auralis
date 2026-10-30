# Fresh Windows TTS runtime installation

The root `setup:tts:runtime` Task creates a new local CPython 3.12 x64 CUDA
environment from a complete, reviewed wheelhouse. It does not copy a prior GPU
environment, download model weights, qualify voices or run inference. The older
`setup:tts:local` Task remains the next step for installing pinned components,
voices and model inventory into the Auralis profile.

## Inputs

- Native Windows CPython **3.12 x64** with `venv` and `pip`.
- Microsoft Visual C++ x64 runtime DLLs `vcruntime140.dll`,
  `vcruntime140_1.dll` and `msvcp140.dll` in `System32`.
- An NVIDIA driver capable of the pinned PyTorch CUDA 12.8 build and an
  available CUDA device. The installer checks this with PyTorch on this machine.
- A local wheelhouse containing the complete Windows CPython 3.12 dependency
  closure, including the exact reviewed TTS and speech CONTRACT wheels. Use
  binary wheels only. `torch` and `torchaudio` must be `2.8.0+cu128`; the pinned
  versions of VoxCPM, F5-TTS and the components are enforced by the installer.
- Local Windows `ffmpeg.exe` and `ffprobe.exe` in one directory. Their exact
  sizes and SHA-256 values are included in the manifest.

Prepare the wheelhouse from trusted upstream indexes on a connected preparation
machine, then review its package list and provenance. A complete manifest has
`version: 1`, `platform: "cp312-win_amd64"`, a `wheels` array of objects with
`filename`, `size_bytes` and `sha256`, and `windows_tools` entries of that same
shape for `ffmpeg` and `ffprobe`. The `source_archives` array retains the hashes
of source archives used to build missing wheels. The manifest capture Task verifies all wheel
RECORD entries, supported Python/platform tags, exact reviewed component wheel
hashes and required package versions, then writes a **candidate** manifest.
Keep that file with the reviewed installation inputs; it is not proof of an
installation by itself.

The observed 30 October preparation used the exact component wheels pinned in
`tools/tts/component_pins.py` and the verified FFmpeg/FFprobe sidecars from
`src-tauri/binaries` (whose hashes are in `tools/media-tools/manifest.json`).
With those two wheels already copied into `$wheels`, the dependency acquisition
was:

```powershell
& $python -m pip wheel --no-deps --wheel-dir $wheels 'argbind==0.3.9'
& $python -m pip download --no-deps --no-binary=:all: --dest $wheels 'argbind==0.3.9'
& $python -m pip download --prefer-binary --dest $wheels --find-links $wheels `
  --index-url https://pypi.org/simple --extra-index-url https://download.pytorch.org/whl/cu128 `
  "$wheels/puza_tts-0.3.0-py3-none-any.whl" `
  "$wheels/voxscript_contract-0.2.0-py3-none-any.whl" `
  'torch==2.8.0+cu128' 'torchaudio==2.8.0+cu128' 'voxcpm==2.0.3' `
  'f5-tts==1.1.21' 'accelerate==1.14.0' 'safetensors==0.8.0' `
  'huggingface-hub==1.29.0' 'ruaccent==1.5.8.3' 'jsonschema==4.26.0' `
  'portalocker==4.4.0' 'psutil==7.2.2' 'aiohttp==3.14.1' 'rich==13.9.4'
& $python -m pip wheel --no-deps --wheel-dir $wheels (Get-ChildItem -LiteralPath $wheels -Filter '*.tar.gz').FullName
```

The last command built six additional source-only dependencies on this host.
The exact built wheel bytes, rather than a claim that a rebuild will match them,
are pinned in the local manifest. The TTS direct lock still lists
`portalocker==4.3.0`; the pinned CONTRACT 0.2.0 wheel requires 4.4.0 and the
actual resolver refused 4.3.0. Use the resolved wheel closure above.

```powershell
task setup:tts:runtime:manifest PYTHON=C:/Python312/python.exe WHEELHOUSE=C:/runtime-wheels WINDOWS_TOOLS=C:/ffmpeg/bin OUTPUT=C:/runtime-inputs/win-cp312-cu128.json
task setup:tts:runtime PYTHON=C:/Python312/python.exe MANIFEST=C:/runtime-inputs/win-cp312-cu128.json WHEELHOUSE=C:/runtime-wheels WINDOWS_TOOLS=C:/ffmpeg/bin DATA_ROOT="$env:APPDATA/com.auralis.desktop"
```

The install Task checks all pinned wheel bytes and internal RECORD hashes
before writing anything to the destination. It then creates a new venv,
installs with `--no-index --no-deps`, runs `pip check`, checks the exact
Python/Torch/TorchAudio/CUDA versions and local CUDA availability, and executes
both media tools. A failed installation leaves an `.installing` marker and no
success receipt. It never replaces an existing runtime directory.

On success, the Task prints a path of the form
`<DATA_ROOT>/tts/runtime/<manifest-sha-prefix>/runtime-installation.json`.
That local receipt records the timestamp, exact wheel and media-tool pins,
installed Python hash, observed Torch/TorchAudio/CUDA versions and GPU name.
The adjacent `venv` is the `--runtime-source` for the next Task:

```powershell
$runtime = 'C:/.../tts/runtime/<manifest-sha-prefix>'
task setup:tts:local:fresh RUNTIME_ROOT="$runtime" DATA_ROOT="$env:APPDATA/com.auralis.desktop"
```

`setup:tts:local` verifies that the fresh receipt names the executing Python,
copies the pinned media tools with the runtime, and links the fresh receipt
SHA-256 in `tts/receipt.json`. The TTS worker prepends the installed runtime's
`bin` directory to its child-process PATH. This is local installation identity,
not a clean-machine acceptance or speech-quality result. The complete native
and listening gates remain in [TTS integration](tts-execution.md).
