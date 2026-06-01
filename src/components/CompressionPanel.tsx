import { Gauge } from "lucide-react";
import type { CompressionPreset, EncoderSupport } from "../lib/types";

type CompressionPanelProps = {
  selectedPreset: CompressionPreset;
  encoders: EncoderSupport | null;
  hasVideo: boolean;
  isBusy: boolean;
  onPresetChange: (preset: CompressionPreset) => void;
  onCompress: () => void;
};

const PRESETS: Array<{ id: CompressionPreset; label: string; hint: string }> = [
  { id: "balanced", label: "Balanced", hint: "Targets about 45 MB per 2 min" },
  { id: "small", label: "Small File", hint: "Harder squeeze" },
  { id: "high_quality", label: "High Quality", hint: "Cleaner, still capped" },
  { id: "nvidia_fast", label: "NVIDIA Fast", hint: "Uses NVENC" },
];

export function CompressionPanel({
  selectedPreset,
  encoders,
  hasVideo,
  isBusy,
  onPresetChange,
  onCompress,
}: CompressionPanelProps) {
  const hasNvenc = !!encoders?.hasH264Nvenc;
  const selectedNvidiaUnavailable = selectedPreset === "nvidia_fast" && !hasNvenc;
  const canCompress = hasVideo && !isBusy && !selectedNvidiaUnavailable;

  return (
    <section className="rounded-lg border border-white/10 bg-ink-850 p-4 shadow-panel">
      <div className="mb-4 flex items-center gap-2">
        <Gauge size={18} className="text-hit-300" />
        <h2 className="text-sm font-semibold uppercase text-slate-300">Compression</h2>
      </div>

      <div className="grid grid-cols-2 gap-2">
        {PRESETS.map((preset) => {
          const disabled = preset.id === "nvidia_fast" && !hasNvenc;
          const active = preset.id === selectedPreset;

          return (
            <button
              key={preset.id}
              type="button"
              disabled={disabled || isBusy}
              onClick={() => onPresetChange(preset.id)}
              className={`preset-button ${active ? "preset-button-active" : ""}`}
            >
              <span>{preset.label}</span>
              <small>{disabled ? "NVENC not detected" : preset.hint}</small>
            </button>
          );
        })}
      </div>

      {!hasNvenc ? <p className="mt-3 text-xs text-slate-500">NVIDIA NVENC not detected.</p> : null}

      <button type="button" className="primary-button mt-4 w-full" onClick={onCompress} disabled={!canCompress}>
        Compress Video
      </button>
      <p className="mt-3 text-xs leading-5 text-slate-500">
        CPU presets run with extra headroom for games and OBS. NVIDIA Fast uses NVENC, so avoid it while OBS is
        already using NVENC.
      </p>
    </section>
  );
}
