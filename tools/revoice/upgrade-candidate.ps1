param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-f0-9]{64}$')][string]$InstallerSha256,
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-f0-9]{64}$')][string]$ApplicationSha256,
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-f0-9]{64}$')][string]$PreviousApplicationSha256,
    [ValidatePattern('^[A-Z0-9][A-Za-z0-9_-]{1,95}$')][string]$CaseId = 'NORMAL-PROFILE-LONG-HOST-UPGRADE-v1',
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
$backupPath = $receiptPath + '.backup'
if ((Test-Path -LiteralPath $receiptPath) -or (Test-Path -LiteralPath $backupPath)) { throw 'Retained upgrade evidence already exists' }
$installer = Join-Path $taskRoot 'target/release/bundle/nsis/Auralis_0.1.0_x64-setup.exe'
$installRoot = Join-Path $env:LOCALAPPDATA 'Programs\Auralis'
$application = Join-Path $installRoot 'auralis-app.exe'
if ((Get-PinnedSha256 $installer) -ne $InstallerSha256 -or (Get-PinnedSha256 $application) -ne $PreviousApplicationSha256) { throw 'Upgrade candidate or previous installed identity differs' }
$clock = [Diagnostics.Stopwatch]::StartNew()
$receipt = [ordered]@{
    version=1;case_id=$CaseId;started_at=[DateTime]::UtcNow.ToString('o')
    installer=$installer;installer_sha256=$InstallerSha256;application=$application;expected_application_sha256=$ApplicationSha256
    previous_application_sha256=$PreviousApplicationSha256;backup=$backupPath;owned_pids=@();phase='preflight';error=$null
    script_sha256=(Get-PinnedSha256 $PSCommandPath)
    limitation='Local existing-profile upgrade only; no clean Windows, native workflow or inference acceptance. Does not launch the application or models.'
}
function Save-Receipt {
    $receipt.ended_at = [DateTime]::UtcNow.ToString('o')
    $receipt.elapsed_ms = $clock.ElapsedMilliseconds
    [IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
}
Save-Receipt
try {
    $taskNode = (Get-Command node -ErrorAction Stop).Source
    $snapshot = Join-Path $PSScriptRoot 'profile-snapshot.mjs'
    $profile = & $taskNode --disable-warning=ExperimentalWarning $snapshot inspect | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $null -eq $profile -or $profile.projects -ne 0 -or $profile.active_jobs -ne 0) { throw 'Upgrade expects the owned empty idle native-test profile; inspect changed user state before stopping the app' }
    $receipt.profile_before = $profile
    $processes = @(Get-CimInstance Win32_Process -ErrorAction Stop)
    $owned = @($processes | Where-Object { $_.ExecutablePath -eq $application })
    if ($owned.Count -gt 1) { throw 'Multiple application owners are ambiguous' }
    if ($owned.Count -eq 1) {
        $ids = @([int]$owned[0].ProcessId)
        do {
            $children = @($processes | Where-Object { [int]$_.ParentProcessId -in $ids -and [int]$_.ProcessId -notin $ids } | ForEach-Object { [int]$_.ProcessId })
            $ids += $children
        } while ($children.Count)
        $receipt.owned_pids = $ids
        $receipt.phase = 'stopping_owned_idle_app'; Save-Receipt
        $current = Get-CimInstance Win32_Process -Filter ('ProcessId=' + $owned[0].ProcessId) -ErrorAction Stop
        if ($current.ExecutablePath -ne $application -or (Get-PinnedSha256 $application) -ne $PreviousApplicationSha256) { throw 'Application owner changed before stop' }
        $stop = [Diagnostics.ProcessStartInfo]::new()
        $stop.FileName = Join-Path $env:WINDIR 'System32/taskkill.exe'
        $stop.Arguments = '/PID ' + $owned[0].ProcessId + ' /T /F'
        $stop.UseShellExecute=$false; $stop.CreateNoWindow=$true
        $terminator = [Diagnostics.Process]::Start($stop)
        if (-not $terminator.WaitForExit(30000) -or $terminator.ExitCode -ne 0) { throw 'Owned idle application stop was not confirmed' }
        if (@(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { [int]$_.ProcessId -in $ids }).Count) { throw 'An owned application descendant remains; stop before installation' }
    }
    $receipt.phase='backing_up_profile'; Save-Receipt
    & $taskNode --disable-warning=ExperimentalWarning $snapshot backup $backupPath
    if ($LASTEXITCODE -ne 0) { throw 'Profile backup failed; installation was not started' }
    Copy-Item -LiteralPath $application -Destination (Join-Path $backupPath 'previous-auralis-app.exe')
    if ((Get-PinnedSha256 (Join-Path $backupPath 'previous-auralis-app.exe')) -ne $PreviousApplicationSha256) { throw 'Previous executable copy differs' }
    $receipt.phase='installing'; Save-Receipt
    $process = Start-Process -FilePath $installer -ArgumentList @('/S', ('/D=' + $installRoot)) -PassThru -WindowStyle Hidden
    $receipt.installer_pid=$process.Id; Save-Receipt
    if (-not $process.WaitForExit(600000)) { throw 'Upgrade exceeded 10 minutes; inspect the owned installer before retrying' }
    $receipt.installer_exit_code=$process.ExitCode
    if ($process.ExitCode -ne 0) { throw ('Installer exit ' + $process.ExitCode) }
    $receipt.application_sha256=Get-PinnedSha256 $application
    if ($receipt.application_sha256 -ne $ApplicationSha256) { throw 'Upgraded application differs from pinned NSIS payload' }
    $receipt.phase='installed_verified'
} catch { $receipt.error=$_.Exception.Message; $receipt.phase='failed' }
Save-Receipt
Write-Output ($receipt | ConvertTo-Json -Depth 6)
if ($receipt.error) { throw $receipt.error }
