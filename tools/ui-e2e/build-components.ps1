param([Parameter(Mandatory=$true)][string]$Id)
$ErrorActionPreference = 'Stop'
function Get-PinnedSha([string]$File) {
    $algorithm = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($File)
    try { return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $algorithm.Dispose() }
}
if ($Id -notmatch '^[a-zA-Z0-9][a-zA-Z0-9._-]{0,120}$') { throw 'Invalid component build identity' }
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$workspace = Split-Path -Parent $taskRoot
$destination = Join-Path $taskRoot ('.cache/ui-e2e/wheels/' + $Id)
if (Test-Path -LiteralPath $destination) { throw 'Retained component builds cannot be overwritten' }
[IO.Directory]::CreateDirectory($destination) | Out-Null
$python = Join-Path $workspace 'TTS/PUZA_TTS/.venv_gpu/Scripts/python.exe'
$source = Join-Path $workspace 'CONTRACT/VOXSCRIPT_CONTRACT'
$env:SOURCE_DATE_EPOCH = '1791639837'
& $python -I -m pip wheel --no-index --no-deps --no-build-isolation --wheel-dir $destination $source
if ($LASTEXITCODE -ne 0) { throw 'CONTRACT wheel build failed' }
$ttsPin = '40af9aba28b42c02220fe941805a114c23ad905737b5dc5df54d89e873e82723'
$ttsWheel = Join-Path $workspace ('TTS/PUZA_TTS/.tmp_audio/wheels-retained/' + $ttsPin + '/puza_tts-0.3.0-py3-none-any.whl')
if ((Get-PinnedSha $ttsWheel) -ne $ttsPin) { throw 'Streamed TTS wheel pin differs' }
[IO.File]::Copy($ttsWheel, (Join-Path $destination 'puza_tts-0.3.0-py3-none-any.whl'), $false)
$files = @(Get-ChildItem -LiteralPath $destination -Filter '*.whl' -File | ForEach-Object {
    [ordered]@{path=('.cache/ui-e2e/wheels/' + $Id + '/' + $_.Name);size_bytes=$_.Length;sha256=(Get-PinnedSha $_.FullName)}
})
$receipt = [ordered]@{version=1;contract_commit=(& git -C (Join-Path $workspace 'CONTRACT') rev-parse HEAD);tts_commit='e18f64d1bad80012b085d907673451c9e5b886f7';files=$files;models_loaded=$false;voices_modified=$false}
[IO.File]::WriteAllText((Join-Path $destination 'components.json'), ($receipt | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
Write-Output 'Built and retained two offline component wheels; no models or voice modifications'
