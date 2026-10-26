$ErrorActionPreference = 'Stop'
$fixtureCommand = if ($env:AURALIS_TEST_JOB_MODE -eq 'linger') { 'Start-Sleep -Seconds 60' } else { 'Start-Sleep -Milliseconds 100' }
$fixtureChild = Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', $fixtureCommand) -PassThru -WindowStyle Hidden
[Console]::Out.WriteLine($fixtureChild.Id)
[Console]::Out.Flush()
if ($env:AURALIS_TEST_JOB_MODE -eq 'finished') { $fixtureChild.WaitForExit() }
exit 0
