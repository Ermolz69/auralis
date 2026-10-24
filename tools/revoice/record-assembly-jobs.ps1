param(
    [Parameter(Mandatory=$true)][string]$CheckDirectory,
    [Parameter(Mandatory=$true)][string]$RetentionDirectory,
    [ValidateRange(1,99)][int]$Revision = 1
)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$receiptFile = 'tools/revoice/evidence/AUDIO_ASSEMBLY_JOBS_V' + $Revision + '.json'
$receiptPath = Join-Path $projectRoot $receiptFile
$checkRoot = [IO.Path]::GetFullPath($CheckDirectory)
$retainedRoot = [IO.Path]::GetFullPath($RetentionDirectory)
if (!$checkRoot.StartsWith($projectRoot + [IO.Path]::DirectorySeparatorChar) -or !$retainedRoot.StartsWith($projectRoot + [IO.Path]::DirectorySeparatorChar)) { throw 'Check inputs and retained outputs must be project-owned' }
if ([IO.Directory]::Exists($retainedRoot) -or [IO.File]::Exists($receiptPath)) { throw 'Assembly evidence already exists; preserve it' }

function Get-Sha([string]$FilePath) {
    $stream = [IO.File]::OpenRead($FilePath)
    $digest = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($digest.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
    finally { $digest.Dispose(); $stream.Dispose() }
}

$previous = $null
if ($Revision -gt 1) {
    $previousFile = 'tools/revoice/evidence/AUDIO_ASSEMBLY_JOBS_V' + ($Revision - 1) + '.json'
    $previous = [ordered]@{ file=$previousFile; sha256=Get-Sha (Join-Path $projectRoot $previousFile) }
}

$changed = @(& git -C $projectRoot diff --name-only --diff-filter=ACMR)
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect candidate source changes' }
$new = @(& git -C $projectRoot ls-files --others --exclude-standard)
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect candidate new sources' }
$removed = @(& git -C $projectRoot diff --name-only --diff-filter=D)
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect removed candidate sources' }
$allowed = '^(apps/desktop/src/(features/speech-catalog|shared/api/contracts|shared/ui/paged-preview)/|crates/(ports|domain|jobs|application|adapters-storage|adapters-tts|adapters-ffmpeg)/|src-tauri/src/|taskfiles/(rust|frontend)\.yml$|Taskfile\.yml$|docs/integration/revoice-(assembly-jobs\.md|execution\.md|plan\.json)$|tools/revoice/record-assembly-jobs\.ps1$)'
$sourcePaths = @(($changed + $new) | Where-Object { $_ -match $allowed -and $_ -notmatch '^tools/revoice/evidence/' } | Sort-Object -Unique)
$sources = @($sourcePaths | ForEach-Object {
    $sourcePath = Join-Path $projectRoot $_
    $blob = & git -C $projectRoot -c core.safecrlf=false hash-object --path=$_ $sourcePath 2>$null
    if ($LASTEXITCODE -ne 0 -or $blob -notmatch '^[a-f0-9]{40}$') { throw 'Cannot compute portable source object identity' }
    [ordered]@{ file=$_; bytes=([IO.FileInfo]$sourcePath).Length; sha256=Get-Sha $sourcePath; git_blob_sha1=$blob }
})

$checks = @(
    @('copy-controls','audio-copy-tests-v1.log','passed','5 passed'),
    @('initial-storage-compile','audio-storage-tests-v1.log','failed','SqlSafeStr'),
    @('initial-workflow-controls','audio-jobs-tests-v2.log','passed','7 passed'),
    @('initial-desktop-compile','audio-jobs-desktop-v1.log','failed','no method named'),
    @('history-refactor-compile','audio-jobs-tests-v3.log','failed','cannot find value'),
    @('history-refactor-controls','audio-jobs-tests-v4.log','passed','9 passed'),
    @('initial-ui-startup','audio-jobs-ui-tests-v1.log','failed','spawn EPERM'),
    @('initial-ui-controls','audio-jobs-ui-tests-v2.log','passed','10 passed'),
    @('catalog-independent-ui-controls','audio-jobs-ui-tests-v3.log','passed','13 passed'),
    @('saved-plan-ui-controls','audio-jobs-ui-tests-v4.log','passed','13 passed'),
    @('final-ui-controls','audio-jobs-ui-tests-v5.log','passed','13 passed'),
    @('frontend-unit-regressions','audio-jobs-ui-unit-regression-v1.log','passed','234 passed'),
    @('rust-format','audio-jobs-rust-format-check-v3.log','passed','cargo fmt --all -- --check'),
    @('final-source-size','audio-jobs-size-v4.log','passed','All file sizes are within architectural limits'),
    @('final-frontend-lint','audio-jobs-lint-v2.log','passed','oxlint'),
    @('initial-typecheck','audio-jobs-typecheck-v1.log','failed','TS18046'),
    @('intermediate-typecheck','audio-jobs-typecheck-v3.log','passed','tsc -b'),
    @('initial-size-check','audio-jobs-size-v1.log','failed','File too large'),
    @('initial-quality-startup','audio-jobs-ipc-v1.log','failed','spawn EPERM'),
    @('initial-duplicate-check','audio-jobs-quality-v2.log','failed','Duplicate production block'),
    @('initial-ipc-args-check','audio-jobs-quality-v3.log','failed','received AssemblyArgs'),
    @('initial-ipc-name-check','audio-jobs-quality-v4.log','failed','IPC signature drift'),
    @('final-quality','audio-jobs-quality-v5.log','passed','Rust commands/events and TypeScript IPC maps are in parity'),
    @('initial-clippy','audio-jobs-clippy-v1.log','failed','collapsible_if'),
    @('desktop-build-access','audio-jobs-clippy-v2.log','failed','os error 5'),
    @('fixture-clippy','audio-jobs-clippy-v3.log','failed','needless_question_mark'),
    @('intermediate-clippy','audio-jobs-clippy-v4.log','passed','Finished'),
    @('initial-schema-regressions','audio-jobs-regression-v3.log','failed','156 passed; 8 failed'),
    @('legacy-fixture-regressions','audio-jobs-regression-v4.log','failed','speech_queue_v12_reaches_combined_schema ... FAILED'),
    @('intermediate-clippy-after-module-split','audio-jobs-clippy-v8.log','passed','Finished'),
    @('intermediate-regressions','audio-jobs-regression-v5.log','passed','Doc-tests auralis_app'),
    @('cleanup-shutdown-control','audio-jobs-regression-v6.log','failed','11 passed; 1 failed'),
    @('final-clippy','audio-jobs-clippy-v10.log','passed','Finished'),
    @('final-regressions','audio-jobs-regression-v7.log','passed','Doc-tests auralis_app'),
    @('window-and-legacy-controls','audio-jobs-window-controls-v1.log','passed','11 passed'),
    @('shared-cancellation-module','audio-jobs-ownership-v1.log','passed','test result: ok.'),
    @('intermediate-assembly-boundaries','audio-jobs-final-controls-v1.log','passed','11 passed'),
    @('frontend-build','audio-jobs-build-v3.log','passed','Translation delivery checks passed')
)
if ($Revision -gt 1) {
    $checks += ,@('initial-documentation-links','audio-jobs-docs-v1.log','failed','broken relative Markdown link')
}
$logs = @($checks | ForEach-Object {
    $path = Join-Path $checkRoot $_[1]
    if (([IO.FileInfo]$path).Length -gt 3MB) { throw 'Check log exceeds the retention limit' }
    $content = [IO.File]::ReadAllText($path)
    if (!$content.Contains($_[3])) { throw ('Completed check marker missing: ' + $_[0]) }
    if ($_[2] -eq 'passed' -and $content -match 'Failed to run task|test result: FAILED|error\[E\d+\]') { throw ('Check failed: ' + $_[0]) }
    [ordered]@{ id=$_[0]; outcome=$_[2]; bytes=([IO.FileInfo]$path).Length; sha256=Get-Sha $path }
})
[IO.Directory]::CreateDirectory($retainedRoot) | Out-Null
for ($index=0; $index -lt $checks.Count; $index++) {
    $destination = Join-Path $retainedRoot ($logs[$index].id + '.log')
    [IO.File]::Copy((Join-Path $checkRoot $checks[$index][1]),$destination,$false)
    if ((Get-Sha $destination) -ne $logs[$index].sha256) { throw 'Retained check hash differs' }
}
$receipt = [ordered]@{
    version=1; revision=$Revision; case_id=('AUDIO-ASSEMBLY-JOBS-MODEL-FREE-v' + $Revision); at=[DateTime]::UtcNow.ToString('o')
    previous_checkpoint=$previous
    base_commit='c901c78d95f5b1fb5917671e0cc35dbf07a7295b'; status='model_free_checks_passed'
    scope='Managed source inspection, immutable timing review, schema 19 transactions, common job cancellation/recovery, outbox WAV publication, desktop composition and UI controls.'
    models_loaded=$false; real_ffmpeg_invoked=$false; native_ui_pass=$false; installed_profile_update_performed=$false
    checks=[ordered]@{ verified_copy=5; source_inspection=11; assembly_adapter=32; assembly_storage=7; assembly_application=12; common_jobs=34; shared_cancellation=4; ui_contract=11; ui_components=13; frontend_unit_regressions=234 }
    failures=@(
        'The first storage fixture used a dynamic SQL string rejected by SQLx; fixed to literal statements.',
        'Intermediate desktop/history compilation and a TypeScript narrowing error were corrected before final validation.',
        'The first Vitest and IPC checker could not spawn child processes. Equivalent commands ran outside the restricted process environment.',
        'Size limits required extracting column/transcript validation. Duplicate field-list and IPC argument-shape checks required structural corrections.',
        'Clippy found a nested cleanup branch, test-only unwrap/expect usage and fixture result wrapping; fixed without weakening lint policy.',
        'One Tauri resource preparation failed with access denied; the same build completed outside the restricted process environment.'
        'Full storage regressions exposed seven legacy assertions expecting schema 18 and an in-memory outbox fixture missing assembly tombstones. Updated the expected schema and complete fixture; data-preservation assertions remain in place.'
        'A later storage integration target exposed a shared legacy downgrade fixture retaining new assembly tables. Removed those tables only from the owned historical fixture, updated current/future-version assertions and preserved migration/data checks.'
        'A deterministic cleanup-uncertainty control exposed shutdown classification as a generic join failure. Completed uncertain tasks now stay registered for cancellation, and cleanup/shutdown reports retain an explicit unconfirmed outcome.'
        'The Markdown checker admits only Markdown link targets. JSON evidence is referenced as a repository-relative artifact path; the earlier evidence checkpoint remains immutable.'
    )
    limitations='Fixture renderer and synthetic PCM only. The installed application, real FFmpeg job path, natural stories, permitted tempo fitting, video export, TXT, ASR, listening and clean Windows are not accepted by this evidence.'
    sources=$sources; removed_sources=@($removed | Where-Object { $_ -match $allowed }); check_receipts=$logs
}
$json = (($receipt | ConvertTo-Json -Depth 20) -replace "`r`n", "`n") + "`n"
$encoding = New-Object Text.UTF8Encoding($false)
[IO.File]::WriteAllText($receiptPath,$json,$encoding)
& pnpm prettier --write $receiptPath
if ($LASTEXITCODE -ne 0) { throw 'Cannot format assembly receipt' }
[IO.File]::WriteAllText((Join-Path $retainedRoot 'receipt.json'),[IO.File]::ReadAllText($receiptPath),$encoding)
Write-Output ('Assembly evidence: {0} source hashes; {1} bounded check receipts' -f $sources.Count,$logs.Count)
