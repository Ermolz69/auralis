param([Parameter(Mandatory=$true)][string]$Output, [string]$CaseId='NORMAL-PROFILE-TRANSLATE-ASSETS-v3')
$ErrorActionPreference = 'Stop'
function Get-PinnedSha256([string]$File) {
    $algorithm = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($File)
    try { return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $algorithm.Dispose() }
}
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$receiptPath = [IO.Path]::GetFullPath((Join-Path $taskRoot $Output))
if (Test-Path -LiteralPath $receiptPath) { throw 'Retained asset receipt already exists' }
$releasePath = Join-Path $taskRoot 'modules/auralis-translate/models/releases/hy_mt2_1_8b_q4_k_m.windows_x64_cpu.experimental.json'
$release = Get-Content -LiteralPath $releasePath -Raw | ConvertFrom-Json
$assetSource = Join-Path (Split-Path -Parent $taskRoot) 'auralis-translate/.cache/install-sources'
$destination = Join-Path $env:APPDATA 'com.auralis.desktop/cache/translation-downloads'
$started = [DateTime]::UtcNow.ToString('o')
$clock = [Diagnostics.Stopwatch]::StartNew()
$records = @()
$admission = $null
$admitted = $false
$failure = $null
try {
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = 'powershell.exe'
    $guard = Join-Path $taskRoot 'crates/common/src/model_admission.ps1'
    $start.Arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + $guard + '" -ModelFree'
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $admission = [Diagnostics.Process]::Start($start)
    $line = $admission.StandardOutput.ReadLineAsync()
    if (-not $line.Wait(30000)) { throw 'Shared admission did not return within 30 seconds' }
    $answer = $line.Result
    if ($answer -ne 'READY') { throw ('Shared admission refused: ' + $answer) }
    $admitted = $true
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    $assets = @($release.model.file, $release.model.license.notice, $release.runtime.license.notice, $release.runtime.variants[0].archive)
    foreach ($asset in $assets) {
        $source = Join-Path $assetSource $asset.filename
        $target = Join-Path $destination $asset.filename
        if ((Get-Item -LiteralPath $source).Length -ne $asset.bytes -or (Get-PinnedSha256 $source) -ne $asset.sha256) { throw ('Source pin differs: ' + $asset.filename) }
        $copied = $false
        if (-not (Test-Path -LiteralPath $target)) {
            [IO.File]::Copy($source, $target, $false)
            $copied = $true
        }
        if ((Get-Item -LiteralPath $target).Length -ne $asset.bytes -or (Get-PinnedSha256 $target) -ne $asset.sha256) { throw ('Profile cache differs; retained without overwriting: ' + $asset.filename) }
        $records += [ordered]@{filename=$asset.filename;source=$source;target=$target;bytes=$asset.bytes;sha256=$asset.sha256;copied=$copied}
    }
} catch {
    $failure = $_.Exception.Message
} finally {
    if ($admission -and -not $admission.HasExited) {
        $admission.StandardInput.WriteLine('release')
        $admission.StandardInput.Flush()
        $admission.StandardInput.Close()
        if (-not $admission.WaitForExit(30000)) { $failure = 'Shared admission release could not be confirmed; inspect the retained guardian process before any model work' }
    }
    $receipt = [ordered]@{
        version=1;case_id=$CaseId;started_at=$started;ended_at=[DateTime]::UtcNow.ToString('o');elapsed_ms=$clock.ElapsedMilliseconds
        command='task revoice:translate:seed-assets';release_id=$release.id;release_sha256=(Get-PinnedSha256 $releasePath)
        admission_pid=if ($admission) {$admission.Id} else {$null};admitted=$admitted;admission_released=if ($admission) {$admission.HasExited} else {$false}
        script_sha256=(Get-PinnedSha256 $PSCommandPath);assets=$records;error=$failure
        status=if ($failure) {'failed'} else {'verified_assets_copied'}
        limitation='Preseeded existing local assets only, not model inference, installed package selection, native translation or clean Windows acceptance. Original files unchanged.'
    }
    [IO.Directory]::CreateDirectory((Split-Path -Parent $receiptPath)) | Out-Null
    [IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
}
Write-Output ($receipt | ConvertTo-Json -Depth 8)
if ($failure) { throw $failure }
