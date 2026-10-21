param(
    [Parameter(Mandatory = $true)][string]$DataRoot,
    [Parameter(Mandatory = $true)][string]$Output
)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($DataRoot)
$config = Get-Content -LiteralPath (Join-Path $root 'tts-runtime.json') -Raw | ConvertFrom-Json
$script = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'export_renderer_evidence.py'))
$outputPath = [IO.Path]::GetFullPath($Output)
& $config.python_executable $script --data-root $root --output $outputPath
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
