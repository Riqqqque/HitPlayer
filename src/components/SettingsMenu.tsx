import { FolderOpen, RotateCcw, Settings, Volume2, VolumeX, X } from "lucide-react";
import { useEffect, useRef } from "react";
import type {
  CompressionPreset,
  PhotoCompressionFormat,
  PhotoCompressionPreset,
  PlaybackAudioState,
} from "../lib/types";

type SettingsMenuProps = {
  open: boolean;
  startInTheaterMode: boolean;
  defaultPreset: CompressionPreset;
  defaultPhotoPreset: PhotoCompressionPreset;
  defaultPhotoFormat: PhotoCompressionFormat;
  hasWebp: boolean;
  hasNvenc: boolean;
  playbackAudio: PlaybackAudioState;
  outputDirectory: string | null;
  defaultPlayerStatus: string | null;
  isBusy: boolean;
  onClose: () => void;
  onStartInTheaterModeChange: (enabled: boolean) => void;
  onDefaultPresetChange: (preset: CompressionPreset) => void;
  onDefaultPhotoPresetChange: (preset: PhotoCompressionPreset) => void;
  onDefaultPhotoFormatChange: (format: PhotoCompressionFormat) => void;
  onPlaybackAudioChange: (audio: PlaybackAudioState) => void;
  onChooseOutputDirectory: () => void;
  onClearOutputDirectory: () => void;
  onOpenDefaultPlayerSettings: () => void;
  onResetSettings: () => void;
};

const PRESET_LABELS: Record<CompressionPreset, string> = {
  balanced: "Balanced",
  small: "Small File",
  high_quality: "High Quality",
  nvidia_fast: "NVIDIA Fast",
};

const PHOTO_PRESET_LABELS: Record<PhotoCompressionPreset, string> = {
  balanced: "Balanced",
  small: "Small File",
  high_quality: "High Quality",
};

