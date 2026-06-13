import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";
import { CompressionPanel } from "./components/CompressionPanel";
import { ConvertPanel } from "./components/ConvertPanel";
import { FileInfoCard } from "./components/FileInfoCard";
import { PhotoCompressionPanel } from "./components/PhotoCompressionPanel";
import { ProgressPanel } from "./components/ProgressPanel";
import { SettingsMenu } from "./components/SettingsMenu";
import { TopBar } from "./components/TopBar";
import { TrimPanel } from "./components/TrimPanel";
import { canTryPreview, VideoPlayer } from "./components/VideoPlayer";
import {
  cancelJob,
  compressPhoto,
  compressVideo,
  convertToMp4,
  detectEncoders,
  fastTrim,
  getLaunchVideoPath,
  openDefaultPlayerSettings,
  openOutputFolderDialog,
  openVideoDialog,
  preparePreview,
  preciseTrim,
  probeVideo,
  toAssetUrl,
} from "./lib/api";
import type {
  CompressionPreset,
  EncoderSupport,
  JobProgress,
  JobResult,
  PhotoCompressionFormat,
  PhotoCompressionPreset,
  VideoMetadata,
} from "./lib/types";

const SETTINGS_KEYS = {
  startInTheaterMode: "hitplayer.startInTheaterMode",
  defaultCompressionPreset: "hitplayer.defaultCompressionPreset",
  defaultPhotoCompressionFormat: "hitplayer.defaultPhotoCompressionFormat",
  defaultPhotoCompressionPreset: "hitplayer.defaultPhotoCompressionPreset",
  outputDirectory: "hitplayer.outputDirectory",
};

const COMPRESSION_PRESETS: CompressionPreset[] = ["balanced", "small", "high_quality", "nvidia_fast"];
const PHOTO_COMPRESSION_PRESETS: PhotoCompressionPreset[] = ["balanced", "small", "high_quality"];
const PHOTO_COMPRESSION_FORMATS: PhotoCompressionFormat[] = ["jpeg", "webp"];
type PreviewState = "idle" | "native" | "preparing" | "ready" | "failed";
type PreviewSource = "native" | "stream_copy" | "transcode" | null;

function storedBoolean(key: string, fallback: boolean): boolean {
  try {
    const value = window.localStorage.getItem(key);
    return value == null ? fallback : value === "true";
  } catch {
    return fallback;
  }
}

function storedPreset(): CompressionPreset {
  try {
    const preset = window.localStorage.getItem(SETTINGS_KEYS.defaultCompressionPreset);
    return COMPRESSION_PRESETS.includes(preset as CompressionPreset) ? (preset as CompressionPreset) : "balanced";
  } catch {
    return "balanced";
  }
}

function storedPhotoPreset(): PhotoCompressionPreset {
  try {
    const preset = window.localStorage.getItem(SETTINGS_KEYS.defaultPhotoCompressionPreset);
    return PHOTO_COMPRESSION_PRESETS.includes(preset as PhotoCompressionPreset)
      ? (preset as PhotoCompressionPreset)
      : "balanced";
  } catch {
    return "balanced";
  }
}

function storedPhotoFormat(): PhotoCompressionFormat {
  try {
    const format = window.localStorage.getItem(SETTINGS_KEYS.defaultPhotoCompressionFormat);
    return PHOTO_COMPRESSION_FORMATS.includes(format as PhotoCompressionFormat)
      ? (format as PhotoCompressionFormat)
      : "jpeg";
  } catch {
    return "jpeg";
  }
}

function storedString(key: string): string | null {
  try {
    const value = window.localStorage.getItem(key)?.trim();
    return value ? value : null;
  } catch {
    return null;
  }
}

function trimValidation(
  hasMedia: boolean,
  startSeconds: number,
  endSeconds: number,
  durationSeconds: number | null,
): string | null {
  if (!hasMedia) {
    return "Select a media file first.";
  }
  if (!Number.isFinite(startSeconds) || !Number.isFinite(endSeconds)) {
    return "Invalid trim range.";
  }
  if (startSeconds < 0) {
    return "Start must be at least 0.";
  }
  if (endSeconds <= startSeconds) {
    return "End must be greater than start.";
  }
  if (durationSeconds != null && endSeconds > durationSeconds + 0.001) {
    return "End must be within the media duration.";
  }
  if (endSeconds - startSeconds <= 0.1) {
    return "Selected duration must be greater than 0.1 seconds.";
  }
  return null;
}

