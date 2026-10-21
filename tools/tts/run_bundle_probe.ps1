param(
    [Parameter(Mandatory = $true)][string]$DataRoot,
    [Parameter(Mandatory = $true)][string]$CaseId
)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($DataRoot)
$config = Get-Content -LiteralPath (Join-Path $root 'tts-runtime.json') -Raw | ConvertFrom-Json
$env:AURALIS_TTS_BUNDLE_DATA_ROOT = $root
$admission = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../crates/common/src/model_admission.ps1'))
$probe = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'probe_installed_bundle.py'))
& $admission -ProbePython $config.python_executable -ProbeScript $probe -CaseId $CaseId
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
