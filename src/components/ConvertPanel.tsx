import { RefreshCw } from "lucide-react";

type ConvertPanelProps = {
  hasVideo: boolean;
  isBusy: boolean;
  onConvert: () => void;
};

export function ConvertPanel({ hasVideo, isBusy, onConvert }: ConvertPanelProps) {
  return (
    <section className="rounded-lg border border-white/10 bg-ink-850 p-4 shadow-panel">
      <div className="mb-4 flex items-center gap-2">
        <RefreshCw size={18} className="text-hit-300" />
        <h2 className="text-sm font-semibold uppercase text-slate-300">Convert</h2>
      </div>

      <button type="button" className="primary-button w-full" onClick={onConvert} disabled={!hasVideo || isBusy}>
        Convert to Compatible MP4
      </button>
      <p className="mt-3 text-xs leading-5 text-slate-500">
        Use this for files that will not preview or play correctly elsewhere.
      </p>
    </section>
  );
}
