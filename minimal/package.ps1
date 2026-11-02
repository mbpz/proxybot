param([string]$Target = 'x86_64-pc-windows-msvc')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$staging = $null
Push-Location $root
try {
    cargo build --release --locked --manifest-path minimal/Cargo.toml --target $Target
    if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed' }
    $dist = Join-Path $root 'minimal/dist'
    New-Item -ItemType Directory -Force $dist | Out-Null
    $binary = Join-Path $root "minimal/target/$Target/release/proxybot.exe"
    if (-not (Test-Path $binary)) { throw "Missing binary: $binary" }
    $asset = "proxybot-min-$Target.zip"
    $archive = Join-Path $dist $asset
    if (Test-Path $archive) { Remove-Item $archive }
    $staging = Join-Path ([System.IO.Path]::GetTempPath()) ("proxybot-package-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force $staging | Out-Null
    Copy-Item $binary (Join-Path $staging 'proxybot.exe')
    Copy-Item 'LICENSE' $staging
    Copy-Item 'minimal/THIRD_PARTY.md' $staging
    Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $archive
    $hash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
    "$hash  $asset" | Set-Content -Encoding ascii "$archive.sha256"
    Write-Output "Built $archive"
} finally {
    if ($staging -and (Test-Path $staging)) { Remove-Item -Recurse -Force $staging }
    Pop-Location
}
