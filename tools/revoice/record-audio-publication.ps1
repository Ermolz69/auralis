$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$retainedRoot = Join-Path $projectRoot '.cache/revoice-checks/audio-publication-v1'
$receiptPath = Join-Path $projectRoot 'tools/revoice/evidence/AUDIO_LAYOUT_PUBLICATION_V1.json'
if ([IO.Directory]::Exists($retainedRoot) -or [IO.File]::Exists($receiptPath)) { throw 'Publication evidence already exists; preserve it' }

function Get-Sha([string]$FilePath) {
    $stream = [IO.File]::OpenRead($FilePath)
    $digest = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($digest.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
    finally { $digest.Dispose(); $stream.Dispose() }
}

$sourcePaths = @(
    'Taskfile.yml', 'taskfiles/rust.yml', 'taskfiles/frontend.yml',
    'crates/adapters-tts/src/lib.rs', 'crates/adapters-tts/src/execute.rs',
    'crates/adapters-tts/src/result_verification.rs', 'crates/adapters-tts/src/audio_layout.rs',
    'crates/adapters-tts/src/output_layout.rs', 'crates/adapters-tts/src/output_layout/tests.rs',
    'crates/adapters-tts/src/verification_control.rs', 'crates/adapters-tts/src/workflow.rs',
    'crates/application/src/usecases/speech/mod.rs', 'crates/application/src/usecases/speech/run.rs',
    'crates/application/src/usecases/speech/publication.rs', 'crates/application/tests/speech_workflow.rs',
    'crates/application/tests/support/speech_layout_publication.rs',
    'crates/adapters-storage/src/sqlite/speech_run_repository/workflow.rs',
    'crates/adapters-storage/tests/speech_publication.rs',
    'apps/desktop/src/shared/api/contracts/speechValidation.ts',
    'apps/desktop/src/shared/api/contracts/speechValidation.test.ts',
    'docs/integration/revoice-audio-publication.md', 'docs/integration/revoice-plan.json',
    'docs/integration/revoice-execution.md', 'tools/revoice/record-audio-publication.ps1'
)
$sources = @($sourcePaths | ForEach-Object {
    $sourcePath = Join-Path $projectRoot $_
    [ordered]@{ file = $_; bytes = ([IO.FileInfo]$sourcePath).Length; sha256 = Get-Sha $sourcePath }
})
$expectedLogs = @(
    @('audio-publication-checks-v1.log', 'passed', 'test result: ok. 11 passed'),
    @('audio-publication-ui-v1.log', 'failed', '3221225477'),
    @('audio-publication-ui-v2.log', 'passed', '3 passed'),
    @('audio-publication-frontend-build-v1.log', 'passed', 'built in'),
    @('audio-publication-clippy-v1.log', 'passed', 'Finished'),
    @('audio-publication-desktop-v1.log', 'passed', 'Finished'),
    @('audio-publication-size-v1.log', 'passed', 'All file sizes'),
    @('audio-publication-docs-v1.log', 'passed', 'Documentation contract is valid'),
    @('audio-publication-plan-v1.log', 'passed', '24 mandatory cases; status active')
)
$logs = @($expectedLogs | ForEach-Object {
    $logPath = Join-Path $projectRoot ('target/revoice/' + $_[0])
    if (([IO.FileInfo]$logPath).Length -gt 2MB) { throw 'Check log exceeds the retained byte limit' }
    $content = [IO.File]::ReadAllText($logPath)
    if (!$content.Contains($_[2])) { throw ('Expected completed check marker absent: ' + $_[0]) }
    if ($_[1] -eq 'passed' -and $content -match 'Failed to run task|test result: FAILED|error\[E\d+\]') { throw ('Check failed: ' + $_[0]) }
    [ordered]@{ file = $_[0]; outcome = $_[1]; bytes = ([IO.FileInfo]$logPath).Length; sha256 = Get-Sha $logPath; retained_path = '.cache/revoice-checks/audio-publication-v1/' + $_[0] }
})
[IO.Directory]::CreateDirectory($retainedRoot) | Out-Null
foreach ($log in $logs) {
    $destination = Join-Path $retainedRoot $log.file
    [IO.File]::Copy((Join-Path $projectRoot ('target/revoice/' + $log.file)), $destination, $false)
    if ((Get-Sha $destination) -ne $log.sha256) { throw 'Retained check log hash differs' }
}
$receipt = [ordered]@{
    version = 1; case_id = 'AUDIO-LAYOUT-PUBLICATION-MODEL-FREE-v1'; at = [DateTime]::UtcNow.ToString('o')
    base_commit = '16dfa78588d461c8c4978cdf886f54f7c136fc05'; status = 'model_free_checks_passed'
    checks = [ordered]@{ layout = 11; assembly = 19; tts_library = 10; storage_publication = 2; application_workflow = 11; ui_contract = 2; ui_workflow = 3 }
    initial_failures = @(
        'Frontend formatter rejected an extra closing brace in the added test at line 55; corrected before running tests. Formatter output remains in the tool transcript.',
        'The first sandboxed Vitest process exited 3221225477 (0xC0000005) before test results. The unchanged test code passed outside sandbox; root cause was not separately established.'
    )
    models_loaded = $false; real_ffmpeg_invoked = $false; native_ui_pass = $false
    installed_runtime_changed = $false; voice_references_or_qualification_changed = $false
    limitations = 'Synthetic PCM and fixture execution only. Managed layout publication is connected; durable assembly jobs, timing controls, real worker/native flow, video, natural stories, listening and clean Windows remain pending.'
    sources = $sources; logs = $logs
}
$json = (($receipt | ConvertTo-Json -Depth 20) -replace "`r`n", "`n") + "`n"
$encoding = New-Object Text.UTF8Encoding($false)
[IO.File]::WriteAllText($receiptPath, $json, $encoding)
[IO.File]::WriteAllText((Join-Path $retainedRoot 'receipt.json'), $json, $encoding)
Write-Output ('Audio publication evidence: {0} source hashes, {1} retained logs' -f $sources.Count, $logs.Count)
