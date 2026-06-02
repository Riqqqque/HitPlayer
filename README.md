# HitPlayer

<img src="src-tauri/icons/icon.png" alt="HitPlayer icon" width="96">

**Play it. Cut it. Compress it.**

HitPlayer is a dark, clean Windows media player and FFmpeg-powered video utility. It is built for normal desktop use first: open a video, preview it directly or through a local FFmpeg-prepared preview, trim it, shrink it, convert it, and get back to the output without fighting command lines.

[![Build](https://github.com/Riqqqque/HitPlayer/actions/workflows/build.yml/badge.svg)](https://github.com/Riqqqque/HitPlayer/actions/workflows/build.yml)

## Download

Grab the latest installer from the [Releases page](https://github.com/Riqqqque/HitPlayer/releases/latest).

The installer is current-user only, does not need admin rights, bundles FFmpeg/FFprobe locally, and launches as a normal Windows app without a command prompt in the background.

## What HitPlayer Does

- Opens common video files: MP4, MKV, MOV, AVI, WebM, M4V, FLV, WMV, TS, and M2TS.
- Opens common audio files: MP3, WAV, M4A, AAC, FLAC, OGG, OPUS, WMA, AIFF, and MKA.
- Previews MP4, MOV, WebM, and M4V through the built-in Windows WebView video engine.
- Prepares a local MP4 preview for MKV and other files WebView cannot play directly, including files opened from mapped or UNC network drives.
- Prepares a local M4A preview for audio formats the built-in player cannot open directly.
- Still processes the original file with FFmpeg even when a local preview copy is needed.
- Shows real metadata from FFprobe: duration, resolution, codecs, container, bitrate, and file size.
- Sets start/end trim points from playback or manual timestamps.
- Fast trims with stream copy, keeping quality and avoiding re-encoding.
- Precise trims with H.264/AAC when frame-accurate cuts matter.
- Fast trims audio with stream copy and precise trims audio to compatible M4A/AAC.
- Compresses with simple presets that target practical file sizes instead of bloating exports.
- Converts odd files into compatible MP4 for easier sharing and playback.
- Lets you choose a custom output folder from Settings.
- Shows FFmpeg progress, speed, percent, logs, cancel, and output actions.
- Registers itself as a Windows video app so you can pick it as the default player.

## Built For Windows

HitPlayer targets Windows 10/11 x64.

It does not require:

- admin rights
- internet access after install
- an account
- telemetry
- FFmpeg installed globally
- PATH changes

## Quick Start From Source

Install the normal Tauri Windows prerequisites first:

- Node.js
- Rust
- Microsoft C++ Build Tools
- WebView2 Runtime

Then run:

```powershell
npm install
npm run setup:ffmpeg
npm run tauri:dev
```

## Build A Release Installer

```powershell
npm install
npm run setup:ffmpeg
npm run tauri:build
```

The installer is written to:

```text
src-tauri\target\release\bundle\nsis\HitPlayer_0.1.12_x64-setup.exe
```

GitHub keeps normal push checks quick. The main build workflow runs the frontend build and Rust tests only. Full Windows installer builds run from the separate **Installer** workflow when started manually or when a `v*` release tag is pushed.

## FFmpeg Sidecars

HitPlayer always uses bundled/local binaries. It does not call `ffmpeg` or `ffprobe` from `PATH`.

Development binaries belong here:

```text
src-tauri\binaries\ffmpeg.exe
src-tauri\binaries\ffprobe.exe
src-tauri\binaries\ffmpeg-x86_64-pc-windows-msvc.exe
src-tauri\binaries\ffprobe-x86_64-pc-windows-msvc.exe
```

Run this to download and place them:

```powershell
npm run setup:ffmpeg
```

If a binary is missing, HitPlayer shows a clean app error instead of failing silently:

```text
FFmpeg was not found. Place ffmpeg.exe in the required app binary folder.
```

```text
FFprobe was not found. Place ffprobe.exe in the required app binary folder.
```

## Output Files

By default, HitPlayer creates a `HitPlayerExports` folder beside the source file.

Example:

```text
C:\Videos\clip.mp4
C:\Videos\HitPlayerExports\clip_trim_fast.mp4
C:\Videos\HitPlayerExports\clip_trim_precise.mp4
C:\Videos\HitPlayerExports\clip_compressed_balanced.mp4
C:\Videos\HitPlayerExports\clip_compressed_small.mp4
C:\Videos\HitPlayerExports\clip_compressed_high_quality.mp4
C:\Videos\HitPlayerExports\clip_compressed_nvidia_fast.mp4
C:\Videos\HitPlayerExports\clip_converted.mp4
```

Existing outputs are never overwritten. HitPlayer adds `_001`, `_002`, and so on.

Fast Trim keeps the source container when that is safer. For example, fast trimming `clip.mkv` writes `clip_trim_fast.mkv`. Re-encoded outputs use MP4.

For audio-only files, Fast Trim keeps the source audio extension when possible. Precise Trim writes a compatible M4A file, for example `song_trim_precise.m4a`.

You can also set a custom output folder in **Settings > Output folder**. When that is set, every trim, compression, and conversion export uses that folder while keeping the same clear filenames.

## Compression Targets

The Balanced preset is tuned for the everyday case that started this app: a two-minute clip around 80 MB should usually land under 50 MB while still looking decent.

Preset behavior:

- **Balanced**: practical size target for normal sharing.
- **Small File**: stronger compression when size matters most.
- **High Quality**: cleaner output with a capped bitrate.
- **NVIDIA Fast**: hardware encode path when NVENC is available.

HitPlayer detects NVENC before enabling the NVIDIA preset.

## Gaming And Streaming Performance

HitPlayer is tuned to stay out of the way while a game or stream is running:

- FFmpeg jobs run at below-normal Windows process priority and never open a console window.
- CPU H.264 jobs use capped x264 worker threads so Windows has headroom for games, OBS, Discord, and browser tabs.
- FFmpeg progress updates are limited to about once per second to avoid UI churn.
- MKV and other non-browser previews try a cheap stream-copy MP4 preview first when the video/audio codecs are already compatible.
- Full preview transcodes are capped to 1080p because the preview only needs to be playable, not archival quality.

NVIDIA Fast uses NVENC. It is quick, but it can still compete with OBS if OBS is also using NVENC, so Balanced or Small is the safer choice while actively streaming.

## Default Player

The installer registers HitPlayer as a video file handler. Windows still requires the user to confirm default apps.

Inside HitPlayer, open **Settings > Windows defaults > Open Default Apps**. It registers HitPlayer for the current Windows user and opens Windows Default Apps so you can choose it for the media extensions you want.

## Updating

Future installers keep the same app identity:

```text
productName: HitPlayer
identifier: com.rique.hitplayer
```

Bump the version with:

```powershell
npm run version:bump -- 0.1.2
```

Then build:

```powershell
npm run tauri:build
```

Running a newer setup EXE updates the existing current-user install. The installer asks to close HitPlayer if it is running, clears stale app binaries/resources from the install folder, refreshes Windows shell icons, and then installs the new files. It does not touch exported videos or user media.

For GitHub release builds, run the **Installer** workflow or push a version tag. This avoids rebuilding the full installer after every regular code change.

## Known Limitations

- V1 preview uses the Windows WebView/browser video engine first, so some containers need a local MP4 preview cache before they play.
- Audio preview uses the same built-in browser media engine first, then falls back to a local M4A preview for formats it cannot open directly.
- FFmpeg processing supports many more formats than direct browser preview.
- Fast Trim is not always frame-perfect because it stream-copies.
- Fast Trim only shrinks by cutting duration because it does not re-encode.
- Precise Trim is slower because it re-encodes.
- Compression and compatible MP4 conversion are video-only in this version.
- NVIDIA Fast only works when NVENC is available.
- libmpv or libVLC playback is planned later for near-VLC-level compatibility.

## Roadmap Notes

- TODO: add libmpv or libVLC playback.
- TODO: add batch compression.
- TODO: add AV1 encoding.
- TODO: add AMD and Intel GPU presets.
- TODO: add frame-step controls.
- TODO: add timeline thumbnails.
- TODO: add portable ZIP packaging.
- TODO: add Windows installer packaging polish.

## License

See [LICENSE](LICENSE).
