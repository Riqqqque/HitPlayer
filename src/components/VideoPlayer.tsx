import { AlertCircle, Film } from "lucide-react";
import { useEffect, useRef } from "react";
import { extensionFromPath, filenameFromPath, secondsToTimestamp } from "../lib/format";

const PREVIEW_EXTENSIONS = new Set(["mp4", "mov", "webm", "m4v"]);

type VideoPlayerProps = {
  filePath: string | null;
  previewUrl: string | null;
  durationSeconds: number | null;
  previewFailed: boolean;
  onPreviewFailed: () => void;
  onTimeUpdate: (seconds: number) => void;
};

export function canTryPreview(path: string | null): boolean {
  return path ? PREVIEW_EXTENSIONS.has(extensionFromPath(path)) : false;
}

export function VideoPlayer({
  filePath,
  previewUrl,
  durationSeconds,
  previewFailed,
  onPreviewFailed,
  onTimeUpdate,
}: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const canPreview = canTryPreview(filePath) && !!previewUrl && !previewFailed;

  useEffect(() => {
    if (!videoRef.current || !previewUrl) {
      return;
    }

    videoRef.current.load();
  }, [previewUrl]);

  return (
    <section className="flex h-full min-h-0 flex-col rounded-lg border border-white/10 bg-black shadow-panel">
      <div className="flex shrink-0 items-center justify-between border-b border-white/10 px-4 py-3">
        <div className="flex min-w-0 items-center gap-2 text-sm text-slate-300">
          <Film size={17} className="shrink-0 text-hit-300" />
          <span className="truncate">{filenameFromPath(filePath)}</span>
        </div>
        <span className="text-xs text-slate-500">{secondsToTimestamp(durationSeconds)}</span>
      </div>

      <div className="relative grid min-h-0 flex-1 place-items-center overflow-hidden rounded-b-lg bg-black">
        {canPreview ? (
          <video
            ref={videoRef}
            className="block h-full w-full bg-black object-contain"
            controls
            preload="metadata"
            onError={onPreviewFailed}
            onTimeUpdate={(event) => onTimeUpdate(event.currentTarget.currentTime)}
          >
            <source src={previewUrl ?? undefined} />
          </video>
        ) : (
          <div className="mx-auto max-w-lg px-8 text-center">
            <div className="mx-auto mb-5 grid h-16 w-16 place-items-center rounded-lg border border-white/10 bg-white/5 text-hit-300">
              <AlertCircle size={30} />
            </div>
            <h2 className="text-lg font-semibold text-white">
              {filePath ? "Preview is not available for this file." : "Open a video to get started."}
            </h2>
            <p className="mt-2 text-sm leading-6 text-slate-400">
              {filePath
                ? "This file may not preview in HitPlayer yet, but FFmpeg can still process it."
                : "Common MP4, MOV, WebM, and M4V files can preview here."}
            </p>
          </div>
        )}
      </div>
    </section>
  );
}
