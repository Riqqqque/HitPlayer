import { FolderOpen, Maximize2, Minimize2 } from "lucide-react";

type TopBarProps = {
  onOpenVideo: () => void;
  isBusy: boolean;
  theaterMode: boolean;
  onToggleTheaterMode: () => void;
};

export function TopBar({ onOpenVideo, isBusy, theaterMode, onToggleTheaterMode }: TopBarProps) {
  return (
    <header className="flex items-center justify-between border-b border-white/10 bg-ink-950/92 px-6 py-4 backdrop-blur">
      <div className="flex items-center gap-3">
        <div className="grid h-10 w-10 place-items-center rounded-lg border border-white/15 bg-black text-sm font-black text-white shadow-inner shadow-white/10">
          HP
        </div>
        <div>
          <h1 className="text-xl font-semibold tracking-normal text-white">HitPlayer</h1>
          <p className="text-sm text-slate-400">Play it. Cut it. Compress it.</p>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <button type="button" onClick={onToggleTheaterMode} className="secondary-button">
          {theaterMode ? <Minimize2 size={17} /> : <Maximize2 size={17} />}
          {theaterMode ? "Exit Theater" : "Theater"}
        </button>
        <button
          type="button"
          onClick={onOpenVideo}
          disabled={isBusy}
          className="inline-flex items-center gap-2 rounded-lg bg-hit-500 px-4 py-2 text-sm font-semibold text-slate-950 shadow-lg shadow-sky-950/40 transition hover:bg-hit-400 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <FolderOpen size={18} />
          Open Video
        </button>
      </div>
    </header>
  );
}
