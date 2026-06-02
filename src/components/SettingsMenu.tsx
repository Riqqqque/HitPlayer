import { FolderOpen, RotateCcw, Settings, X } from "lucide-react";
import type { CompressionPreset } from "../lib/types";

type SettingsMenuProps = {
  open: boolean;
  startInTheaterMode: boolean;
  defaultPreset: CompressionPreset;
  outputDirectory: string | null;
  defaultPlayerStatus: string | null;
  isBusy: boolean;
  onClose: () => void;
  onStartInTheaterModeChange: (enabled: boolean) => void;
  onDefaultPresetChange: (preset: CompressionPreset) => void;
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

export function SettingsMenu({
  open,
  startInTheaterMode,
  defaultPreset,
  outputDirectory,
  defaultPlayerStatus,
  isBusy,
  onClose,
  onStartInTheaterModeChange,
  onDefaultPresetChange,
  onChooseOutputDirectory,
  onClearOutputDirectory,
  onOpenDefaultPlayerSettings,
  onResetSettings,
}: SettingsMenuProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-5 backdrop-blur-sm">
      <section className="max-h-[calc(100vh-40px)] w-full max-w-lg overflow-y-auto rounded-lg border border-white/12 bg-ink-900 p-5 shadow-panel">
        <div className="mb-5 flex items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="grid h-10 w-10 place-items-center rounded-lg border border-white/10 bg-black/35 text-hit-300">
              <Settings size={19} />
            </div>
            <div>
              <h2 className="text-lg font-semibold text-white">Settings</h2>
              <p className="text-sm text-slate-400">Small defaults that make HitPlayer feel right.</p>
            </div>
          </div>

          <button type="button" className="secondary-button h-9 w-9 px-0" onClick={onClose} aria-label="Close settings">
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

          <label className="block rounded-lg border border-white/10 bg-black/20 p-4">
            <span className="block text-sm font-semibold text-slate-100">Default compression preset</span>
            <select
              className="field mt-3"
              value={defaultPreset}
              onChange={(event) => onDefaultPresetChange(event.currentTarget.value as CompressionPreset)}
            >
              {(Object.keys(PRESET_LABELS) as CompressionPreset[]).map((preset) => (
                <option key={preset} value={preset}>
                  {PRESET_LABELS[preset]}
                </option>
              ))}
            </select>
          </label>

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
