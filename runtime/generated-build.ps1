$ErrorActionPreference = 'Stop'

$binary = Join-Path $PSScriptRoot 'ecu_host.exe'
if (Test-Path -LiteralPath $binary) {
    throw "Executable already exists: $binary. Move it before rebuilding."
}

$compiler = if ($env:AUTOSAR_CC) { $env:AUTOSAR_CC } else { 'gcc' }
$include = Join-Path $PSScriptRoot 'include'
$config = Join-Path $PSScriptRoot 'Ecu_Config.c'
$sources = @(Get-ChildItem -LiteralPath (Join-Path $PSScriptRoot 'src') -Filter '*.c' -File |
    Sort-Object Name | ForEach-Object FullName)
if ($sources.Count -eq 0) { throw 'No C99 runtime sources found.' }

$staged = Join-Path $PSScriptRoot ('.ecu_host-' + [guid]::NewGuid().ToString('N') + '.exe')
try {
    & $compiler -std=c99 -Wall -Wextra -Werror -pedantic -I $include @sources $config -o $staged -lbcrypt
    if ($LASTEXITCODE -ne 0) { throw "C99 build failed with exit code $LASTEXITCODE." }
    if (Test-Path -LiteralPath $binary) { throw "Executable already exists: $binary. Move it before rebuilding." }
    Move-Item -LiteralPath $staged -Destination $binary -ErrorAction Stop
} finally {
    if (Test-Path -LiteralPath $staged) { Remove-Item -LiteralPath $staged }
}
