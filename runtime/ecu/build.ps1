param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [switch]$TestMode,
    [switch]$HostBatch,
    [string]$ControlSource
)
$ErrorActionPreference = 'Stop'
if ($HostBatch -and $ControlSource) { throw 'Select HostBatch or an independent ControlSource entry, not both.' }

function Get-SourceSha256([string]$LiteralPath) {
    $stream = [System.IO.File]::OpenRead($LiteralPath)
    $hash = [System.Security.Cryptography.SHA256]::Create()
    try {
        return [BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-', '').ToLowerInvariant()
    } finally {
        $hash.Dispose()
        $stream.Dispose()
    }
}

$projectRoot = [System.IO.Path]::GetFullPath($PSScriptRoot)
$buildRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
if ($buildRoot.Equals($projectRoot, [StringComparison]::OrdinalIgnoreCase) -or
    $buildRoot.StartsWith($projectRoot + [System.IO.Path]::DirectorySeparatorChar,
        [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Use a separate new build directory outside the generated source project.'
}
if ((Test-Path -LiteralPath $buildRoot) -and
    @(Get-ChildItem -LiteralPath $buildRoot -Force).Count -ne 0) {
    throw 'Build output must be new or empty; existing owner files are preserved.'
}
$names = @(Get-Content -LiteralPath (Join-Path $projectRoot 'files.list') -Encoding UTF8)
$hashes = @(Get-Content -LiteralPath (Join-Path $projectRoot 'files.sha256') -Encoding UTF8)
if ($hashes.Count -ne ($names.Count + 1)) { throw 'Invalid source hash manifest.' }
function Test-SourceTree([string]$Directory) {
    foreach ($item in @(Get-ChildItem -LiteralPath $Directory -Force)) {
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw 'Source links and reparse points are refused.'
        }
        $relative = $item.FullName.Substring($projectRoot.Length + 1).Replace('\', '/')
        if ($item.PSIsContainer) {
            $prefix = $relative + '/'
            if (@($names | Where-Object { $_.StartsWith($prefix, [StringComparison]::Ordinal) }).Count -eq 0) {
                throw "Unlisted source directory: $relative"
            }
            Test-SourceTree $item.FullName
        } elseif ($relative -ne 'files.list' -and $relative -ne 'files.sha256' -and
            $names -notcontains $relative) {
            throw "Unlisted source file: $relative"
        }
    }
}
Test-SourceTree $projectRoot
foreach ($line in $hashes) {
    if ($line -notmatch '^([0-9a-f]{64})  ([^\:]+)$') { throw 'Invalid source identity.' }
    $expected = $Matches[1]
    $relative = $Matches[2]
    if ([System.IO.Path]::IsPathRooted($relative) -or
        $relative.Split('/') -contains '..' -or $relative.Split('/') -contains '.') {
        throw 'Unsafe source identity.'
    }
    if ($relative -ne 'files.list' -and $names -notcontains $relative) {
        throw 'Hash manifest names an unlisted source.'
    }
    $path = Join-Path $projectRoot $relative
    if ((Get-SourceSha256 $path) -ne $expected) {
        throw "Source identity mismatch: $relative"
    }
}
$compiler = if ($env:AUTOSAR_CC) { $env:AUTOSAR_CC } else { 'gcc' }
$compilerPath = (Get-Command $compiler -CommandType Application -ErrorAction Stop).Source
$toolchain = Get-Content -LiteralPath (Join-Path $projectRoot 'toolchain.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$version = (& $compilerPath --version)[0]
if ($LASTEXITCODE -ne 0) { throw 'Compiler version query failed.' }
$machine = & $compilerPath -dumpmachine
if ($LASTEXITCODE -ne 0) { throw 'Compiler target query failed.' }
if (($version.Substring($version.IndexOf(' ') + 1) -ne
        $toolchain.identity.Substring($toolchain.identity.IndexOf(' ') + 1)) -or
    $machine.Trim() -ne $toolchain.target -or
    (Get-SourceSha256 $compilerPath) -ne $toolchain.sha256) {
    throw 'This target requires the pinned GCC 16.1.0 x86_64-w64-mingw32 compiler.'
}
$null = Get-Command git -CommandType Application -ErrorAction Stop
$null = New-Item -ItemType Directory -Path $buildRoot -Force
$kernelCopy = Join-Path $buildRoot 'kernel'
Copy-Item -LiteralPath (Join-Path $projectRoot 'kernel') -Destination $kernelCopy -Recurse
Push-Location -LiteralPath $kernelCopy
try {
    foreach ($patch in @(Get-ChildItem -LiteralPath (Join-Path $projectRoot 'os/patches') -File -Filter '*.patch' | Sort-Object Name)) {
        & git apply --ignore-space-change --check $patch.FullName
        if ($LASTEXITCODE -ne 0) { throw "Kernel patch check failed: $($patch.Name)" }
        & git apply --ignore-space-change $patch.FullName
        if ($LASTEXITCODE -ne 0) { throw "Kernel patch apply failed: $($patch.Name)" }
    }
} finally {
    Pop-Location
}
$includeFlags = @('-I', (Join-Path $projectRoot 'include'),
    '-I', (Join-Path $projectRoot 'os'), '-I', (Join-Path $projectRoot 'os/include'),
    '-I', (Join-Path $projectRoot 'os/src'), '-I', (Join-Path $kernelCopy 'include'),
    '-I', (Join-Path $kernelCopy 'portable/MSVC-MingW'))
$sources = @($names | Where-Object { $_ -match '^(src|os/src)/[^/]+\.c$' } |
    ForEach-Object { Join-Path $projectRoot $_ })
$sources = @($sources | Where-Object { $_ -ne (Join-Path $projectRoot 'src/ecu_probe.c') -and
    $_ -ne (Join-Path $projectRoot 'src/ecu_host_batch.c') })
if (-not $ControlSource) {
    if ($HostBatch -and $TestMode) { throw 'HostBatch uses production mode; select a separate independent test consumer.' }
    $entry = if ($HostBatch) { 'src/ecu_host_batch.c' } else { 'src/ecu_probe.c' }
    $sources += Join-Path $projectRoot $entry
}
if ($ControlSource) {
    $controlPath = [System.IO.Path]::GetFullPath($ControlSource)
    if (-not (Test-Path -LiteralPath $controlPath -PathType Leaf) -or
        $controlPath.StartsWith($projectRoot + [System.IO.Path]::DirectorySeparatorChar,
            [StringComparison]::OrdinalIgnoreCase)) {
        throw 'The independent control consumer must be an existing file outside the source project.'
    }
    $sources = @($sources | Where-Object { $_ -ne (Join-Path $projectRoot 'src/ecu_probe.c') })
    $sources += $controlPath
}
$sources += @(@('tasks.c', 'list.c', 'queue.c', 'portable/MSVC-MingW/port.c') | ForEach-Object { Join-Path $kernelCopy $_ })
$testFlags = @()
if ($TestMode) { $testFlags += '-DECU_TARGET_TESTS' }
$binary = Join-Path $buildRoot $(if ($HostBatch) { 'ecu_host_batch.exe' } else { 'ecu_probe.exe' })
$compileArguments = @('-std=c99', '-O1', '-g', '-Wall', '-Wextra', '-Werror', '-DECU_TARGET_EPIC4')
$compileArguments += $testFlags
$compileArguments += $includeFlags
$compileArguments += $sources
$compileArguments += @('-lwinmm', '-lbcrypt', '-o', $binary)
if ($compileArguments -contains '' -or $compileArguments -contains '-') {
    throw 'Invalid compiler argument in the generated build.'
}
& $compilerPath @compileArguments
if ($LASTEXITCODE -ne 0) { throw "ECU C99 build failed: $LASTEXITCODE" }
$objdump = Join-Path (Split-Path -Parent $compilerPath) 'objdump.exe'
$symbols = & $objdump -t $binary
if ($LASTEXITCODE -ne 0) { throw 'Native ABI symbol check failed.' }
if ($symbols -match '__emutls') { throw 'Physical stack faults require native PE TLS.' }
$sections = & $objdump -h $binary
if ($LASTEXITCODE -ne 0) { throw 'Native section check failed.' }
$vectors = @($sections | Select-String '^\s*(\d+)\s+\.os_vec\s+([0-9a-fA-F]+)[^\r\n]*$')
if ($vectors.Count -ne 1 -or [Convert]::ToInt32($vectors[0].Matches[0].Groups[2].Value, 16) -ne 256) {
    throw 'The native interrupt vector section is missing or has the wrong size.'
}
$vectorSection = [int]$vectors[0].Matches[0].Groups[1].Value + 1
$vectorFlags = $sections[$vectors[0].LineNumber]
if ($vectorFlags -notmatch 'DATA' -or $vectorFlags -notmatch 'ALLOC' -or $vectorFlags -match 'READONLY') {
    throw 'The native interrupt vector section must be writable allocated data.'
}
$tables = @($symbols | Select-String '\(sec\s+(\d+)\)[^\r\n]*\s0x([0-9a-fA-F]+)\s+Os_InterruptVectorTable$')
if ($tables.Count -ne 1 -or [int]$tables[0].Matches[0].Groups[1].Value -ne $vectorSection -or
    [Convert]::ToInt64($tables[0].Matches[0].Groups[2].Value, 16) -ne 0) {
    throw 'The actual interrupt vector table is not in its dedicated section.'
}
$code = @($sections | Select-String '^\s*(\d+)\s+\.os_code\s+([0-9a-fA-F]+)[^\r\n]*$')
if ($code.Count -ne 1 -or [Convert]::ToInt32($code[0].Matches[0].Groups[2].Value, 16) -le 0) {
    throw 'The OS entry code section is missing.'
}
$codeSection = [int]$code[0].Matches[0].Groups[1].Value + 1
$codeFlags = $sections[$code[0].LineNumber]
if ($codeFlags -notmatch 'CODE' -or $codeFlags -notmatch 'READONLY' -or $codeFlags -notmatch 'ALLOC') {
    throw 'The OS entry section must be allocated read-only executable code.'
}
foreach ($entryName in @('ErrorHook', 'PreTaskHook', 'PostTaskHook', 'StartupHook', 'ShutdownHook')) {
    $entry = @($symbols | Select-String ('\(sec\s+(\d+)\)[^\r\n]*\s' + $entryName + '$'))
    if ($entry.Count -ne 1 -or [int]$entry[0].Matches[0].Groups[1].Value -ne $codeSection) {
        throw "The OS hook is outside its code section: $entryName"
    }
}
$taskEntries = @($symbols | Select-String '\(sec\s+(\d+)\)[^\r\n]*\sOs_TaskEntry_[A-Za-z0-9_]+$')
if ($taskEntries.Count -ne 1 -or [int]$taskEntries[0].Matches[0].Groups[1].Value -ne $codeSection) {
    throw 'The generated Task is outside its code section.'
}
Write-Output "Built ECU native entry: $binary"
