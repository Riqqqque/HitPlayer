import { FolderOpen, Maximize2, Minimize2, Settings } from "lucide-react";
import appIcon from "../assets/app-icon.png";

type TopBarProps = {
  onOpenVideo: () => void;
  onOpenSettings: () => void;
  isBusy: boolean;
  theaterMode: boolean;
  onToggleTheaterMode: () => void;
};

export function TopBar({
  onOpenVideo,
  onOpenSettings,
  isBusy,
  theaterMode,
  onToggleTheaterMode,
}: TopBarProps) {
  return (
    <header className="flex shrink-0 items-center justify-between border-b border-white/10 bg-ink-950 px-6 py-4">
      <div className="flex items-center gap-3">
        <img
          src={appIcon}
          alt=""
          className="h-10 w-10 rounded-lg border border-white/15 bg-black object-cover shadow-inner shadow-white/10"
        />
        <div>
          <h1 className="text-xl font-semibold tracking-normal text-white">HitPlayer</h1>
          <p className="text-sm text-slate-400">Play it. Cut it. Compress it.</p>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={onOpenSettings}
          className="secondary-button h-10 w-10 px-0"
          title="Settings"
          aria-label="Settings"
        >
          <Settings size={17} />
        </button>
        <button type="button" onClick={onToggleTheaterMode} className="secondary-button">
          {theaterMode ? <Minimize2 size={17} /> : <Maximize2 size={17} />}
          {theaterMode ? "Exit Theater" : "Theater"}
        </button>
        <button
          type="button"
          onClick={onOpenVideo}
          disabled={isBusy}
          className="primary-button gap-2 px-4 shadow-lg shadow-sky-950/40"
        >
          <FolderOpen size={18} />
          Open Media
        </button>
      </div>
    </header>
  );
}
