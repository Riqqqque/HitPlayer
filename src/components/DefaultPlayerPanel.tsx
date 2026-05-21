import { MonitorPlay } from "lucide-react";

type DefaultPlayerPanelProps = {
  isBusy: boolean;
  status: string | null;
  onSetDefaultPlayer: () => void;
};

export function DefaultPlayerPanel({
  isBusy,
  status,
  onSetDefaultPlayer,
}: DefaultPlayerPanelProps) {
  return (
    <section className="rounded-lg border border-white/10 bg-ink-850 p-4 shadow-panel">
      <div className="mb-4 flex items-center gap-2">
        <MonitorPlay size={18} className="text-hit-300" />
        <h2 className="text-sm font-semibold uppercase text-slate-300">Default Player</h2>
      </div>

      <button
        type="button"
        className="primary-button w-full"
        onClick={onSetDefaultPlayer}
        disabled={isBusy}
      >
        Set as Default Player
      </button>
      <p className="mt-3 text-xs leading-5 text-slate-500">
        Registers HitPlayer for this Windows user, then opens Default Apps so you can confirm the video extensions.
      </p>
      {status ? <p className="mt-3 text-xs text-slate-300">{status}</p> : null}
    </section>
  );
}
