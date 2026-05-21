# FFmpeg Sidecars

HitPlayer bundles local FFmpeg and FFprobe binaries when it is built, but the raw `.exe` files are not committed because they are too large for a clean source repo.

Run this from the project root before `npm run tauri:dev` or `npm run tauri:build`:

```powershell
npm run setup:ffmpeg
```

The script writes:

```text
src-tauri\binaries\ffmpeg.exe
src-tauri\binaries\ffprobe.exe
src-tauri\binaries\ffmpeg-x86_64-pc-windows-msvc.exe
src-tauri\binaries\ffprobe-x86_64-pc-windows-msvc.exe
```

The target-triple filenames are required by Tauri v2 sidecar bundling on Windows.