export function SettingsMenu({
  open,
  startInTheaterMode,
  defaultPreset,
  defaultPhotoPreset,
  defaultPhotoFormat,
  hasWebp,
  hasNvenc,
  playbackAudio,
  outputDirectory,
  defaultPlayerStatus,
  isBusy,
  onClose,
  onStartInTheaterModeChange,
  onDefaultPresetChange,
  onDefaultPhotoPresetChange,
  onDefaultPhotoFormatChange,
  onPlaybackAudioChange,
  onChooseOutputDirectory,
  onClearOutputDirectory,
  onOpenDefaultPlayerSettings,
  onResetSettings,
}: SettingsMenuProps) {
  const dialogRef = useRef<HTMLElement | null>(null);
  const closeButtonRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    const previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const focusTimer = window.setTimeout(() => closeButtonRef.current?.focus(), 0);
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }

      if (event.key !== "Tab" || !dialogRef.current) {
        return;
      }

      const focusable = Array.from(
        dialogRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ),
      );
      if (focusable.length === 0) {
        event.preventDefault();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && (document.activeElement === first || !dialogRef.current.contains(document.activeElement))) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.clearTimeout(focusTimer);
      window.removeEventListener("keydown", handleKeyDown);
      previouslyFocused?.focus();
    };
  }, [onClose, open]);

  if (!open) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-5"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <section
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        className="max-h-[calc(100vh-40px)] w-full max-w-lg overflow-y-auto rounded-lg border border-white/12 bg-ink-900 p-5 shadow-panel"
      >
        <div className="mb-5 flex items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="grid h-10 w-10 place-items-center rounded-lg border border-white/10 bg-black/35 text-hit-300">
              <Settings size={19} />
            </div>
            <div>
              <h2 id="settings-title" className="text-lg font-semibold text-white">
                Settings
              </h2>
              <p className="text-sm text-slate-400">Small defaults that make HitPlayer feel right.</p>
            </div>
          </div>

          <button
            ref={closeButtonRef}
            type="button"
            className="secondary-button h-9 w-9 px-0"
            onClick={onClose}
            aria-label="Close settings"
          >
            <X size={17} />
          </button>
        </div>

        <div className="space-y-4">
          <label className="flex items-start justify-between gap-4 rounded-lg border border-white/10 bg-black/20 p-4">
            <span>
              <span className="block text-sm font-semibold text-slate-100">Start in theater mode</span>
              <span className="mt-1 block text-xs leading-5 text-slate-500">
                Opens the app with the larger preview layout.
              </span>
            </span>
            <input
              type="checkbox"
              checked={startInTheaterMode}
              onChange={(event) => onStartInTheaterModeChange(event.currentTarget.checked)}
              className="mt-1 h-5 w-5 accent-hit-400"
            />
          </label>

          <div className="rounded-lg border border-white/10 bg-black/20 p-4">
            <div className="flex items-center justify-between gap-3">
              <span className="text-sm font-semibold text-slate-100">Playback volume</span>
              <span className="text-xs tabular-nums text-slate-400">
                {playbackAudio.muted ? "Muted" : `${Math.round(playbackAudio.volume * 100)}%`}
              </span>
            </div>
            <div className="mt-3 flex items-center gap-3">
              <button
                type="button"
                className="secondary-button h-9 w-9 shrink-0 px-0"
                onClick={() => onPlaybackAudioChange({ ...playbackAudio, muted: !playbackAudio.muted })}
                aria-label={playbackAudio.muted ? "Unmute playback" : "Mute playback"}
                title={playbackAudio.muted ? "Unmute playback" : "Mute playback"}
              >
                {playbackAudio.muted ? <VolumeX size={17} /> : <Volume2 size={17} />}
              </button>
              <input
                type="range"
                min={0}
                max={1}
                step={0.01}
                value={playbackAudio.volume}
                onChange={(event) =>
                  onPlaybackAudioChange({
                    volume: Number(event.currentTarget.value),
                    muted: false,
                  })
                }
                className="h-2 w-full cursor-pointer accent-hit-400"
                aria-label="Playback volume"
              />
            </div>
          </div>

          <label className="block rounded-lg border border-white/10 bg-black/20 p-4">
            <span className="block text-sm font-semibold text-slate-100">Default compression preset</span>
            <select
              className="field mt-3"
              value={defaultPreset}
              onChange={(event) => onDefaultPresetChange(event.currentTarget.value as CompressionPreset)}
            >
              {(Object.keys(PRESET_LABELS) as CompressionPreset[]).map((preset) => (
                <option key={preset} value={preset} disabled={preset === "nvidia_fast" && !hasNvenc}>
                  {PRESET_LABELS[preset]}
                  {preset === "nvidia_fast" && !hasNvenc ? " unavailable" : ""}
                </option>
              ))}
            </select>
          </label>

          <div className="rounded-lg border border-white/10 bg-black/20 p-4">
            <span className="block text-sm font-semibold text-slate-100">Default photo compression</span>
            <div className="mt-3 grid grid-cols-2 gap-3">
              <label className="block text-xs text-slate-500">
                Preset
                <select
                  className="field mt-2"
                  value={defaultPhotoPreset}
                  onChange={(event) =>
                    onDefaultPhotoPresetChange(event.currentTarget.value as PhotoCompressionPreset)
                  }
                >
                  {(Object.keys(PHOTO_PRESET_LABELS) as PhotoCompressionPreset[]).map((preset) => (
                    <option key={preset} value={preset}>
                      {PHOTO_PRESET_LABELS[preset]}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block text-xs text-slate-500">
                Format
                <select
                  className="field mt-2"
                  value={defaultPhotoFormat}
                  onChange={(event) =>
                    onDefaultPhotoFormatChange(event.currentTarget.value as PhotoCompressionFormat)
                  }
                >
                  <option value="jpeg">JPEG</option>
                  <option value="webp" disabled={!hasWebp}>
                    WebP{hasWebp ? "" : " unavailable"}
                  </option>
                </select>
              </label>
            </div>
          </div>

          <div className="rounded-lg border border-white/10 bg-black/20 p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold text-slate-100">Output folder</h3>
                <p className="mt-1 text-xs leading-5 text-slate-500">
                  {outputDirectory ? "Exports use this folder." : "Exports go beside the source file."}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  className="secondary-button"
                  onClick={onChooseOutputDirectory}
                  disabled={isBusy}
                >
                  <FolderOpen size={15} />
                  Choose
                </button>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={onClearOutputDirectory}
                  disabled={isBusy || !outputDirectory}
                >
                  <X size={15} />
                  Clear
                </button>
              </div>
            </div>
            <div className="mt-3 rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-xs text-slate-300">
              <span className="block break-all">
                {outputDirectory ?? "HitPlayerExports folder next to each source file"}
              </span>
            </div>
          </div>

          <div className="rounded-lg border border-white/10 bg-black/20 p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold text-slate-100">Windows defaults</h3>
                <p className="mt-1 text-xs leading-5 text-slate-500">
                  Register HitPlayer and open Default Apps for this user.
                </p>
              </div>
              <button
                type="button"
                className="secondary-button"
                onClick={onOpenDefaultPlayerSettings}
                disabled={isBusy}
              >
                Open Default Apps
              </button>
            </div>
            {defaultPlayerStatus ? <p className="mt-3 text-xs text-slate-300">{defaultPlayerStatus}</p> : null}
          </div>
        </div>

        <div className="mt-5 flex justify-between gap-3 border-t border-white/10 pt-4">
          <button type="button" className="secondary-button" onClick={onResetSettings}>
            <RotateCcw size={15} />
            Reset
          </button>
          <button type="button" className="primary-button px-5" onClick={onClose}>
            Done
          </button>
        </div>
      </section>
    </div>
  );
}
