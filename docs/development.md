# KuttyLM — developer guide

For users: see the [README](../README.md).

KuttyLM is built with Tauri 2 (Rust), Svelte 5 + TypeScript, and llama.cpp's `llama-server`
(CPU + Vulkan on Linux and Windows, CPU + Metal on macOS) running in the background.

Supported: **Ubuntu 22.04 and 24.04** and **Windows 10 and 11** (x86_64), and
**macOS 13.3 or newer on Apple Silicon**. OS-specific code lives in `src-tauri/src/platform/`
(`linux.rs`, `windows.rs`, `macos.rs`).

The notes for each system (SmartScreen, the Visual C++ runtime, WebView2, Vulkan drivers,
ad-hoc signing on the Mac) are in "First start" in the [README](../README.md) and in the build sections below.

## Build on GitHub (Linux, Windows and macOS)

- **Release:** push a tag like `v0.1.1` (update `version` in `package.json`,
  `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` first).
  `.github/workflows/release.yml` builds the `.deb`, `.AppImage`, Windows `.exe` and macOS `.dmg`
  and puts them in a **draft** release with the text from `docs/release-notes.md`.
  Check the draft on GitHub, then click **Publish release**.
- **Test build:** **Actions → Build → Run workflow** (`.github/workflows/build.yml`). The files
  appear under the run's **Artifacts**; nothing is published.

## Build tools on Windows (developers)

