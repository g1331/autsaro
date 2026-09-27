param([string]$ReportPath)

$ErrorActionPreference = 'Stop'
$bundle = $PSScriptRoot
if (-not $ReportPath) {
    $ReportPath = Join-Path ([IO.Path]::GetTempPath()) ('autosar-reference-' + [guid]::NewGuid().ToString('N') + '.json')
}
$invalidReportLocation = $false
$absoluteReportPath = [IO.Path]::GetFullPath($ReportPath)
$absoluteBundlePath = [IO.Path]::GetFullPath($bundle).TrimEnd('\', '/')
if ($absoluteReportPath.StartsWith($absoluteBundlePath + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
    $absoluteReportPath.Equals($absoluteBundlePath, [StringComparison]::OrdinalIgnoreCase)) {
    $invalidReportLocation = $true
}
$reportProbe = Split-Path -Parent $absoluteReportPath
while ($reportProbe) {
    if (Test-Path -LiteralPath $reportProbe) {
        if (((Get-Item -LiteralPath $reportProbe -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            $invalidReportLocation = $true
            break
        }
    }
    $nextProbe = Split-Path -Parent $reportProbe
    if ($nextProbe -eq $reportProbe) { break }
    $reportProbe = $nextProbe
}
if (Test-Path -LiteralPath $absoluteReportPath) { $invalidReportLocation = $true }
if ($invalidReportLocation) {
    $ReportPath = Join-Path ([IO.Path]::GetTempPath()) ('autosar-reference-' + [guid]::NewGuid().ToString('N') + '.json')
}
$report = [ordered]@{
    format = 'autosar-host-reference-report-v1'
    target = 'Windows host virtual ECU; fixed reference profile'
    status = 'failed'
    bundle = $bundle
    powershell = $PSVersionTable.PSVersion.ToString()
    gcc = $null
    checks = @()
    error = $null
}
$scratch = $null

function Add-Check([string]$name, [string]$status, [string]$detail) {
    $script:report.checks += [ordered]@{ name = $name; status = $status; detail = $detail }
}

function Get-Sha256([string]$path) {
    $stream = [IO.File]::OpenRead($path)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($hasher.ComputeHash($stream)).Replace('-', '').ToLowerInvariant())
    } finally {
        $hasher.Dispose()
        $stream.Dispose()
    }
}

function Invoke-Child([string]$executable, [string]$arguments, [string]$inputText, [int]$timeoutMs) {
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = $executable
    $start.Arguments = $arguments
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $start
    if (-not $process.Start()) { throw "Could not start $executable" }
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $inputBytes = [Text.Encoding]::ASCII.GetBytes($inputText)
        $process.StandardInput.BaseStream.Write($inputBytes, 0, $inputBytes.Length)
        $process.StandardInput.BaseStream.Flush()
        $process.StandardInput.Close()
        if (-not $process.WaitForExit($timeoutMs)) {
            $killer = New-Object Diagnostics.ProcessStartInfo
            $killer.FileName = Join-Path $env:WINDIR 'System32/taskkill.exe'
            $killer.Arguments = "/PID $($process.Id) /T /F"
            $killer.UseShellExecute = $false
            $killer.CreateNoWindow = $true
            $killProcess = [Diagnostics.Process]::Start($killer)
            if (-not $killProcess.WaitForExit(5000)) { $killProcess.Kill() }
            $killProcess.Dispose()
            if (-not $process.HasExited) { $process.Kill() }
            $process.WaitForExit()
            throw "Child timed out after $timeoutMs ms: $executable"
        }
        $result = [ordered]@{
            code = $process.ExitCode
            stdout = $stdout.Result
            stderr = $stderr.Result
        }
        return $result
    } finally {
        $process.Dispose()
    }
}

function Assert-Manifest([string]$root, [string]$listName, [string]$hashName) {
    $list = Get-Content -LiteralPath (Join-Path $root $listName) -Raw -Encoding UTF8
    $lines = @($list -split "`n" | Where-Object { $_ -ne '' })
    if ($lines.Count -eq 0 -or $list -ne (($lines -join "`n") + "`n")) {
        throw "Invalid manifest: $listName"
    }
    for ($index = 1; $index -lt $lines.Count; $index++) {
        if ([StringComparer]::Ordinal.Compare($lines[$index - 1], $lines[$index]) -ge 0) {
            throw "Unsorted or duplicate manifest: $listName"
        }
    }
    $records = @((Get-Content -LiteralPath (Join-Path $root $hashName) -Encoding UTF8) | Where-Object { $_ -ne '' })
    if ($records.Count -ne ($lines.Count + 1)) { throw "Invalid digest count: $hashName" }
    for ($index = 0; $index -lt $lines.Count; $index++) {
        $name = $lines[$index]
        if ($name -notmatch '^[A-Za-z0-9_./-]+$' -or $name -match '(^|/)\.\.?(/|$)' -or $name.Contains('//')) {
            throw "Unsafe manifest path: $name"
        }
        $path = Join-Path $root $name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing package file: $name" }
        $item = Get-Item -LiteralPath $path -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Linked package file: $name" }
        $parent = Split-Path -Parent $path
        while ($parent -and $parent.Length -gt $root.Length) {
            $directory = Get-Item -LiteralPath $parent -Force
            if (($directory.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Linked package directory: $parent" }
            $parent = Split-Path -Parent $parent
        }
        $actual = Get-Sha256 $path
        if ($records[$index] -cne "$actual  $name") { throw "Digest mismatch: $name" }
    }
    $listHash = Get-Sha256 (Join-Path $root $listName)
    if ($records[$lines.Count] -cne "$listHash  $listName") { throw "Digest mismatch: $listName" }
    return $lines
}

function Assert-NoExtras([string]$root, [string[]]$listed) {
    $allowed = @($listed) + @('reference-files.list', 'reference-files.sha256')
    $pending = New-Object 'System.Collections.Generic.Stack[string]'
    $pending.Push($root)
    while ($pending.Count -gt 0) {
        $directory = $pending.Pop()
        foreach ($item in Get-ChildItem -LiteralPath $directory -Force) {
            $relative = $item.FullName.Substring($root.Length).TrimStart('\', '/').Replace('\', '/')
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Linked package entry: $relative"
            }
            if ($item.PSIsContainer) {
                if (-not @($allowed | Where-Object { $_.StartsWith($relative + '/', [StringComparison]::Ordinal) }).Count) {
                    throw "Unlisted package directory: $relative"
                }
                $pending.Push($item.FullName)
            } elseif ($allowed -cnotcontains $relative) {
                throw "Unlisted package file: $relative"
            }
        }
    }
}

try {
    if ($invalidReportLocation) { throw 'Report path must be outside the reference bundle.' }
    if (((Get-Item -LiteralPath $bundle -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw 'Reference bundle directory is a link.'
    }
    if (-not $IsWindows -and $PSVersionTable.PSEdition -eq 'Core') {
        throw 'This reference targets Windows only.'
    }
    $rootFiles = @(Assert-Manifest $bundle 'reference-files.list' 'reference-files.sha256')
    foreach ($ecu in @('Alpha', 'Beta')) {
        $ecuRoot = Join-Path $bundle $ecu
        $childFiles = @(Assert-Manifest $ecuRoot 'files.list' 'files.sha256')
        foreach ($name in $childFiles + @('files.list', 'files.sha256')) {
            if ($rootFiles -cnotcontains "$ecu/$name") { throw "Root manifest omits $ecu/$name" }
        }
    }
    Assert-NoExtras $bundle $rootFiles
    Add-Check 'package integrity' 'passed' 'Root and both ECU manifests match SHA-256 records.'

    $compiler = Get-Command gcc -CommandType Application -ErrorAction Stop
    $version = Invoke-Child -executable $compiler.Source -arguments '--version' -inputText '' -timeoutMs 15000
    if ($version.code -ne 0) { throw "gcc --version failed: $($version.stderr)" }
    $report.gcc = ($version.stdout -split "`r?`n")[0]
    Add-Check 'compiler' 'passed' $report.gcc

    $vectors = Get-Content -LiteralPath (Join-Path $bundle 'vectors.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($vectors.format -cne 'autosar-host-reference-v1' -or @($vectors.cases).Count -ne 3) {
        throw 'Unsupported reference vector format or case count.'
    }
    $requiredCases = @(
        @{ name = 'Alpha CAN transmit and receive'; ecu = 'Alpha' },
        @{ name = 'Beta CAN receive and transmit'; ecu = 'Beta' },
        @{ name = 'Alpha diagnostic rejection then recovery'; ecu = 'Alpha' }
    )
    $timeoutValue = 0L
    if (-not [long]::TryParse([string]$vectors.timeoutMs, [ref]$timeoutValue) -or
        $timeoutValue -lt 1 -or $timeoutValue -gt 60000) {
        throw 'Reference timeout must be between 1 and 60000 ms.'
    }
    for ($index = 0; $index -lt $requiredCases.Count; $index++) {
        $referenceCase = $vectors.cases[$index]
        if ($referenceCase.name -cne $requiredCases[$index].name -or
            $referenceCase.ecu -cne $requiredCases[$index].ecu -or
            $referenceCase.input -isnot [string] -or
            [string]::IsNullOrWhiteSpace($referenceCase.input) -or
            @($referenceCase.expected).Count -eq 0 -or
            @($referenceCase.expected | Where-Object { $_ -isnot [string] -or $_ -notmatch '^[A-Z] [0-9 ]+([0-9A-F]+)?$' }).Count -ne 0) {
            throw "Invalid fixed reference case at index $index."
        }
    }
    $scratch = Join-Path ([IO.Path]::GetTempPath()) ('autosar-reference-build-' + [guid]::NewGuid().ToString('N'))
    $report.buildDirectory = $scratch
    New-Item -ItemType Directory -Path $scratch -ErrorAction Stop | Out-Null
    foreach ($ecu in @('Alpha', 'Beta')) {
        $sourceRoot = Join-Path $bundle $ecu
        $buildRoot = Join-Path $scratch $ecu
        New-Item -ItemType Directory -Path $buildRoot -ErrorAction Stop | Out-Null
        foreach ($name in @(Get-Content -LiteralPath (Join-Path $sourceRoot 'files.list') -Encoding UTF8)) {
            $target = Join-Path $buildRoot $name
            New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
            Copy-Item -LiteralPath (Join-Path $sourceRoot $name) -Destination $target -ErrorAction Stop
        }
        $sources = @(Get-ChildItem -LiteralPath (Join-Path $buildRoot 'src') -Filter '*.c' -File | Sort-Object Name | ForEach-Object { '"' + $_.FullName + '"' })
        if ($sources.Count -eq 0) { throw "No C99 source files for $ecu" }
        $binary = Join-Path $buildRoot 'ecu_host.exe'
        $arguments = '-std=c99 -Wall -Wextra -Werror -pedantic -I "' + (Join-Path $buildRoot 'include') + '" ' + ($sources -join ' ') + ' "' + (Join-Path $buildRoot 'Ecu_Config.c') + '" -o "' + $binary + '" -lbcrypt'
        $build = Invoke-Child -executable $compiler.Source -arguments $arguments -inputText '' -timeoutMs 120000
        if ($build.code -ne 0 -or -not (Test-Path -LiteralPath $binary -PathType Leaf)) {
            throw "Build failed for $ecu, exit $($build.code): $($build.stderr)"
        }
        Add-Check "$ecu build" 'passed' 'C99 source built in isolated temporary directory.'
    }
    $caseIndex = 0
    foreach ($case in $vectors.cases) {
        $binary = Join-Path (Join-Path $scratch $case.ecu) 'ecu_host.exe'
        $stdinPath = Join-Path $scratch "case-$caseIndex.in"
        [IO.File]::WriteAllBytes($stdinPath, [Text.Encoding]::ASCII.GetBytes($case.input))
        $arguments = '/D /S /C ""' + $binary + '" < "' + $stdinPath + '""'
        $run = Invoke-Child -executable $env:ComSpec -arguments $arguments -inputText '' -timeoutMs ([int]$vectors.timeoutMs)
        if ($run.code -ne 0) { throw "$($case.name): process exited $($run.code); stdout [$($run.stdout)]; stderr [$($run.stderr)]" }
        $actual = @($run.stdout -split "`r?`n" | Where-Object { $_ -ne '' })
        $expected = @($case.expected)
        if (($actual -join "`n") -cne ($expected -join "`n")) {
            throw "$($case.name): expected [$($expected -join '; ')] but got [$($actual -join '; ')]"
        }
        Add-Check $case.name 'passed' ($actual -join '; ')
        $caseIndex++
    }
    $report.status = 'passed'
} catch {
    $report.error = $_.Exception.Message
    Add-Check 'verification' 'failed' $report.error
} finally {
    if ($report.status -eq 'passed' -and $scratch -and (Test-Path -LiteralPath $scratch)) {
        Remove-Item -LiteralPath $scratch -Recurse -Force
    }
    $reportDirectory = Split-Path -Parent $ReportPath
    if ($reportDirectory -and -not (Test-Path -LiteralPath $reportDirectory)) {
        New-Item -ItemType Directory -Path $reportDirectory -Force | Out-Null
    }
    [IO.File]::WriteAllText($ReportPath, ($report | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
    Write-Output "Reference report: $ReportPath"
    Write-Output "Status: $($report.status)"
    if ($report.error) { Write-Error $report.error }
}
if ($report.status -ne 'passed') { exit 1 }
