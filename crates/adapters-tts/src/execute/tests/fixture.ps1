$ErrorActionPreference = 'Stop'
$fixtureSequence = 0
function Send-FixtureEvent($fixtureIdentity, [string]$fixtureKind, $fixturePayload) {
    $script:fixtureSequence++
    $packet = [ordered]@{
        protocol_version = 1
        request_id = $fixtureIdentity.request_id
        project_id = $fixtureIdentity.project_id
        run_id = $fixtureIdentity.run_id
        attempt_id = $fixtureIdentity.attempt_id
        event_sequence = $script:fixtureSequence
        event_kind = $fixtureKind
        payload = $fixturePayload
    }
    [Console]::Out.WriteLine(($packet | ConvertTo-Json -Depth 8 -Compress))
    [Console]::Out.Flush()
}
$workerRequest = [Console]::In.ReadLine() | ConvertFrom-Json
Send-FixtureEvent $workerRequest 'started' @{}
if ($env:AURALIS_TEST_TTS_MODE -eq 'cancel') {
    Send-FixtureEvent $workerRequest 'saved_checkpoint' @{
        sequence = 1
        run_id = $workerRequest.run_id
        attempt_id = $workerRequest.attempt_id
        plan_digest = ('a' * 64)
        utterance_id = 'utterance-1'
        cue_id = $null
        chunk_id = 'chunk-1'
        cache_key = ('b' * 64)
        audio_sha256 = ('c' * 64)
        duration_ms = 100
        status = 'generated'
    }
    $cancelRequest = [Console]::In.ReadLine() | ConvertFrom-Json
    Send-FixtureEvent $cancelRequest 'cancelled' @{}
    exit 130
}
foreach ($checkpointIndex in 1..100) {
    Start-Sleep -Milliseconds 100
    Send-FixtureEvent $workerRequest 'saved_checkpoint' @{
        sequence = $checkpointIndex
        run_id = $workerRequest.run_id
        attempt_id = $workerRequest.attempt_id
        plan_digest = ('a' * 64)
        utterance_id = 'utterance-1'
        cue_id = $null
        chunk_id = ('chunk-' + $checkpointIndex)
        cache_key = ('b' * 64)
        audio_sha256 = ('c' * 64)
        duration_ms = 100
        status = 'generated'
    }
}
Start-Sleep -Seconds 30
