param(
    [string]$Output = 'tools/native-e2e/evidence/CACHE_SPLIT_20261021.json'
)
$ErrorActionPreference = 'Stop'
$configPath = Join-Path $env:APPDATA 'com.auralis.desktop/tts-runtime.json'
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$profilePath = Join-Path $config.profile_root 'native-plan-voxcpm2.json'
if (-not (Test-Path -LiteralPath $config.python_executable -PathType Leaf)) { throw 'Pinned TTS Python is unavailable' }
if (-not (Test-Path -LiteralPath $profilePath -PathType Leaf)) { throw 'Pinned VoxCPM2 profile is unavailable' }
$env:PYTHONPATH = $config.package_root
& $config.python_executable tools/native-e2e/speech-cache-split.py --profile $profilePath --config $configPath --output $Output
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
