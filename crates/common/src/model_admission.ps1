param([string]$ProbePython, [string]$ProbeScript, [string]$CaseId, [switch]$ModelFree)
$ErrorActionPreference = 'Stop'
$modelMutex = [Threading.Mutex]::new($false, 'Global\PUZA_TTS_NATIVE_RENDER')
$modelLocked = $false
$modelAdmitted = $false
$modelFile = $null
try {
    if ($ModelFree -and $ProbeScript) { throw 'Model-free admission cannot execute a model probe.' }
    try { $modelLocked = $modelMutex.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $modelLocked = $true }
    if (-not $modelLocked) { throw 'Another PUZA model job holds the shared mutex.' }
    $modelRoot = Join-Path $env:LOCALAPPDATA 'PUZA'
    [IO.Directory]::CreateDirectory($modelRoot) | Out-Null
    $modelFile = [IO.File]::Open((Join-Path $modelRoot 'model-server.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    $modelWsl = @(& wsl --list --running --quiet 2>$null)
    if ($LASTEXITCODE -ne 0) { throw 'WSL state cannot be inspected.' }
    if ($modelWsl | Where-Object { ($_ -replace "`0", '').Trim() }) { throw 'A WSL distribution is running; its model state is unknown.' }
    $modelProcesses = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object {
        $_.ProcessId -ne $PID -and (
            $_.Name -match '^(python(w|3(\.\d+)?)?|llama-server|llama-cli|ollama|vllm|text-marker|tts)(\.exe)?$' -or
            $_.Name -match 'docker|^(wsl|wslhost|vmmem(wsl)?)\.exe$' -or
            ($_.Name -notmatch '^(powershell|pwsh|cmake|ninja|git|curl|cl|clang|link|msbuild)\.exe$' -and $_.CommandLine -match 'llama-server|vllm.entrypoints|tts.*server|text.marker')
        )
    })
    if ($modelProcesses.Count) { throw ('Model/job processes present: ' + (($modelProcesses | ForEach-Object { $_.Name + ':' + $_.ProcessId }) -join ', ')) }
    $modelGpuJobs = @(& nvidia-smi --query-compute-apps=pid --format=csv,noheader 2>$null)
    if ($LASTEXITCODE -ne 0) { throw 'GPU process inspection failed.' }
    $modelDesktopNames = @('dwm.exe','explorer.exe','searchapp.exe','phoneexperiencehost.exe','vgtray.exe','lghub_system_tray.exe','discord.exe','chrome.exe','msedge.exe','chatgpt.exe','codex-computer-use-swift.exe','msedgewebview2.exe','textinputhost.exe','telegram.exe','steamwebhelper.exe')
    $modelDesktopNames += @('crossdeviceresume.exe','startmenuexperiencehost.exe','searchhost.exe','lockapp.exe','wallpaper64.exe','armourycratekeycontrol.exe','shellexperiencehost.exe','winword.exe','spotify.exe','applicationframehost.exe','systemsettings.exe','shellhost.exe','githubdesktop.exe')
    $modelDesktopNames += @('taskmgr.exe')
    foreach ($modelGpuJob in $modelGpuJobs) {
        if ($modelGpuJob -match '^\s*(\d+)\s*$') {
            $modelGpuPid = [int]$Matches[1]
            $modelGpuProcess = Get-CimInstance Win32_Process -Filter "ProcessId=$modelGpuPid" -ErrorAction Stop
            $modelSpotifyGamebar = $modelGpuProcess -and $modelGpuProcess.Name -eq 'SpotifyXboxGamebarWebView.exe' -and $modelGpuProcess.ExecutablePath -like (Join-Path $env:ProgramFiles 'WindowsApps\SpotifyAB.SpotifyMusic_*\SpotifyXboxGamebarWebView.exe')
            $modelComputerUse = $modelGpuProcess -and $modelGpuProcess.Name -eq 'codex-computer-use.exe' -and (
                $modelGpuProcess.ExecutablePath -like (Join-Path $env:ProgramFiles 'WindowsApps\OpenAI.Codex_*\app\resources\cua_node\bin\node_modules\@oai\sky\bin\windows\codex-computer-use.exe') -or
                $modelGpuProcess.ExecutablePath -match ('^' + [regex]::Escape((Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\runtimes\cua_node')) + '\\[0-9a-f]{16}\\bin\\node_modules\\@oai\\sky\\bin\\windows\\codex-computer-use\.exe$')
            )
            $modelEditor = $ModelFree -and $modelGpuProcess -and $modelGpuProcess.Name -in @('UnrealEditor.exe','LiveCodingConsole.exe') -and $modelGpuProcess.ExecutablePath -eq (Join-Path $env:ProgramFiles ('Epic Games\UE_5.3\Engine\Binaries\Win64\' + $modelGpuProcess.Name))
            if (-not $modelGpuProcess -or ($modelGpuProcess.Name.ToLowerInvariant() -notin $modelDesktopNames -and -not $modelSpotifyGamebar -and -not $modelComputerUse -and -not $modelEditor)) { throw ('GPU compute owner is unknown or is a model: ' + $modelGpuPid) }
        }
    }
    $modelGpuUsed = @(& nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits 2>$null)
    if ($LASTEXITCODE -ne 0 -or $modelGpuUsed.Count -ne 1 -or $modelGpuUsed[0] -notmatch '^\s*\d+\s*$') { throw 'GPU memory state cannot be established.' }
    if (-not $ModelFree -and [int]$modelGpuUsed[0] -gt 2048) { throw 'GPU baseline exceeds 2048 MiB; model resources may not have been released.' }
    [Console]::Out.WriteLine('READY')
    [Console]::Out.Flush()
    $modelAdmitted = $true
    if ($ProbeScript) {
        if (-not [IO.Path]::IsPathRooted($ProbeScript) -or -not [IO.File]::Exists($ProbeScript) -or -not [IO.File]::Exists($ProbePython)) { throw 'Probe paths are invalid.' }
        $env:PUZA_PROBE_ADMITTED = '1'
        & $ProbePython $ProbeScript --case-id $CaseId
        if ($LASTEXITCODE -ne 0) { throw 'Qualification probe failed; inspect its retained evidence.' }
    } else { [Console]::In.ReadLine() | Out-Null }
    do {
        $modelRemaining = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $_.Name -match '^(python(w|3(\.\d+)?)?|llama-server|llama-cli|ollama|vllm|text-marker|tts)(\.exe)?$' })
        $modelRemainingGpu = @(& nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits 2>$null)
        if ($LASTEXITCODE -ne 0) { throw 'GPU resource release could not be inspected.' }
        $modelReleased = $modelRemaining.Count -eq 0 -and $modelRemainingGpu.Count -eq 1 -and $modelRemainingGpu[0] -match '^\s*\d+\s*$' -and ($ModelFree -or [int]$modelRemainingGpu[0] -le 2048)
        if (-not $modelReleased) { Start-Sleep -Milliseconds 200 }
    } while (-not $modelReleased)
} catch {
    if ($modelAdmitted) {
        # A failed inspection after admission cannot release either shared lease.
        do {
            $modelConfirmedReleased = $false
            try {
                $modelRecoveryProcesses = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $_.Name -match '^(python(w|3(\.\d+)?)?|llama-server|llama-cli|ollama|vllm|text-marker|tts)(\.exe)?$' })
                $modelRecoveryGpu = @(& nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits 2>$null)
                $modelConfirmedReleased = $LASTEXITCODE -eq 0 -and $modelRecoveryProcesses.Count -eq 0 -and $modelRecoveryGpu.Count -eq 1 -and $modelRecoveryGpu[0] -match '^\s*\d+\s*$' -and ($ModelFree -or [int]$modelRecoveryGpu[0] -le 2048)
            } catch { $modelConfirmedReleased = $false }
            if (-not $modelConfirmedReleased) { Start-Sleep -Milliseconds 200 }
        } while (-not $modelConfirmedReleased)
    } else {
        [Console]::Out.WriteLine($_.Exception.Message)
        [Console]::Out.Flush()
    }
    exit 1
} finally {
    if ($modelFile) { $modelFile.Dispose() }
    if ($modelLocked) { $modelMutex.ReleaseMutex() }
    $modelMutex.Dispose()
}
