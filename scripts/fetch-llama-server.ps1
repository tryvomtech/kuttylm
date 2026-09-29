# Downloads the pinned llama.cpp build for Windows (CPU + Vulkan) and copies
# llama-server.exe and its DLLs into src-tauri\resources\llama\.
# Windows finds DLLs next to the .exe, so all files stay in one folder.
#
# Usage (PowerShell):  .\scripts\fetch-llama-server.ps1 [path\to\already-downloaded.zip]
param([string]$Archive = "")
$ErrorActionPreference = "Stop"

$Tag = "b11224"
$Asset = "llama-$Tag-bin-win-vulkan-x64.zip"
$Sha256 = "e6f33c15c472a425d1ba8873ee61f36c4abb494111fb9f21095822a4d3ff63fb"
$Url = "https://github.com/ggml-org/llama.cpp/releases/download/$Tag/$Asset"

$Root = Split-Path -Parent $PSScriptRoot
$Dest = Join-Path $Root "src-tauri\resources\llama-windows"
$Redist = Join-Path $Root "src-tauri\resources\windows-redist"
$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("kuttylm-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $Work | Out-Null

try {
  # The installer carries Microsoft's runtime installer (see src-tauri/windows/hooks.nsh).
  $RedistExe = Join-Path $Redist "vc_redist.x64.exe"
  if (-not (Test-Path $RedistExe)) {
    New-Item -ItemType Directory -Force -Path $Redist | Out-Null
    Write-Host "Downloading Microsoft Visual C++ runtime installer (about 25 MB)..."
    Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vc_redist.x64.exe" -OutFile $RedistExe -UseBasicParsing
  }

  $VersionFile = Join-Path $Dest "VERSION"
  if ((Test-Path $VersionFile) -and ((Get-Content $VersionFile -Raw).Trim() -eq "$Tag-windows")) {
    Write-Host "llama-server $Tag is already in $Dest"
    exit 0
  }

  if (-not $Archive) {
    $Archive = Join-Path $Work $Asset
    Write-Host "Downloading $Asset (about 33 MB)..."
    Invoke-WebRequest -Uri $Url -OutFile $Archive -UseBasicParsing
  }

  $Actual = (Get-FileHash -Algorithm SHA256 $Archive).Hash.ToLower()
  if ($Actual -ne $Sha256) {
    throw "Checksum does not match. The file is damaged or not the pinned version."
  }

  $Unpacked = Join-Path $Work "x"
  Expand-Archive -Path $Archive -DestinationPath $Unpacked
  # The zip may or may not have a top folder.
  $Server = Get-ChildItem -Path $Unpacked -Recurse -Filter "llama-server.exe" | Select-Object -First 1
  if (-not $Server) { throw "llama-server.exe was not found in the zip." }
  $Src = $Server.DirectoryName

  if (Test-Path $Dest) { Remove-Item -Recurse -Force $Dest }
  New-Item -ItemType Directory -Path $Dest | Out-Null
  Copy-Item (Join-Path $Src "llama-server.exe") $Dest
  Copy-Item (Join-Path $Src "*.dll") $Dest
  if (Test-Path (Join-Path $Src "LICENSE*")) { Copy-Item (Join-Path $Src "LICENSE*") $Dest }

  # llama-server and its DLLs need the Visual C++ runtime (msvcp140 / vcruntime140).
  # A clean Windows may not have it, so copy it next to llama-server.exe
  # ("app-local" copy, allowed by Microsoft's redistribution terms).
  foreach ($Dll in "msvcp140.dll", "vcruntime140.dll", "vcruntime140_1.dll") {
    $From = Join-Path $env:SystemRoot "System32\$Dll"
    if (Test-Path $From) {
      Copy-Item $From $Dest
    } else {
      Write-Warning "$Dll was not found. Install the Visual C++ Redistributable on this build machine."
    }
  }
  Set-Content -Path $VersionFile -Value "$Tag-windows"

  & (Join-Path $Dest "llama-server.exe") --version
  $Size = [math]::Round(((Get-ChildItem $Dest | Measure-Object Length -Sum).Sum / 1MB), 1)
  Write-Host "Done: $Size MB in $Dest"
}
finally {
  Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue
}
