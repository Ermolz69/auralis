param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-f0-9]{64}$')][string]$InstallerSha256,
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-f0-9]{64}$')][string]$ApplicationSha256,
    [Parameter(Mandatory=$true)][string]$Output
)
$ErrorActionPreference = 'Stop'
function Get-PinnedSha256([string]$File) {
    $algorithm = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($File)
    try { return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $algorithm.Dispose() }
}
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$receiptPath = [IO.Path]::GetFullPath((Join-Path $taskRoot $Output))
if (Test-Path -LiteralPath $receiptPath) { throw 'Retained installation receipt already exists' }
$installer = Join-Path $taskRoot 'target/release/bundle/nsis/Auralis_0.1.0_x64-setup.exe'
$installRoot = Join-Path $env:LOCALAPPDATA 'Programs\Auralis'
$application = Join-Path $installRoot 'auralis-app.exe'
if (Test-Path -LiteralPath $installRoot) { throw 'Installation target already exists; freeze an explicit upgrade case instead of replacing it' }
if ((Get-PinnedSha256 $installer) -ne $InstallerSha256) { throw 'Installer identity differs from the predeclared candidate' }
$clock = [Diagnostics.Stopwatch]::StartNew()
$started = [DateTime]::UtcNow.ToString('o')
$failure = $null
$process = $null
$actual = $null
try {
    $process = Start-Process -FilePath $installer -ArgumentList @('/S', ('/D=' + $installRoot)) -PassThru -WindowStyle Hidden
    if (-not $process.WaitForExit(600000)) { throw 'Installer exceeded its 10-minute budget; inspect the owned installer process before a retry' }
    if ($process.ExitCode -ne 0) { throw ('Installer exit ' + $process.ExitCode) }
    $actual = Get-PinnedSha256 $application
    if ($actual -ne $ApplicationSha256) { throw 'Installed application differs from the captured NSIS payload' }
} catch { $failure = $_.Exception.Message }
$receipt = [ordered]@{
    version=1;case_id='NORMAL-PROFILE-INSTALL-v1';started_at=$started;ended_at=[DateTime]::UtcNow.ToString('o');elapsed_ms=$clock.ElapsedMilliseconds
    installer=$installer;installer_sha256=$InstallerSha256;install_root=$installRoot;application=$application;application_sha256=$actual;expected_application_sha256=$ApplicationSha256
    installer_pid=if ($process) {$process.Id} else {$null};exit_code=if ($process -and $process.HasExited) {$process.ExitCode} else {$null}
    script_sha256=(Get-PinnedSha256 $PSCommandPath);error=$failure;status=if ($failure) {'failed'} else {'installed_verified'}
    limitation='Local development-host installation only. Native workflow, asset acquisition, listening and clean Windows are separate acceptance gates. No app or model is launched by this command.'
}
[IO.Directory]::CreateDirectory((Split-Path -Parent $receiptPath)) | Out-Null
[IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
Write-Output ($receipt | ConvertTo-Json -Depth 6)
if ($failure) { throw $failure }