1. Install [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   with "Desktop development with C++".
2. Install Rust from [rustup.rs](https://rustup.rs) and Node.js 20 or newer.
3. In PowerShell, in the project folder:

```powershell
npm install
.\scripts\fetch-llama-server.ps1    # Windows engine, about 33 MB, checked with sha256
npm run tauri dev                   # dev mode
npm run tauri build                 # installer: src-tauri\target\release\bundle\nsis\
```

## Build on a Mac (developers)

A `.dmg` can only be made on a Mac (it uses Apple's `hdiutil`). You need an Apple Silicon Mac,
the Xcode Command Line Tools (`xcode-select --install`), Rust from [rustup.rs](https://rustup.rs)
and Node.js 20 or newer. Then, in the project folder:

```bash
npm install
bash scripts/fetch-llama-server-macos.sh   # Mac engine (CPU + Metal), about 12 MB
npm run tauri dev                          # dev mode
npm run tauri build                        # src-tauri/target/release/bundle/dmg/KuttyLM_<version>_aarch64.dmg
```

`src-tauri/tauri.macos.conf.json` is added to the main config on a Mac by itself. The app is
signed "ad-hoc" (no Apple Developer account). To remove the "Open Anyway" step for users,
sign and notarise it with an Apple Developer ID (see Tauri's "macOS Code Signing" guide).

On macOS, llama-server is stopped when the app closes. If the app crashes, llama-server can keep
running; stop it in **Activity Monitor**.

## Build tools on Ubuntu (developers)

```bash
sudo apt update
sudo apt install -y build-essential curl wget file pciutils vulkan-tools \
  libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

# Rust (the minimal profile saves about 0.7 GB)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile minimal -y
source "$HOME/.cargo/env"
```

You also need **Node.js 20 or newer** and npm.

Disk space: about 1.5 GB for Rust, and 2–4 GB for build files in `src-tauri/target/`
(`cargo clean` in `src-tauri/` removes them; the next build takes longer).

## First setup

```bash
npm install
bash scripts/fetch-llama-server.sh   # downloads the pinned llama.cpp build (about 31 MB)
```

The script puts `llama-server` and its libraries in `src-tauri/resources/llama/`
(not in git). It checks the download with sha256. To update llama.cpp, change `TAG` and
`SHA256` at the top of the script.

## Run in dev mode

```bash
npm run app
```

The first start builds the Rust part (a few minutes). The window reloads when you change
frontend files; Rust changes restart the app.

`npm run app` runs `scripts/dev.sh`, which removes snap settings (`GTK_PATH`, `LOCPATH`…)
that the VS Code snap terminal passes to programs. Without that, the app can crash with
`symbol lookup error: /snap/core20/…/libpthread.so.0`. From a normal terminal,
`npm run tauri dev` works too.

## Build the packages

```bash
npm run tauri build
```

Output:

- `src-tauri/target/release/bundle/deb/KuttyLM_<version>_amd64.deb`
- `src-tauri/target/release/bundle/appimage/KuttyLM_<version>_amd64.AppImage`

The first AppImage build downloads Tauri's packaging tools (about 20–30 MB).
The `.deb` depends on `libvulkan1`, `libgomp1` and `libssl3` (standard on Ubuntu).

## Tests

```bash
npm run check                         # TypeScript / Svelte type check
npm run test:ui                       # chat stream tests (Node's built-in test runner)
cd src-tauri && cargo test            # Rust unit tests (no internet needed)
```

Tests that need the internet, the engine or a downloaded model are marked `#[ignore]`:

```bash
cargo test engine::tests -- --ignored --test-threads 1   # real llama-server (needs SmolLM2 downloaded)
cargo test hf::tests -- --ignored --nocapture            # real Hugging Face search (a few KB)
cargo test real_download_smallest_model -- --ignored     # real 105 MB download
cargo test print_this_device -- --ignored --nocapture    # prints what your computer reports
```

Chat tests against a running engine:
`KUTTYLM_TEST_CHAT_URL=http://127.0.0.1:PORT/v1 KUTTYLM_TEST_CHAT_KEY=KEY npm run test:ui`

---

## Where files are stored

| What | Ubuntu | Windows | macOS |
|---|---|---|---|
| Models (one folder per model; can be changed in Settings) | `~/.local/share/kuttylm/models/` | `%LOCALAPPDATA%\kuttylm\models\` | `~/Library/Application Support/kuttylm/models/` |
| Settings | `~/.config/kuttylm/settings.json` | `%APPDATA%\kuttylm\settings.json` | `~/Library/Application Support/kuttylm/settings.json` |
| Offline copy of the model list | `~/.cache/kuttylm/models.json` | `%LOCALAPPDATA%\kuttylm\models.json` | `~/Library/Caches/kuttylm/models.json` |
| Saved chats (one file per chat) | `~/.local/share/kuttylm/chats/` | `%LOCALAPPDATA%\kuttylm\chats\` | `~/Library/Application Support/kuttylm/chats/` |

A model folder has the GGUF file(s), `download.json` while the download is not finished,
and `model.json` when it is complete and checked. Unfinished files end in `.part`.

## Using the API from other apps

Turn on **Settings → Allow other apps to use the model**, then run a model. Example:

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_KEY" \
  -d '{"model": "any", "messages": [{"role": "user", "content": "Hello"}]}'
```

Leave out the `Authorization` line if you did not set a key. With the API off, the engine
listens on a random port on `127.0.0.1` with a hidden key that only KuttyLM knows.

## The model list API

The "Picked for you" list is `catalog/models.json`. The app reads it from GitHub
(`MODELS_API_URL` in `src-tauri/src/config.rs`) and also has it built in as an offline fallback.
Change the list by editing that file; `scripts/check-catalog.py` (also run by
`.github/workflows/catalog.yml`) checks it against Hugging Face. Details and the format:
[model-list-api.md](model-list-api.md).

## Settings for developers (environment variables)

| Variable | Use |
|---|---|
| `KUTTYLM_MODELS_API_URL` | Use another model list API (for example a local test server) |
| `KUTTYLM_HF_BASE_URL` | Download from another server than `https://huggingface.co` (tests) |
| `KUTTYLM_MODELS_DIR` | Use another models folder (tests) |
| `KUTTYLM_DEV_AUTORUN=<model-id>` | Debug builds only: start this model when the app opens |

---

## Project layout

```
src/                     Svelte frontend
  screens/               Home, Browse (Find models), MyModels, Chat, Settings
  components/            small UI parts (cards, progress, dialogs, Hugging Face search…)
  lib/                   API calls to Rust, stores, chat client, formatters
src-tauri/src/
  device/, platform/     computer info and process control (linux.rs, windows.rs, macos.rs)
  catalog/               curated model list: fetch, cache, parse
  hf/                    Hugging Face search and file lists
  fit.rs                 the "fits my computer" formula (change it here)
  download/              downloads: resume, pause, cancel, sha256
  library.rs             downloaded models on disk, delete
  engine/                starts and stops llama-server
  settings.rs            user settings
scripts/                 dev.sh, fetch-llama-server.sh (Ubuntu), fetch-llama-server.ps1 (Windows),
                         fetch-llama-server-macos.sh (macOS), check-catalog.py (list check)
docs/                    developer guide, model list format, release notes, screenshots
catalog/models.json      the "Picked for you" list (read online by the app, and built in)
.github/workflows/       release.yml (tagged releases), build.yml (test builds), catalog.yml (list check)
```

## Troubleshooting

- **`linking with cc failed … unknown option '-m64'`** — another program called `cc` comes
  first in your `PATH`. `src-tauri/.cargo/config.toml` already points Cargo to `/usr/bin/cc`.
- **The model is slow on a laptop** — built-in graphics (for example Intel UHD) are often slower
  than the processor, so "Automatic" uses the processor for them. You can change this in Settings.
- **"Port 8080 is used by another program"** — choose another port in Settings.
- **Building the Windows installer on Linux** — install `nsis` and `lld-14` (apt), then
  `cargo install --locked cargo-xwin`, then:

  ```bash
  bash scripts/fetch-windows-engine.sh      # Windows engine + VC++ runtime installer (about 58 MB)
  PATH=/usr/lib/llvm-14/bin:$PATH npm run tauri build -- --runner cargo-xwin \
    --target x86_64-pc-windows-msvc --bundles nsis --config src-tauri/tauri.windows.conf.json
  ```

  The first run downloads Microsoft's Windows SDK and C runtime files (about 1.1 GB on disk, in
  `~/.cache/cargo-xwin`). The installer is written to
  `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`.
- **Checking the Windows code from Linux** — `rustup target add x86_64-pc-windows-msvc`, then
  `PATH=/usr/lib/llvm-14/bin:$PATH cargo check --target x86_64-pc-windows-msvc` in `src-tauri/`
  (needs `llvm-rc` from the `llvm` package). This compiles everything but does not make an installer.
- **Disk full** — delete models in **My models**, or run `cargo clean` in `src-tauri/`.
