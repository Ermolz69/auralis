param(
    [Parameter(Mandatory=$true)][string]$Id,
    [Parameter(Mandatory=$true)][string]$Python,
    [Parameter(Mandatory=$true)][string]$Uv,
    [Parameter(Mandatory=$true)][string]$ContractRoot,
    [Parameter(Mandatory=$true)][string]$ContractCommit,
    [Parameter(Mandatory=$true)][string]$TtsRoot,
    [Parameter(Mandatory=$true)][string]$TtsCommit,
    [Parameter(Mandatory=$true)][string]$TtsWheel,
    [Parameter(Mandatory=$true)][string]$TtsWheelSha256
)
$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$builder = Join-Path $PSScriptRoot 'component-build.py'
& $Python -I -B $builder --id $Id --worktree-root $taskRoot --uv $Uv --contract-root $ContractRoot --contract-commit $ContractCommit --tts-root $TtsRoot --tts-commit $TtsCommit --tts-wheel $TtsWheel --tts-wheel-sha256 $TtsWheelSha256
if ($LASTEXITCODE -ne 0) { throw 'Verified offline component build failed' }
