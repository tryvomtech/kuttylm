#!/usr/bin/env bash
# Downloads the pinned llama.cpp build (CPU + Vulkan) and copies only what
# llama-server needs into src-tauri/resources/llama/.
#
# llama-server looks for its libraries and its CPU/Vulkan backends in its own
# folder (RUNPATH $ORIGIN), so all files stay together in one folder.
#
# Usage: bash scripts/fetch-llama-server.sh [path/to/already-downloaded.tar.gz]
set -euo pipefail

TAG="b11224"
ASSET="llama-${TAG}-bin-ubuntu-vulkan-x64.tar.gz"
SHA256="f0686be882aa7dba0ed75b2748c0b64e4ac11a020e22e252620e16b8262e7b40"
URL="https://github.com/ggml-org/llama.cpp/releases/download/${TAG}/${ASSET}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/resources/llama"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [[ -f "$DEST/VERSION" && "$(cat "$DEST/VERSION")" == "$TAG-linux" ]]; then
  echo "llama-server $TAG is already in $DEST"
  exit 0
fi

ARCHIVE="${1:-}"
if [[ -z "$ARCHIVE" ]]; then
  ARCHIVE="$WORK/$ASSET"
  echo "Downloading $ASSET (about 31 MB)…"
  curl -fL --retry 3 -o "$ARCHIVE" "$URL"
fi

echo "$SHA256  $ARCHIVE" | sha256sum -c --quiet - || {
  echo "ERROR: checksum does not match. The file is damaged or not the pinned version." >&2
  exit 1
}

tar -xzf "$ARCHIVE" -C "$WORK"
SRC="$WORK/llama-$TAG"

rm -rf "$DEST"
mkdir -p "$DEST"

# The program, its libraries (by their soname), and all runtime backends.
FILES=(
  llama-server
  libllama-server-impl.so
  libllama-common.so.0
  libmtmd.so.0
  libllama.so.0
  libggml.so.0
  libggml-base.so.0
  libggml-vulkan.so
  LICENSE
)
for f in "${FILES[@]}"; do
  cp -L "$SRC/$f" "$DEST/$f"
done
cp -L "$SRC"/libggml-cpu-*.so "$DEST/"
chmod 755 "$DEST/llama-server"
echo "$TAG-linux" > "$DEST/VERSION"

# Every needed library must be found (inside the folder or on the system).
if ldd "$DEST/llama-server" | grep -q "not found"; then
  ldd "$DEST/llama-server" | grep "not found" >&2
  echo "ERROR: missing libraries (see above)." >&2
  exit 1
fi

"$DEST/llama-server" --version 2>&1 | head -1
echo "Done: $(du -sh "$DEST" | cut -f1) in $DEST"
