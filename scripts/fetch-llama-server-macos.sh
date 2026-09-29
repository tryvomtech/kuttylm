#!/usr/bin/env bash
# Downloads the pinned llama.cpp build for Apple Silicon (CPU + Metal) and copies
# only what llama-server needs into src-tauri/resources/llama-macos/.
#
# llama-server finds its libraries in its own folder (@rpath = @loader_path).
# The libraries are copied under their link names (libggml.0.dylib…), as real
# files, not symlinks, so the app bundle and its code signature stay simple.
#
# Runs on macOS and Linux (for a check only; the .dmg must be built on a Mac).
# Usage: bash scripts/fetch-llama-server-macos.sh [path/to/already-downloaded.tar.gz]
set -euo pipefail

TAG="b11224"
ASSET="llama-${TAG}-bin-macos-arm64.tar.gz"
SHA256="d8650ab3e9e7b803df3035f4d8bc00924266f4989644bf4055c2bb7172e9bcc3"
URL="https://github.com/ggml-org/llama.cpp/releases/download/${TAG}/${ASSET}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/resources/llama-macos"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [[ -f "$DEST/VERSION" && "$(cat "$DEST/VERSION")" == "$TAG-macos-arm64" ]]; then
  echo "llama-server $TAG is already in $DEST"
  exit 0
fi

ARCHIVE="${1:-}"
if [[ -z "$ARCHIVE" ]]; then
  ARCHIVE="$WORK/$ASSET"
  echo "Downloading $ASSET (about 12 MB)…"
  curl -fL --retry 3 -o "$ARCHIVE" "$URL"
fi

if command -v sha256sum >/dev/null; then
  ACTUAL="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
else
  ACTUAL="$(shasum -a 256 "$ARCHIVE" | cut -d' ' -f1)"
fi
if [[ "$ACTUAL" != "$SHA256" ]]; then
  echo "ERROR: checksum does not match. The file is damaged or not the pinned version." >&2
  exit 1
fi

tar -xzf "$ARCHIVE" -C "$WORK"
SRC="$WORK/llama-$TAG"

rm -rf "$DEST"
mkdir -p "$DEST"

# The program and every library it loads (by the name it asks for).
FILES=(
  llama-server
  libllama-server-impl.dylib
  libllama-common.0.dylib
  libmtmd.0.dylib
  libllama.0.dylib
  libggml.0.dylib
  libggml-base.0.dylib
  libggml-cpu.0.dylib
  libggml-metal.0.dylib
  libggml-blas.0.dylib
  libggml-rpc.0.dylib
  LICENSE
)
for f in "${FILES[@]}"; do
  cp -L "$SRC/$f" "$DEST/$f"
done
chmod 755 "$DEST/llama-server"
echo "$TAG-macos-arm64" > "$DEST/VERSION"

if [[ "$(uname -s)" == "Darwin" ]]; then
  # A downloaded file can carry the "quarantine" flag; the app must not.
  xattr -cr "$DEST" 2>/dev/null || true
  "$DEST/llama-server" --version 2>&1 | head -1
fi
echo "Done: $(du -sh "$DEST" | cut -f1) in $DEST"
