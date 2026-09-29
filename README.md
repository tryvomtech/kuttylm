<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="110" alt="KuttyLM icon">
</p>

<h1 align="center">KuttyLM</h1>

<p align="center">
  <b>Chat with AI on your own computer.</b><br>
  Private, free and simple — no commands, no account, no subscription.
</p>

<p align="center">
  <a href="https://github.com/tryvomtech/kuttylm/releases/latest"><img src="https://img.shields.io/github/v/release/tryvomtech/kuttylm?label=download&color=2563eb" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-lightgrey" alt="Windows, macOS and Linux">
  <a href="LICENSE"><img src="https://img.shields.io/badge/licence-MIT-green" alt="MIT licence"></a>
</p>

<p align="center">
  <a href="#download"><b>⬇ Download KuttyLM</b></a>
</p>

<p align="center">
  <img src="docs/images/find-models.png" width="860" alt="KuttyLM showing AI models and whether each one fits the computer">
</p>

## Why KuttyLM?

There are many tools for running AI on your own computer, but most of them need a terminal,
are large and complicated, or use technical words. KuttyLM is built for everyone:

- **Lightweight** — a small download that starts fast and leaves your memory for the AI.
- **No commands** — everything is buttons and screens.
- **Made for non-technical people** — it checks your computer and tells you which AI models will run well.

## Download

Click the link for your computer. It always downloads the newest version.

| Your computer | Download |
|---|---|
| **Windows** 10 or 11 | [**KuttyLM-Windows-Setup.exe**](https://github.com/tryvomtech/kuttylm/releases/latest/download/KuttyLM-Windows-Setup.exe) |
| **Mac** with an Apple chip (M1 or newer) | [**KuttyLM-Mac.dmg**](https://github.com/tryvomtech/kuttylm/releases/latest/download/KuttyLM-Mac.dmg) |
| **Ubuntu** 22.04 or 24.04 | [**KuttyLM-Linux.deb**](https://github.com/tryvomtech/kuttylm/releases/latest/download/KuttyLM-Linux.deb) |
| **Other Linux** | [**KuttyLM-Linux.AppImage**](https://github.com/tryvomtech/kuttylm/releases/latest/download/KuttyLM-Linux.AppImage) |

All versions and release notes: [Releases](https://github.com/tryvomtech/kuttylm/releases).

<details>
<summary><b>First start: "Windows protected your PC" or "Apple could not check it"?</b></summary>

KuttyLM is new and not yet signed by Microsoft or Apple, so your computer asks once if you trust it.

- **Windows:** click **More info → Run anyway**, then **Install**. If Windows asks to install
  "Microsoft Visual C++", click **Yes** (KuttyLM needs it to run AI models).
- **Mac:** drag KuttyLM into **Applications** and open it once. Then open
  **System Settings → Privacy & Security** and click **Open Anyway**.
- **Ubuntu:** double-click the `.deb` file and click **Install**.
- **Other Linux:** allow the `.AppImage` to run as a program (right-click → Properties →
  Permissions), then double-click it.

</details>

## How to use it

1. Open **Find models** and download a model marked **Fits well**. Try **SmolLM2 135M** (100 MB) first.
2. Open **My models** and click **Run**.
3. Open **Chat** and ask anything.

<p align="center">
  <img src="docs/images/chat.png" width="860" alt="A chat with an AI model running on the computer">
</p>

## Features

- **Private** — models run on your computer. Your chats are never sent to the internet.
- **Chat history** — your chats are saved on your computer, sorted by date, with short titles written by the AI. Search, rename or delete them.
- **Nice answers** — tables, lists, headings and code blocks (with a Copy button) are shown neatly.
- **Fits my computer** — every model shows *Fits well*, *May be slow* or *Will not fit*.
- **Thousands of models** — a hand-picked list, plus search on Hugging Face.
- **Easy downloads** — pause, resume, and a check that the file is complete.
- **Uses your graphics card** when it helps (NVIDIA, AMD, Intel Arc, Apple chips).
- **For other apps** — optional OpenAI-compatible API, with a key and network access if you want.
- **Light and dark mode.**

| My computer | Settings |
|---|---|
| <img src="docs/images/home.png" alt="Memory, processor and graphics card of the computer"> | <img src="docs/images/settings.png" alt="Settings for the API, the graphics card and the models folder"> |

## What your computer needs

| | |
|---|---|
| Memory (RAM) | 8 GB or more is best. Small models work with 4 GB. |
| Disk space | 200 MB for KuttyLM, plus 0.1–5 GB for each model |
| Internet | Only to download KuttyLM and the models. Chatting works offline. |

## Help and ideas

Found a problem or have an idea? [Open an issue](https://github.com/tryvomtech/kuttylm/issues).

Developers: see the [developer guide](docs/development.md) to build KuttyLM from source.

## Licence

KuttyLM is free and open source under the [MIT licence](LICENSE).
It includes [llama.cpp](https://github.com/ggml-org/llama.cpp) (MIT licence).
The Windows installer includes Microsoft's Visual C++ runtime installer, as allowed by Microsoft.
Each AI model has its own licence, shown in the app.

<p align="center">Made by <a href="https://tryvom.com">Tryvom</a></p>
