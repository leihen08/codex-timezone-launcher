$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$projectRoot = Split-Path -Parent $PSScriptRoot
$cargoLookup = Get-Command cargo -ErrorAction SilentlyContinue
if ($null -ne $cargoLookup) {
    $cargoCommand = $cargoLookup.Source
}
else {
    $cargoCommand = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (-not (Test-Path -LiteralPath $cargoCommand)) {
        throw "未找到 cargo；请先安装 rustup。"
    }
}
Push-Location $projectRoot
try {
    & $cargoCommand fmt -- --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt failed" }

    & $cargoCommand clippy --locked --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed" }

    & $cargoCommand test --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }

    & $cargoCommand build --release --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }

    $outputDirectory = Join-Path $projectRoot "outputs"
    New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
    $sourceExecutable = Join-Path $projectRoot "target\release\chatgpt-timezone-launcher.exe"
    $outputExecutable = Join-Path $outputDirectory "CodexTimeZoneLauncher.exe"
    Copy-Item -LiteralPath $sourceExecutable -Destination $outputExecutable -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot "README.md") -Destination $outputDirectory -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot "LICENSE") -Destination $outputDirectory -Force

    $hash = (Get-FileHash -LiteralPath $outputExecutable -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  CodexTimeZoneLauncher.exe" |
        Set-Content -LiteralPath (Join-Path $outputDirectory "SHA256SUMS.txt") -Encoding ascii
    Write-Host "Release ready: $outputExecutable"
    Write-Host "SHA-256: $hash"
}
finally {
    Pop-Location
}
