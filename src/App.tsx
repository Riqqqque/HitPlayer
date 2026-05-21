import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";
import { CompressionPanel } from "./components/CompressionPanel";
import { ConvertPanel } from "./components/ConvertPanel";
import { DefaultPlayerPanel } from "./components/DefaultPlayerPanel";
import { FileInfoCard } from "./components/FileInfoCard";
import { ProgressPanel } from "./components/ProgressPanel";
import { SettingsMenu } from "./components/SettingsMenu";
import { TopBar } from "./components/TopBar";
import { TrimPanel } from "./components/TrimPanel";
import { canTryPreview, VideoPlayer } from "./components/VideoPlayer";
import {
  cancelJob,
  compressVideo,
  convertToMp4,
  detectEncoders,
  fastTrim,
  getLaunchVideoPath,
  openDefaultPlayerSettings,
  openVideoDialog,
  preciseTrim,
  probeVideo,
  toAssetUrl,
} from "./lib/api";
import type { CompressionPreset, EncoderSupport, JobProgress, JobResult, VideoMetadata } from "./lib/types";

const SETTINGS_KEYS = {
  startInTheaterMode: "hitplayer.startInTheaterMode",
  defaultCompressionPreset: "hitplayer.defaultCompressionPreset",
};

const COMPRESSION_PRESETS: CompressionPreset[] = ["balanced", "small", "high_quality", "nvidia_fast"];

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

function trimValidation(
  hasVideo: boolean,
  startSeconds: number,
  endSeconds: number,
  durationSeconds: number | null,
): string | null {
  if (!hasVideo) {
    return "Select a video first.";
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
    return "End must be within the video duration.";
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
  const [previewFailed, setPreviewFailed] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [trimStart, setTrimStart] = useState(0);
  const [trimEnd, setTrimEnd] = useState(0);
  const [preset, setPreset] = useState<CompressionPreset>(() => storedPreset());
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
  const [defaultPlayerStatus, setDefaultPlayerStatus] = useState<string | null>(null);
  const loadedLaunchPath = useRef(false);

  const durationSeconds = metadata?.durationSeconds ?? null;
  const hasVideo = !!selectedPath && !!metadata;
  const validationMessage = useMemo(
    () => trimValidation(hasVideo, trimStart, trimEnd, durationSeconds),
    [hasVideo, trimStart, trimEnd, durationSeconds],
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
          void loadVideo(path);
        }
      })
      .catch(() => {
        // Plain browser previews do not have Tauri IPC. The packaged app does.
      });
  }, []);

  async function loadVideo(path: string) {
    setSelectedPath(path);
    setMetadata(null);
    setPreviewFailed(false);
    setCurrentTime(0);
    setTrimStart(0);
    setTrimEnd(0);
    setProgress(null);
    setResult(null);
    setDetailsLog("");

    const info = await probeVideo(path);
    setMetadata(info);
    setTrimEnd(info.durationSeconds ?? 0);
    setPreviewUrl(canTryPreview(path) ? toAssetUrl(path) : null);
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

  function selectedVideoPath(): string {
    if (!selectedPath) {
      throw new Error("Select a video first.");
    }
    return selectedPath;
  }

  function handleFastTrim() {
    void runJob("Fast Trim", () =>
      fastTrim({
        inputPath: selectedVideoPath(),
        startSeconds: trimStart,
        endSeconds: trimEnd,
      }),
    );
  }

  function handlePreciseTrim() {
    void runJob("Precise Trim", () =>
      preciseTrim({
        inputPath: selectedVideoPath(),
        startSeconds: trimStart,
        endSeconds: trimEnd,
      }),
    );
  }

  function handleCompress() {
    void runJob("Compress Video", () => compressVideo(selectedVideoPath(), preset));
  }

  function handleConvert() {
    void runJob("Convert to Compatible MP4", () => convertToMp4(selectedVideoPath()));
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

  async function handleSetDefaultPlayer() {
    try {
      setError(null);
      setDefaultPlayerStatus("Opening Windows Default Apps...");
      await openDefaultPlayerSettings();
      setDefaultPlayerStatus("Pick HitPlayer for the video extensions Windows shows.");
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

  function handleResetSettings() {
    handlePresetChange("balanced");
    setStartInTheaterMode(false);
    setTheaterMode(false);
    try {
      window.localStorage.removeItem(SETTINGS_KEYS.startInTheaterMode);
      window.localStorage.removeItem(SETTINGS_KEYS.defaultCompressionPreset);
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
          durationSeconds={durationSeconds}
          previewFailed={previewFailed}
          theaterMode={theaterMode}
          width={metadata?.width ?? null}
          height={metadata?.height ?? null}
          onPreviewFailed={() => setPreviewFailed(true)}
          onTimeUpdate={setCurrentTime}
        />

        <aside className={theaterMode ? "hidden" : "min-h-0 space-y-4 overflow-y-auto pr-1"}>
          {error ? (
            <div className="rounded-lg border border-amber-400/30 bg-amber-500/10 p-3 text-sm text-amber-100">
              {error}
            </div>
          ) : null}

          <FileInfoCard filePath={selectedPath} metadata={metadata} />

          <TrimPanel
            hasVideo={hasVideo}
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
            hasVideo={hasVideo}
            isBusy={isBusy}
            onPresetChange={handlePresetChange}
            onCompress={handleCompress}
          />

          <ConvertPanel hasVideo={hasVideo} isBusy={isBusy} onConvert={handleConvert} />

          <DefaultPlayerPanel
            isBusy={isBusy}
            status={defaultPlayerStatus}
            onSetDefaultPlayer={handleSetDefaultPlayer}
          />
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
        defaultPlayerStatus={defaultPlayerStatus}
        isBusy={isBusy}
        onClose={() => setSettingsOpen(false)}
        onStartInTheaterModeChange={handleStartInTheaterModeChange}
        onDefaultPresetChange={handlePresetChange}
        onOpenDefaultPlayerSettings={handleSetDefaultPlayer}
        onResetSettings={handleResetSettings}
      />
    </div>
  );
}
