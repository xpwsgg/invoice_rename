$ErrorActionPreference = 'Stop'

$arch = $env:PROCESSOR_ARCHITECTURE
$pkg = if ($arch -eq 'ARM64') { 'pdfium-win-arm64.tgz' } else { 'pdfium-win-x64.tgz' }

$root = Resolve-Path "$PSScriptRoot/.."
$out = Join-Path $root 'src-tauri/lib'
New-Item -ItemType Directory -Force -Path $out | Out-Null

$tmp = New-Item -ItemType Directory -Path (Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid()))
try {
    $url = "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$pkg"
    Write-Host "Downloading $url"
    $tgz = Join-Path $tmp 'pdfium.tgz'
    Invoke-WebRequest -Uri $url -OutFile $tgz
    tar -xzf $tgz -C $tmp
    Copy-Item (Join-Path $tmp 'bin/pdfium.dll') (Join-Path $out 'pdfium.dll') -Force
    Write-Host "PDFium installed at $out\pdfium.dll"
} finally {
    Remove-Item -Recurse -Force $tmp
}
