# Copyright 2026 inno-tauri-utils contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

param(
    [Parameter(Mandatory)][string]$Package,
    [Parameter(Mandatory)][string]$Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$basename = $Package.Replace('_', '-') + '-' + $Version
$targets = [ordered]@{
    x86 = 'i686-pc-windows-msvc'
    x64 = 'x86_64-pc-windows-msvc'
}
$checksums = @()

foreach ($arch in $targets.Keys) {
    $dll = "$root/target/$($targets[$arch])/release/$Package.dll"
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $dll).Hash.ToLowerInvariant()
    $checksums += "$hash  $basename-$arch.dll"
    Write-Output "- SHA256 ($arch): $hash"
}

[IO.File]::WriteAllText("$root/target/$basename.sha256", ($checksums -join "`n") + "`n")
