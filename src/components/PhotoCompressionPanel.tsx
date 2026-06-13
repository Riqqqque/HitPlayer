import { ImageDown } from "lucide-react";
import type { PhotoCompressionFormat, PhotoCompressionPreset } from "../lib/types";

type PhotoCompressionPanelProps = {
  selectedPreset: PhotoCompressionPreset;
  selectedFormat: PhotoCompressionFormat;
  hasImage: boolean;
  mediaSelected: boolean;
  isBusy: boolean;
  hasWebp: boolean;
  onPresetChange: (preset: PhotoCompressionPreset) => void;
  onFormatChange: (format: PhotoCompressionFormat) => void;
  onCompress: () => void;
};

const PRESETS: Array<{ id: PhotoCompressionPreset; label: string; hint: string }> = [
  { id: "balanced", label: "Balanced", hint: "Good shrink, clean photo" },
  { id: "small", label: "Small File", hint: "Lower size, lighter detail" },
  { id: "high_quality", label: "High Quality", hint: "Best detail, less shrink" },
];

export function PhotoCompressionPanel({
  selectedPreset,
  selectedFormat,
  hasImage,
  mediaSelected,
  isBusy,
  hasWebp,
  onPresetChange,
  onFormatChange,
  onCompress,
}: PhotoCompressionPanelProps) {
  const canCompress = hasImage && !isBusy && (selectedFormat !== "webp" || hasWebp);

  return (
    <section className="rounded-lg border border-white/10 bg-ink-850 p-4 shadow-panel">
      <div className="mb-4 flex items-center gap-2">
        <ImageDown size={18} className="text-hit-300" />
        <h2 className="text-sm font-semibold uppercase text-slate-300">Photo Compression</h2>
      </div>

      <div className="grid grid-cols-1 gap-2">
        {PRESETS.map((preset) => {
          const active = preset.id === selectedPreset;

          return (
            <button
              key={preset.id}
              type="button"
              disabled={isBusy}
              onClick={() => onPresetChange(preset.id)}
              className={`preset-button ${active ? "preset-button-active" : ""}`}
            >
              <span>{preset.label}</span>
              <small>{preset.hint}</small>
            </button>
          );
        })}
      </div>

      <label className="mt-4 block text-xs text-slate-500">
        Output format
        <select
          className="field mt-2"
          value={selectedFormat}
          disabled={isBusy}
          onChange={(event) => onFormatChange(event.currentTarget.value as PhotoCompressionFormat)}
        >
          <option value="jpeg">JPEG, most compatible</option>
          <option value="webp" disabled={!hasWebp}>
            WebP, smaller files{hasWebp ? "" : " unavailable"}
          </option>
        </select>
      </label>

      {!hasWebp ? <p className="mt-3 text-xs text-slate-500">WebP encoder not detected in FFmpeg.</p> : null}
      {mediaSelected && !hasImage ? (
        <p className="mt-3 text-xs text-slate-500">Photo compression is only available for image files.</p>
      ) : null}

      <button type="button" className="primary-button mt-4 w-full" onClick={onCompress} disabled={!canCompress}>
        Compress Photo
      </button>
      <p className="mt-3 text-xs leading-5 text-slate-500">
        JPEG is the safest target for sharing. WebP usually gets smaller files when your bundled FFmpeg supports it.
      </p>
    </section>
  );
}
