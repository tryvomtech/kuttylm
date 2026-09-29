#!/usr/bin/env bash
# For building the Windows installer on Linux (cargo-xwin):
# downloads the pinned llama.cpp Windows build (CPU + Vulkan) into
# src-tauri/resources/llama-windows/, and Microsoft's Visual C++ runtime
# installer into src-tauri/resources/windows-redist/.
# (On Windows, use scripts/fetch-llama-server.ps1 instead.)
set -euo pipefail

TAG="b11224"
ASSET="llama-${TAG}-bin-win-vulkan-x64.zip"
SHA256="e6f33c15c472a425d1ba8873ee61f36c4abb494111fb9f21095822a4d3ff63fb"
URL="https://github.com/ggml-org/llama.cpp/releases/download/${TAG}/${ASSET}"
REDIST_URL="https://aka.ms/vs/17/release/vc_redist.x64.exe"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/resources/llama-windows"
REDIST="$ROOT/src-tauri/resources/windows-redist"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [[ -f "$DEST/VERSION" && "$(cat "$DEST/VERSION")" == "$TAG-windows" ]]; then
  echo "Windows llama-server $TAG is already in $DEST"
else
  echo "Downloading $ASSET (about 33 MB)…"
  curl -fL --retry 3 -o "$WORK/$ASSET" "$URL"
  echo "$SHA256  $WORK/$ASSET" | sha256sum -c --quiet - || {
    echo "ERROR: checksum does not match." >&2
    exit 1
  }
  rm -rf "$DEST" && mkdir -p "$DEST"
  # llama-server.exe, every DLL (backends, libomp) and the licences. Other tools are left out.
  python3 - "$WORK/$ASSET" "$DEST" <<'PY'
import sys, zipfile, os, shutil
archive, dest = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(archive) as z:
    for name in z.namelist():
        base = os.path.basename(name)
        if base == "llama-server.exe" or base.lower().endswith(".dll") or base.startswith("LICENSE"):
            with z.open(name) as src, open(os.path.join(dest, base), "wb") as out:
                shutil.copyfileobj(src, out)
PY
  echo "$TAG-windows" > "$DEST/VERSION"
  echo "Engine: $(ls "$DEST" | wc -l) files, $(du -sh "$DEST" | cut -f1)"
fi

if [[ -f "$REDIST/vc_redist.x64.exe" ]]; then
  echo "Visual C++ runtime installer is already in $REDIST"
else
  mkdir -p "$REDIST"
  echo "Downloading Microsoft Visual C++ runtime installer (about 25 MB)…"
  curl -fL --retry 3 -o "$REDIST/vc_redist.x64.exe" "$REDIST_URL"
  echo "vc_redist.x64.exe sha256: $(sha256sum "$REDIST/vc_redist.x64.exe" | cut -d' ' -f1)"
fi
