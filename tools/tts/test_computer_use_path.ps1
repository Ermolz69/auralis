$ErrorActionPreference = 'Stop'
$source = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../crates/common/src/model_admission.ps1'))
$tokens = $null
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($source, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw 'Shared admission script does not parse' }
$assignment = $ast.Find({ param($node) $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -eq '$modelComputerUse' }, $true)
if (-not $assignment) { throw 'Native computer-use classification is missing' }
$expression = $assignment.Right.Extent.Text
$userRoot = Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\runtimes\cua_node'
$suffix = 'bin\node_modules\@oai\sky\bin\windows\codex-computer-use.exe'
$cases = @(
    @{Id='official-windows-app';Name='codex-computer-use.exe';Path=(Join-Path $env:ProgramFiles ('WindowsApps\OpenAI.Codex_26.1016.8120.0_x64__2p2nqsd0c76g0\app\resources\cua_node\' + $suffix));Expected=$true},
    @{Id='official-user-runtime';Name='codex-computer-use.exe';Path=(Join-Path $userRoot ('b2c4f0c90eadfac4\' + $suffix));Expected=$true},
    @{Id='wrong-process-name';Name='llama-server.exe';Path=(Join-Path $userRoot ('b2c4f0c90eadfac4\' + $suffix));Expected=$false},
    @{Id='untrusted-location';Name='codex-computer-use.exe';Path='C:\tmp\codex-computer-use.exe';Expected=$false},
    @{Id='invalid-runtime-id';Name='codex-computer-use.exe';Path=(Join-Path $userRoot ('not-a-runtime\' + $suffix));Expected=$false},
    @{Id='sibling-executable';Name='codex-computer-use.exe';Path=(Join-Path $userRoot 'b2c4f0c90eadfac4\bin\llama-server.exe');Expected=$false}
)
$failed = @()
foreach ($case in $cases) {
    $modelGpuProcess = [pscustomobject]@{Name=$case.Name;ExecutablePath=$case.Path}
    $actual = [bool](Invoke-Expression $expression)
    Write-Output ($case.Id + ': expected=' + $case.Expected + ' actual=' + $actual)
    if ($actual -ne $case.Expected) { $failed += $case.Id }
}
if ($failed.Count) { throw ('Native helper classification failed: ' + ($failed -join ', ')) }
Write-Output 'Six native helper identity controls passed; no admission lease, GPU work or model was started.'
