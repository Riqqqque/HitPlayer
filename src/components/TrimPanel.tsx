import { Scissors } from "lucide-react";
import { formatSelectedDuration, secondsToTimestamp } from "../lib/format";

type TrimPanelProps = {
  hasMedia: boolean;
  isBusy: boolean;
  currentTime: number;
  durationSeconds: number | null;
  startSeconds: number;
  endSeconds: number;
  validationMessage: string | null;
  onSetStart: () => void;
  onSetEnd: () => void;
  onStartChange: (seconds: number) => void;
  onEndChange: (seconds: number) => void;
  onFastTrim: () => void;
  onPreciseTrim: () => void;
};

export function TrimPanel({
  hasMedia,
  isBusy,
  currentTime,
  durationSeconds,
  startSeconds,
  endSeconds,
  validationMessage,
  onSetStart,
  onSetEnd,
  onStartChange,
  onEndChange,
  onFastTrim,
  onPreciseTrim,
}: TrimPanelProps) {
  const canExport = hasMedia && !isBusy && !validationMessage;
  const max = durationSeconds ?? undefined;

  return (
    <section className="rounded-lg border border-white/10 bg-ink-850 p-4 shadow-panel">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Scissors size={18} className="text-hit-300" />
          <h2 className="text-sm font-semibold uppercase text-slate-300">Trim</h2>
        </div>
        <span className="text-xs text-slate-500">{secondsToTimestamp(currentTime)}</span>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <button type="button" className="secondary-button" onClick={onSetStart} disabled={!hasMedia || isBusy}>
          Set Start
        </button>
        <button type="button" className="secondary-button" onClick={onSetEnd} disabled={!hasMedia || isBusy}>
          Set End
        </button>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-3">
        <label className="space-y-1 text-xs text-slate-500">
          Start
          <input
            className="field"
            type="number"
            min={0}
            max={max}
            step={0.001}
            value={Number.isFinite(startSeconds) ? startSeconds : 0}
            onChange={(event) => onStartChange(Number(event.target.value))}
            disabled={!hasMedia || isBusy}
          />
          <span className="block text-[11px] text-slate-400">{secondsToTimestamp(startSeconds)}</span>
        </label>
        <label className="space-y-1 text-xs text-slate-500">
          End
          <input
            className="field"
            type="number"
            min={0}
            max={max}
            step={0.001}
            value={Number.isFinite(endSeconds) ? endSeconds : 0}
            onChange={(event) => onEndChange(Number(event.target.value))}
            disabled={!hasMedia || isBusy}
          />
          <span className="block text-[11px] text-slate-400">{secondsToTimestamp(endSeconds)}</span>
        </label>
      </div>

      <div className="mt-3 rounded-lg border border-white/10 bg-black/25 p-3">
        <div className="flex items-center justify-between text-sm">
          <span className="text-slate-500">Selected duration</span>
          <strong className="font-semibold text-white">{formatSelectedDuration(startSeconds, endSeconds)}</strong>
        </div>
        {validationMessage ? <p className="mt-2 text-xs text-amber-300">{validationMessage}</p> : null}
      </div>

      <div className="mt-4 grid grid-cols-2 gap-3">
        <button type="button" className="primary-button" onClick={onFastTrim} disabled={!canExport}>
          Fast Trim
        </button>
        <button type="button" className="primary-button" onClick={onPreciseTrim} disabled={!canExport}>
          Precise Trim
        </button>
      </div>

      <p className="mt-3 text-xs leading-5 text-slate-500">
        Fast Trim is instant with no quality loss and keeps the source container. Precise Trim re-encodes for cleaner
        exact cuts.
      </p>
    </section>
  );
}
