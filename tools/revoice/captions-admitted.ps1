param([Parameter(Mandatory=$true)][string]$Spec)
$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$case = Get-Content -LiteralPath (Join-Path $taskRoot $Spec) -Raw | ConvertFrom-Json
if ($case.case_id -notmatch '^[A-Za-z0-9_-]{1,96}$') { throw 'Invalid case ID' }
$evidenceRoot = Join-Path $taskRoot ('target/revoice/' + $case.case_id)
[IO.Directory]::CreateDirectory($evidenceRoot) | Out-Null
$receiptPath = Join-Path $evidenceRoot 'admission.json'
if (Test-Path -LiteralPath $receiptPath) { throw 'Retained admission case already exists' }
$guard = $null; $admitted=$false; $failure=$null; $exitCode=$null
try {
    $start=[Diagnostics.ProcessStartInfo]::new()
    $start.FileName='powershell.exe'
    $start.Arguments='-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + (Join-Path $taskRoot 'crates/common/src/model_admission.ps1') + '" -ModelFree'
    $start.UseShellExecute=$false; $start.CreateNoWindow=$true
    $start.RedirectStandardInput=$true; $start.RedirectStandardOutput=$true; $start.RedirectStandardError=$true
    $guard=[Diagnostics.Process]::Start($start)
    $answer=$guard.StandardOutput.ReadLineAsync()
    if (-not $answer.Wait(30000)) { throw 'Shared admission response exceeded 30 seconds' }
    if ($answer.Result -ne 'READY') { throw ('Shared admission refused: ' + $answer.Result) }
    $admitted=$true
    $env:AURALIS_REVOICE_ADMITTED_CASE=$case.case_id
    & node (Join-Path $PSScriptRoot 'captions.mjs') $Spec
    $exitCode=$LASTEXITCODE
    if ($exitCode -ne 0) { throw ('Caption attempt exit ' + $exitCode) }
} catch { $failure=$_.Exception.Message }
finally {
    Remove-Item Env:AURALIS_REVOICE_ADMITTED_CASE -ErrorAction SilentlyContinue
    if ($guard -and -not $guard.HasExited) {
        $guard.StandardInput.WriteLine('release'); $guard.StandardInput.Flush(); $guard.StandardInput.Close()
        if (-not $guard.WaitForExit(30000)) { $failure='Admission release unconfirmed; inspect guardian before other model work' }
    }
    $receipt=@{version=1;case_id=$case.case_id;admitted=$admitted;guardian_pid=if($guard){$guard.Id}else{$null};released=if($guard){$guard.HasExited}else{$false};node_exit_code=$exitCode;error=$failure;ended_at=[DateTime]::UtcNow.ToString('o')}
    [IO.File]::WriteAllText($receiptPath,($receipt|ConvertTo-Json),[Text.UTF8Encoding]::new($false))
    Write-Output ($receipt|ConvertTo-Json)
}
if ($failure) { throw $failure }
