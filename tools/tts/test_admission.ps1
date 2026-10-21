param([switch]$ModelFree)
$ErrorActionPreference = 'Stop'
$admissionScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../crates/common/src/model_admission.ps1'))
function Start-Admission {
    $admissionStart = New-Object Diagnostics.ProcessStartInfo
    $admissionStart.FileName = 'powershell.exe'
    $admissionStart.Arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + $admissionScript + '"'
    if ($ModelFree) { $admissionStart.Arguments += ' -ModelFree' }
    $admissionStart.UseShellExecute = $false
    $admissionStart.CreateNoWindow = $true
    $admissionStart.RedirectStandardInput = $true
    $admissionStart.RedirectStandardOutput = $true
    $admissionStart.RedirectStandardError = $true
    return [Diagnostics.Process]::Start($admissionStart)
}
$firstAdmission = Start-Admission
try {
    $firstLine = $firstAdmission.StandardOutput.ReadLine()
    if ($firstLine -ne 'READY') { throw ('Admission precondition failed: ' + $firstLine) }
    $secondAdmission = Start-Admission
    try {
        $secondOutput = $secondAdmission.StandardOutput.ReadToEnd()
        if (-not $secondAdmission.WaitForExit(20000) -or $secondAdmission.ExitCode -eq 0 -or $secondOutput -notmatch 'shared mutex') { throw 'A second model lease was not rejected.' }
    } finally { $secondAdmission.Dispose() }
    $firstAdmission.StandardInput.WriteLine('release')
    $firstAdmission.StandardInput.Close()
    if (-not $firstAdmission.WaitForExit(45000) -or $firstAdmission.ExitCode -ne 0) { throw 'Resource release was not confirmed.' }
    Write-Output 'Shared admission rejects a competing owner and releases only after idle GPU/process checks.'
} finally {
    if (-not $firstAdmission.HasExited) { $firstAdmission.StandardInput.Close(); $firstAdmission.WaitForExit(45000) | Out-Null }
    $firstAdmission.Dispose()
}
