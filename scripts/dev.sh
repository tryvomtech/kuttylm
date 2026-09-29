#!/usr/bin/env bash
# Starts `tauri dev` with a clean GTK environment.
#
# Terminals inside snap apps (for example VS Code installed from the Snap Store)
# export GTK_PATH, LOCPATH, GIO_MODULE_DIR and similar variables that point into
# the snap. WebKitGTK then loads the snap's libraries and crashes with:
#   symbol lookup error: /snap/core20/current/lib/x86_64-linux-gnu/libpthread.so.0
# This script removes every such variable that points into /snap/.
set -euo pipefail

cd "$(dirname "$0")/.."

# Variables that must not point into a snap. Unset them if they do.
for var in GTK_PATH GTK_EXE_PREFIX GTK_IM_MODULE_FILE GIO_MODULE_DIR \
  GDK_PIXBUF_MODULE_FILE GDK_PIXBUF_MODULEDIR LOCPATH GSETTINGS_SCHEMA_DIR \
  GIO_LAUNCHED_DESKTOP_FILE; do
  if [[ "${!var:-}" == *"/snap/"* ]]; then
    unset "$var"
  fi
done

# Remove snap entries from colon-separated path lists.
strip_snap() {
  local IFS=':' out=() part
  for part in $1; do
    [[ -n "$part" && "$part" != *"/snap/"* ]] && out+=("$part")
  done
  (IFS=':'; echo "${out[*]}")
}

if [[ -n "${XDG_DATA_DIRS_VSCODE_SNAP_ORIG:-}" ]]; then
  export XDG_DATA_DIRS="$XDG_DATA_DIRS_VSCODE_SNAP_ORIG"
elif [[ -n "${XDG_DATA_DIRS:-}" ]]; then
  export XDG_DATA_DIRS="$(strip_snap "$XDG_DATA_DIRS"):/var/lib/snapd/desktop"
fi
if [[ -n "${XDG_CONFIG_DIRS_VSCODE_SNAP_ORIG:-}" ]]; then
  export XDG_CONFIG_DIRS="$XDG_CONFIG_DIRS_VSCODE_SNAP_ORIG"
fi
if [[ -n "${LD_LIBRARY_PATH:-}" ]]; then
  cleaned="$(strip_snap "$LD_LIBRARY_PATH")"
  if [[ -n "$cleaned" ]]; then export LD_LIBRARY_PATH="$cleaned"; else unset LD_LIBRARY_PATH; fi
fi
unset LD_PRELOAD

export PATH="$HOME/.cargo/bin:$PATH"
exec npm run tauri dev
