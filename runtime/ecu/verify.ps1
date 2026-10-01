param(
    [Parameter(Mandatory = $true)][string]$BuildDirectory,
    [int]$TimeoutSeconds = 60
)
$ErrorActionPreference = 'Stop'
if ($TimeoutSeconds -lt 1) { throw 'Verification timeout must be positive.' }
$buildTimeoutSeconds = 180
$sourceRoot = $PSScriptRoot
$inputs = Get-Content -LiteralPath (Join-Path $sourceRoot 'verification/inputs.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($inputs.format -ne 'autosar-ecu-test-inputs-v1') { throw 'Unsupported verification input format.' }
$buildScript = (Join-Path $sourceRoot 'build.ps1').Replace("'", "''")
$buildOutput = $BuildDirectory.Replace("'", "''")
$buildCommand = "& '$buildScript' -OutputDirectory '$buildOutput' -HostBatch"
Add-Type -Path (Join-Path $sourceRoot 'process-tree.cs')
$buildResult = [Autosar.EcuProcessTree]::Run($buildCommand, $buildTimeoutSeconds)
Write-Output $buildResult.Stdout
if ($buildResult.ExitCode -ne 0) { throw "Independent HostBatch build failed: $($buildResult.Stderr)" }
$binary = Join-Path ([IO.Path]::GetFullPath($BuildDirectory)) 'ecu_host_batch.exe'
$watch = [Diagnostics.Stopwatch]::StartNew()
$script:actor = $null
$script:epoch = [uint64]0
$script:records = [Collections.Generic.List[string]]::new()

function Require([bool]$accepted, [string]$message) {
    if (-not $accepted) { throw $message }
}
function Next-Line {
    Require ($watch.Elapsed.TotalSeconds -lt $TimeoutSeconds) 'Independent behavior verification exceeded its deadline.'
    $read = $script:actor.StandardOutput.ReadLineAsync()
    Require ($read.Wait(7000)) 'HostBatch did not produce its bounded response.'
    Require ($null -ne $read.Result) 'HostBatch closed before its expected response.'
    $script:records.Add($read.Result)
    return $read.Result
}
function Start-Actor {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $binary
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $script:actor = [Diagnostics.Process]::Start($info)
    $script:epoch = [uint64]0
    Require ((Next-Line) -eq 'READY HostBatchV1') 'Actual HostBatch entry did not become ready.'
}
function End-Actor {
    $script:actor.StandardInput.Close()
    $closed = $false
    while (-not $closed) {
        $line = Next-Line
        $closed = $line.StartsWith('lifecycle=Closed ')
        if ($closed) { Require ($line.Contains('state=Ready reason=0 ')) 'HostBatch did not close cleanly.' }
    }
    Require ($script:actor.WaitForExit(5000)) 'HostBatch did not exit after close.'
    Require ($script:actor.ExitCode -eq 0) 'HostBatch exit status differs.'
    Require ([string]::IsNullOrEmpty($script:actor.StandardError.ReadToEnd())) 'HostBatch emitted an unexpected stderr failure.'
    $script:actor.Dispose()
    $script:actor = $null
}
function Commit([uint64]$at, [string[]]$messages, [bool]$errorExpected = $false) {
    $outputs = [Collections.Generic.List[string]]::new()
    $step = [uint64][math]::Min(1000, $period * 128)
    while ($at - $script:epoch -gt $step) {
        foreach ($prior in (Commit ($script:epoch + $step) @())) { $outputs.Add($prior) }
    }
    $script:actor.StandardInput.WriteLine("BEGIN $at")
    foreach ($message in $messages) { $script:actor.StandardInput.WriteLine($message) }
    $script:actor.StandardInput.WriteLine('COMMIT')
    $script:actor.StandardInput.Flush()
    while ($true) {
        $line = Next-Line
        if ($line.StartsWith('OUT ')) { $outputs.Add($line); continue }
        $prefix = if ($errorExpected) { 'COMMIT_ERROR ' } else { 'COMMIT_OK ' }
        Require ($line.StartsWith($prefix)) "Unexpected batch receipt: $line"
        Require ($line -match " epoch=$at ") "Receipt epoch differs: $line"
        if ($errorExpected) {
            Require ($line -match "transport_status=10 transport_epoch=$at transport_count=1") "Expected actual N_Cr timeout is absent: $line"
        } else {
            Require ($line -match 'status=0 input_status=0 transport_status=0 ') "Batch unexpectedly failed: $line"
        }
        $script:epoch = $at
        return ,$outputs.ToArray()
    }
}
function Expect-Batch([string[]]$outputs, [uint64]$from, [uint64]$at, [string]$txBytes, [string]$responseBytes = '') {
    $expected = [Collections.Generic.List[string]]::new()
    $actual = [Collections.Generic.List[string]]::new()
    $next = [uint64]([math]::Floor($from / $period) + 1) * $period
    while ($next -le $at) { $expected.Add("$next|$($inputs.transmitCanId)|4|$txBytes"); $next += $period }
    if ($responseBytes) { $expected.Add("$at|$($inputs.responseCanId)|8|$responseBytes") }
    foreach ($output in $outputs) {
        Require ($output -match '^OUT epoch=(\d+) sequence=\d+ ticket=\d+ pdu=\d+ id=(\d+) dlc=(\d+) data=([0-9a-f]+)$') "Malformed output: $output"
        $actual.Add("$($matches[1])|$($matches[2])|$($matches[3])|$($matches[4])")
    }
    Require ($expected.Count -eq $actual.Count) "Unexpected output count: expected=$($expected.Count); actual=$($outputs -join ';')"
    if ($expected.Count -gt 0) {
        $difference = @(Compare-Object ($expected.ToArray() | Sort-Object) ($actual.ToArray() | Sort-Object))
        Require ($difference.Count -eq 0) "Independent complete output set differs: $($outputs -join ';')"
    }
}
$period = [uint64]$inputs.periodMs
Require ($period -gt 0) 'The configured application period is absent.'
$did = '{0:X4}' -f [uint16]$inputs.did
$request = "0322${did}00000000"
$response = "0762${did}12345678".ToLowerInvariant()
$initial = ([BitConverter]::GetBytes([uint32]$inputs.initialReceiveValue) | ForEach-Object { $_.ToString('x2') }) -join ''
try {
    Start-Actor
    $outputs = Commit $period @("RX $($inputs.receiveCanId) 4 78563412")
    Expect-Batch $outputs 0 $period '78563412'
    $outputs = Commit ($period + 1) @("RX $($inputs.receiveCanId) 4 78563412", "RX $($inputs.requestCanId) 8 $request")
    Expect-Batch $outputs $period ($period + 1) '78563412' $response
    End-Actor

    Start-Actor
    $outputs = Commit 1 @("RX $($inputs.requestCanId) 8 100922${did}${did}12")
    Expect-Batch $outputs 0 1 $initial '3000000000000000'
    $deadline = [uint64]1 + [uint64]$inputs.receiveTimeoutMs
    $outputs = Commit $deadline @() $true
    Expect-Batch $outputs 1 $deadline $initial
    # Fresh source bytes precede the next actual application deadline and DID read.
    $recovery = (($deadline / $period) + 1)
    $recovery = [uint64][math]::Floor($recovery) * $period
    $outputs = Commit $recovery @("RX $($inputs.receiveCanId) 4 78563412")
    Expect-Batch $outputs $deadline $recovery '78563412'
    $outputs = Commit ($recovery + 1) @("RX $($inputs.receiveCanId) 4 78563412", "RX $($inputs.requestCanId) 8 $request")
    Expect-Batch $outputs $recovery ($recovery + 1) '78563412' $response
    End-Actor

    Start-Actor
    $script:actor.StandardInput.WriteLine('BEGIN -1')
    $script:actor.StandardInput.Flush()
    $rejected = Next-Line
    Require ($rejected.StartsWith('REJECT ') -and $rejected.Contains('epoch=0 sequence=0')) 'Malformed epoch was not rejected before work.'
    End-Actor
    Write-Output 'ECU_HANDOFF_VERIFY PASS: CAN/DID echo, real N_Cr timeout/recovery, malformed admission; host behavior only.'
} finally {
    if ($null -ne $script:actor) {
        if (-not $script:actor.HasExited) { $script:actor.Kill(); $null = $script:actor.WaitForExit(5000) }
        $script:actor.Dispose()
    }
}