export default function App() {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [metadata, setMetadata] = useState<VideoMetadata | null>(null);
  const [encoders, setEncoders] = useState<EncoderSupport | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [previewState, setPreviewState] = useState<PreviewState>("idle");
  const [previewMessage, setPreviewMessage] = useState<string | null>(null);
  const [previewSource, setPreviewSource] = useState<PreviewSource>(null);
  const [previewFailed, setPreviewFailed] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [trimStart, setTrimStart] = useState(0);
  const [trimEnd, setTrimEnd] = useState(0);
  const [preset, setPreset] = useState<CompressionPreset>(() => storedPreset());
  const [photoPreset, setPhotoPreset] = useState<PhotoCompressionPreset>(() => storedPhotoPreset());
  const [photoFormat, setPhotoFormat] = useState<PhotoCompressionFormat>(() => storedPhotoFormat());
  const [jobName, setJobName] = useState("");
  const [progress, setProgress] = useState<JobProgress | null>(null);
  const [result, setResult] = useState<JobResult | null>(null);
  const [detailsLog, setDetailsLog] = useState("");
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [theaterMode, setTheaterMode] = useState(() => storedBoolean(SETTINGS_KEYS.startInTheaterMode, false));
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [startInTheaterMode, setStartInTheaterMode] = useState(() =>
    storedBoolean(SETTINGS_KEYS.startInTheaterMode, false),
  );
  const [outputDirectory, setOutputDirectory] = useState<string | null>(() =>
    storedString(SETTINGS_KEYS.outputDirectory),
  );
  const [defaultPlayerStatus, setDefaultPlayerStatus] = useState<string | null>(null);
  const loadedLaunchPath = useRef(false);
  const loadRequestId = useRef(0);

  const durationSeconds = metadata?.durationSeconds ?? null;
  const hasMedia = !!selectedPath && !!metadata;
  const hasImage = hasMedia && metadata?.mediaKind === "image";
  const hasVideoStream = hasMedia && metadata?.mediaKind === "video" && !!metadata.videoCodec;
  const hasAudioOnly = hasMedia && metadata?.mediaKind === "audio" && !!metadata.audioCodec;
  const canTrimMedia = hasVideoStream || hasAudioOnly;
  const validationMessage = useMemo(
    () => trimValidation(canTrimMedia, trimStart, trimEnd, durationSeconds),
    [canTrimMedia, trimStart, trimEnd, durationSeconds],
  );

  useEffect(() => {
    detectEncoders()
      .then(setEncoders)
      .catch((err) => {
        setEncoders(null);
        setError(String(err));
      });

    let cleanup: (() => void) | undefined;
    listen<JobProgress>("ffmpeg-progress", (event) => {
      setProgress(event.payload);
      if (["finished", "failed", "canceled"].includes(event.payload.phase)) {
        setIsBusy(false);
      }
    }).then((unlisten) => {
      cleanup = unlisten;
    });

    return () => cleanup?.();
  }, []);

  useEffect(() => {
    if (loadedLaunchPath.current) {
      return;
    }

    loadedLaunchPath.current = true;
    getLaunchVideoPath()
      .then((path) => {
        if (path) {
          loadVideo(path).catch((err) => setError(String(err)));
        }
      })
      .catch(() => {
        // Plain browser previews do not have Tauri IPC. The packaged app does.
      });
  }, []);

  async function preparePreviewForPath(path: string, requestId: number, forceTranscode: boolean) {
    setPreviewFailed(false);
    setPreviewUrl(null);
    setPreviewState("preparing");
    setPreviewSource(null);
    setPreviewMessage(
      forceTranscode
        ? "HitPlayer is building a fully compatible local MP4 preview from the original file."
        : "HitPlayer is preparing a local MP4 preview so this file can play here.",
    );
    setIsBusy(true);
    setJobName("Preparing Preview");
    setProgress(null);
    setResult(null);
    setDetailsLog("");

    try {
      const preview = await preparePreview(path, forceTranscode);
      if (requestId !== loadRequestId.current) {
        return;
      }

      setPreviewUrl(toAssetUrl(preview.previewPath));
      setPreviewState("ready");
      setPreviewSource(preview.method === "stream_copy" ? "stream_copy" : "transcode");
      setPreviewMessage(
        preview.usedCachedFile
          ? "Using the cached local preview. Exports still use the original file."
          : "Preview ready. Exports still use the original file.",
      );
      setDetailsLog(preview.log);
    } catch (err) {
      if (requestId !== loadRequestId.current) {
        return;
      }

      setPreviewFailed(true);
      setPreviewUrl(null);
      setPreviewState("failed");
      setPreviewSource(null);
      setPreviewMessage("This file cannot be previewed yet, but FFmpeg may still process it.");
      setError(String(err));
    } finally {
      if (requestId === loadRequestId.current) {
        setIsBusy(false);
      }
    }
  }

  async function loadVideo(path: string) {
    const requestId = loadRequestId.current + 1;
    loadRequestId.current = requestId;

    setSelectedPath(path);
    setMetadata(null);
    setPreviewUrl(null);
    setPreviewState("idle");
    setPreviewSource(null);
    setPreviewMessage(null);
    setPreviewFailed(false);
    setCurrentTime(0);
    setTrimStart(0);
    setTrimEnd(0);
    setProgress(null);
    setResult(null);
    setDetailsLog("");

    try {
      const info = await probeVideo(path);
      if (requestId !== loadRequestId.current) {
        return;
      }

      setMetadata(info);
      setTrimEnd(info.durationSeconds ?? 0);

      if (info.mediaKind === "image") {
        setPreviewState("native");
        setPreviewSource("native");
        setPreviewMessage(null);
        setPreviewUrl(toAssetUrl(path));
      } else if (canTryPreview(path)) {
        setPreviewState("native");
        setPreviewSource("native");
        setPreviewMessage(null);
        setPreviewUrl(toAssetUrl(path));
      } else {
        await preparePreviewForPath(path, requestId, false);
      }
    } catch (err) {
      if (requestId === loadRequestId.current) {
        setMetadata(null);
        setPreviewFailed(true);
        setPreviewUrl(null);
        setPreviewState("failed");
        setPreviewSource(null);
        setPreviewMessage("Could not read this media file. Check that the file or network drive is still available.");
      }
      throw err;
    }
  }

  async function handleOpenVideo() {
    try {
      setError(null);
      const path = await openVideoDialog();
      if (!path) {
        return;
      }

      await loadVideo(path);
    } catch (err) {
      setError(String(err));
    }
  }

  async function runJob(name: string, action: () => Promise<JobResult>) {
    try {
      setError(null);
      setIsBusy(true);
      setJobName(name);
      setProgress(null);
      setResult(null);
      setDetailsLog("");

      const jobResult = await action();
      setResult(jobResult);
      setDetailsLog(jobResult.log);
      if (!jobResult.success && jobResult.error) {
        setError(jobResult.error);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setIsBusy(false);
    }
  }

  function selectedMediaPath(): string {
    if (!selectedPath) {
      throw new Error("Select a media file first.");
    }
    return selectedPath;
  }

  function selectedOutputDirectory(): string | undefined {
    return outputDirectory?.trim() || undefined;
  }

  function handleFastTrim() {
    void runJob("Fast Trim", () =>
      fastTrim({
        inputPath: selectedMediaPath(),
        startSeconds: trimStart,
        endSeconds: trimEnd,
        outputDirectory: selectedOutputDirectory(),
      }),
    );
  }

  function handlePreciseTrim() {
    void runJob("Precise Trim", () =>
      preciseTrim({
        inputPath: selectedMediaPath(),
        startSeconds: trimStart,
        endSeconds: trimEnd,
        outputDirectory: selectedOutputDirectory(),
      }),
    );
  }

  function handleCompress() {
    void runJob("Compress Video", () =>
      compressVideo(selectedMediaPath(), preset, undefined, selectedOutputDirectory()),
    );
  }

  function handleCompressPhoto() {
    void runJob("Compress Photo", () =>
      compressPhoto({
        inputPath: selectedMediaPath(),
        preset: photoPreset,
        format: photoFormat,
        outputDirectory: selectedOutputDirectory(),
      }),
    );
  }

  function handleConvert() {
    void runJob("Convert to Compatible MP4", () =>
      convertToMp4(selectedMediaPath(), undefined, selectedOutputDirectory()),
    );
  }

  async function handleCancel() {
    try {
      await cancelJob();
      setProgress((current) =>
        current
          ? { ...current, phase: "canceled", message: "Canceled." }
          : { jobId: "", phase: "canceled", percent: 0, message: "Canceled." },
      );
      setIsBusy(false);
    } catch (err) {
      setError(String(err));
    }
  }

  function handlePreviewFailed() {
    setPreviewFailed(true);

    if (hasImage) {
      setPreviewState("failed");
      setPreviewMessage("This image cannot be previewed here, but FFmpeg may still compress it.");
      return;
    }

    if (selectedPath && previewSource !== "transcode") {
      void preparePreviewForPath(selectedPath, loadRequestId.current, true);
      return;
    }

    setPreviewState("failed");
    setPreviewMessage("This file cannot be previewed yet, but FFmpeg may still process it.");
  }

  async function handleSetDefaultPlayer() {
    try {
      setError(null);
      setDefaultPlayerStatus("Opening Windows Default Apps...");
      await openDefaultPlayerSettings();
      setDefaultPlayerStatus("Pick HitPlayer for the media extensions Windows shows.");
    } catch (err) {
      setDefaultPlayerStatus(null);
      setError(String(err));
    }
  }

  function handlePresetChange(nextPreset: CompressionPreset) {
    setPreset(nextPreset);
    try {
      window.localStorage.setItem(SETTINGS_KEYS.defaultCompressionPreset, nextPreset);
    } catch {
      // Ignore storage failures. The visible setting still works for this session.
    }
  }

  function handlePhotoPresetChange(nextPreset: PhotoCompressionPreset) {
    setPhotoPreset(nextPreset);
    try {
      window.localStorage.setItem(SETTINGS_KEYS.defaultPhotoCompressionPreset, nextPreset);
    } catch {
      // Ignore storage failures. The visible setting still works for this session.
    }
  }

  function handlePhotoFormatChange(nextFormat: PhotoCompressionFormat) {
    setPhotoFormat(nextFormat);
    try {
      window.localStorage.setItem(SETTINGS_KEYS.defaultPhotoCompressionFormat, nextFormat);
    } catch {
      // Ignore storage failures. The visible setting still works for this session.
    }
  }

  function handleStartInTheaterModeChange(enabled: boolean) {
    setStartInTheaterMode(enabled);
    if (enabled) {
      setTheaterMode(true);
    }
    try {
      window.localStorage.setItem(SETTINGS_KEYS.startInTheaterMode, String(enabled));
    } catch {
      // Ignore storage failures. The visible setting still works for this session.
    }
  }

  async function handleChooseOutputDirectory() {
    try {
      setError(null);
      const directory = await openOutputFolderDialog(outputDirectory);
      if (!directory) {
        return;
      }

      setOutputDirectory(directory);
      try {
        window.localStorage.setItem(SETTINGS_KEYS.outputDirectory, directory);
      } catch {
        // Ignore storage failures. The visible setting still works for this session.
      }
    } catch (err) {
      setError(String(err));
    }
  }

  function handleClearOutputDirectory() {
    setOutputDirectory(null);
    try {
      window.localStorage.removeItem(SETTINGS_KEYS.outputDirectory);
    } catch {
      // Ignore storage failures. The visible setting still works for this session.
    }
  }

  function handleResetSettings() {
    handlePresetChange("balanced");
    handlePhotoPresetChange("balanced");
    handlePhotoFormatChange("jpeg");
    setStartInTheaterMode(false);
    setTheaterMode(false);
    setOutputDirectory(null);
    try {
      window.localStorage.removeItem(SETTINGS_KEYS.startInTheaterMode);
      window.localStorage.removeItem(SETTINGS_KEYS.defaultCompressionPreset);
      window.localStorage.removeItem(SETTINGS_KEYS.defaultPhotoCompressionPreset);
      window.localStorage.removeItem(SETTINGS_KEYS.defaultPhotoCompressionFormat);
      window.localStorage.removeItem(SETTINGS_KEYS.outputDirectory);
    } catch {
      // Ignore storage failures.
    }
  }

  return (
    <div className="flex h-screen min-h-0 flex-col overflow-hidden bg-ink-950 text-slate-100">
      <TopBar
        onOpenVideo={handleOpenVideo}
        onOpenSettings={() => setSettingsOpen(true)}
        isBusy={isBusy}
        theaterMode={theaterMode}
        onToggleTheaterMode={() => setTheaterMode((enabled) => !enabled)}
      />

      <main
        className={`grid min-h-0 flex-1 gap-5 overflow-hidden p-5 pb-3 ${
          theaterMode ? "grid-cols-1" : "grid-cols-[minmax(0,1fr)_390px]"
        }`}
      >
        <VideoPlayer
          filePath={selectedPath}
          previewUrl={previewUrl}
          previewState={previewState}
          previewMessage={previewMessage}
          durationSeconds={durationSeconds}
          previewFailed={previewFailed}
          theaterMode={theaterMode}
          isAudioOnly={hasAudioOnly}
          isImage={hasImage}
          width={metadata?.width ?? null}
          height={metadata?.height ?? null}
          onPreviewFailed={handlePreviewFailed}
          onTimeUpdate={setCurrentTime}
        />

        <aside className={theaterMode ? "hidden" : "min-h-0 space-y-4 overflow-y-auto pr-1"}>
          {error ? (
            <div className="rounded-lg border border-amber-400/30 bg-amber-500/10 p-3 text-sm text-amber-100">
              {error}
            </div>
          ) : null}

          <FileInfoCard filePath={selectedPath} metadata={metadata} />

          {hasImage ? (
            <PhotoCompressionPanel
              selectedPreset={photoPreset}
              selectedFormat={photoFormat}
              hasImage={hasImage}
              mediaSelected={hasMedia}
              isBusy={isBusy}
              hasWebp={!!encoders?.hasLibwebp}
              onPresetChange={handlePhotoPresetChange}
              onFormatChange={handlePhotoFormatChange}
              onCompress={handleCompressPhoto}
            />
          ) : (
            <>
              <TrimPanel
                hasMedia={canTrimMedia}
                isBusy={isBusy}
                currentTime={currentTime}
                durationSeconds={durationSeconds}
                startSeconds={trimStart}
                endSeconds={trimEnd}
                validationMessage={validationMessage}
                onSetStart={() => setTrimStart(Math.max(0, currentTime))}
                onSetEnd={() => setTrimEnd(Math.max(0, currentTime))}
                onStartChange={setTrimStart}
                onEndChange={setTrimEnd}
                onFastTrim={handleFastTrim}
                onPreciseTrim={handlePreciseTrim}
              />

              <CompressionPanel
                selectedPreset={preset}
                encoders={encoders}
                hasVideo={hasVideoStream}
                mediaSelected={hasMedia}
                isBusy={isBusy}
                onPresetChange={handlePresetChange}
                onCompress={handleCompress}
              />

              <ConvertPanel
                hasVideo={hasVideoStream}
                mediaSelected={hasMedia}
                isBusy={isBusy}
                onConvert={handleConvert}
              />
            </>
          )}
        </aside>
      </main>

      <div
        className={
          theaterMode && !isBusy && !progress && !result
            ? "hidden"
            : "shrink-0 border-t border-white/10 p-4"
        }
      >
        <ProgressPanel
          jobName={jobName}
          progress={progress}
          result={result}
          isBusy={isBusy}
          detailsLog={detailsLog}
          onCancel={handleCancel}
          onError={setError}
        />
      </div>

      <SettingsMenu
        open={settingsOpen}
        startInTheaterMode={startInTheaterMode}
        defaultPreset={preset}
        defaultPhotoPreset={photoPreset}
        defaultPhotoFormat={photoFormat}
        outputDirectory={outputDirectory}
        defaultPlayerStatus={defaultPlayerStatus}
        isBusy={isBusy}
        onClose={() => setSettingsOpen(false)}
        onStartInTheaterModeChange={handleStartInTheaterModeChange}
        onDefaultPresetChange={handlePresetChange}
        onDefaultPhotoPresetChange={handlePhotoPresetChange}
        onDefaultPhotoFormatChange={handlePhotoFormatChange}
        onChooseOutputDirectory={handleChooseOutputDirectory}
        onClearOutputDirectory={handleClearOutputDirectory}
        onOpenDefaultPlayerSettings={handleSetDefaultPlayer}
        onResetSettings={handleResetSettings}
      />
    </div>
  );
}
