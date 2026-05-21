import { ChevronDown, FolderOpen, Loader2, Square, Wand2 } from "lucide-react";
import { useState } from "react";
import { revealOutputFile, openOutputFolder } from "../lib/api";
import type { JobProgress, JobResult } from "../lib/types";

type ProgressPanelProps = {
  jobName: string;
  progress: JobProgress | null;
  result: JobResult | null;
  isBusy: boolean;
  detailsLog: string;
  onCancel: () => void;
  onError: (message: string) => void;
};

export function ProgressPanel({
  jobName,
  progress,
  result,
  isBusy,
  detailsLog,
  onCancel,
  onError,
}: ProgressPanelProps) {
  const [detailsOpen, setDetailsOpen] = useState(false);
  const percent = progress?.percent ?? 0;
  const isIndeterminate = percent < 0;
  const displayPercent = Math.max(0, Math.min(100, percent));
  const statusText = progress?.message ?? (result?.error || (result?.success ? "Finished." : "Ready."));
  const outputPath = result?.outputPath;

  async function handleOpenFolder() {
    if (!outputPath) {
      return;
    }

    try {
      await openOutputFolder(outputPath);
    } catch (error) {
      onError(String(error));
    }
  }

  async function handleRevealFile() {
    if (!outputPath) {
      return;
    }

    try {
      await revealOutputFile(outputPath);
    } catch (error) {
      onError(String(error));
    }
  }

  return (
    <section className="rounded-lg border border-white/10 bg-ink-900 p-4 shadow-panel">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <div className="grid h-10 w-10 shrink-0 place-items-center rounded-lg border border-white/10 bg-white/5 text-hit-300">
            {isBusy ? <Loader2 size={19} className="animate-spin" /> : <Wand2 size={19} />}
          </div>
          <div className="min-w-0">
            <h2 className="truncate text-sm font-semibold text-white">{jobName || "No active job"}</h2>
            <p className="truncate text-xs text-slate-400">{statusText}</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button type="button" className="secondary-button" onClick={onCancel} disabled={!isBusy}>
            <Square size={14} />
            Cancel
          </button>
          <button type="button" className="secondary-button" onClick={handleOpenFolder} disabled={!outputPath}>
            <FolderOpen size={15} />
            Open Folder
          </button>
          <button type="button" className="secondary-button" onClick={handleRevealFile} disabled={!outputPath}>
            Reveal File
          </button>
        </div>
      </div>

      <div className="mt-4 grid grid-cols-[1fr_auto] items-center gap-3">
        <div className="progress-track">
          <div
            className={isIndeterminate ? "progress-bar progress-bar-indeterminate" : "progress-bar"}
            style={isIndeterminate ? undefined : { width: `${displayPercent}%` }}
          />
        </div>
        <span className="w-16 text-right text-sm tabular-nums text-slate-300">
          {isIndeterminate ? "--" : `${displayPercent.toFixed(0)}%`}
        </span>
      </div>

      <div className="mt-3 flex flex-wrap gap-x-5 gap-y-2 text-xs text-slate-500">
        <span>Speed: {progress?.speed ?? "Unknown"}</span>
        <span>FPS: {progress?.fps?.toFixed(1) ?? "Unknown"}</span>
      </div>

      <button
        type="button"
        className="mt-4 inline-flex items-center gap-1 text-xs font-medium text-slate-400 transition hover:text-white"
        onClick={() => setDetailsOpen((open) => !open)}
      >
        <ChevronDown size={15} className={detailsOpen ? "rotate-180 transition" : "transition"} />
        Details
      </button>

      {detailsOpen ? (
        <pre className="mt-3 max-h-48 overflow-auto rounded-lg border border-white/10 bg-black/40 p-3 text-xs leading-5 text-slate-300">
          {detailsLog || "No FFmpeg log yet."}
        </pre>
      ) : null}
    </section>
  );
}
