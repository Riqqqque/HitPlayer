import { AlertCircle, Film, LoaderCircle } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { extensionFromPath, filenameFromPath, secondsToTimestamp } from "../lib/format";

const PREVIEW_EXTENSIONS = new Set(["mp4", "mov", "webm", "m4v"]);

type VideoPlayerProps = {
  filePath: string | null;
  previewUrl: string | null;
  previewState: "idle" | "native" | "preparing" | "ready" | "failed";
  previewMessage: string | null;
  durationSeconds: number | null;
  previewFailed: boolean;
  theaterMode: boolean;
  width: number | null;
  height: number | null;
  onPreviewFailed: () => void;
  onTimeUpdate: (seconds: number) => void;
};

export function canTryPreview(path: string | null): boolean {
  return path ? PREVIEW_EXTENSIONS.has(extensionFromPath(path)) : false;
}

export function VideoPlayer({
  filePath,
  previewUrl,
  previewState,
  previewMessage,
  durationSeconds,
  previewFailed,
  theaterMode,
  width,
  height,
  onPreviewFailed,
  onTimeUpdate,
}: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const [viewportSize, setViewportSize] = useState({ width: 0, height: 0 });
  const [intrinsicSize, setIntrinsicSize] = useState<{ width: number; height: number } | null>(null);
  const canPreview = !!previewUrl && !previewFailed;

  const aspectRatio = useMemo(() => {
    const mediaWidth = width && width > 0 ? width : intrinsicSize?.width;
    const mediaHeight = height && height > 0 ? height : intrinsicSize?.height;

    if (mediaWidth && mediaHeight && mediaWidth > 0 && mediaHeight > 0) {
      return mediaWidth / mediaHeight;
    }

    return 16 / 9;
  }, [height, intrinsicSize, width]);

  const fittedFrameStyle = useMemo<CSSProperties>(() => {
    const maxWidth = viewportSize.width;
    const maxHeight = viewportSize.height;

    if (!maxWidth || !maxHeight) {
      return { height: "100%", width: "100%" };
    }

    let frameWidth = maxWidth;
    let frameHeight = frameWidth / aspectRatio;

    if (frameHeight > maxHeight) {
      frameHeight = maxHeight;
      frameWidth = frameHeight * aspectRatio;
    }

    return {
      height: `${Math.max(1, frameHeight)}px`,
      width: `${Math.max(1, frameWidth)}px`,
    };
  }, [aspectRatio, viewportSize.height, viewportSize.width]);

  useEffect(() => {
    if (!videoRef.current || !previewUrl) {
      return;
    }

    setIntrinsicSize(null);
    videoRef.current.load();
  }, [previewUrl]);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }

    const updateSize = () => {
      const rect = viewport.getBoundingClientRect();
      const nextSize = {
        height: Math.max(0, rect.height),
        width: Math.max(0, rect.width),
      };

      setViewportSize((currentSize) =>
        Math.abs(currentSize.width - nextSize.width) < 0.5 &&
        Math.abs(currentSize.height - nextSize.height) < 0.5
          ? currentSize
          : nextSize,
      );
    };

    updateSize();

    if (typeof ResizeObserver === "undefined") {
      window.addEventListener("resize", updateSize);
      return () => window.removeEventListener("resize", updateSize);
    }

    const observer = new ResizeObserver(updateSize);
    observer.observe(viewport);

    return () => observer.disconnect();
  }, [canPreview, theaterMode]);

  return (
    <section className="flex h-full min-h-0 flex-col rounded-lg border border-white/10 bg-black shadow-panel">
      <div className="flex shrink-0 items-center justify-between border-b border-white/10 px-4 py-3">
        <div className="flex min-w-0 items-center gap-2 text-sm text-slate-300">
          <Film size={17} className="shrink-0 text-hit-300" />
          <span className="truncate">{filenameFromPath(filePath)}</span>
        </div>
        <span className="text-xs text-slate-500">{secondsToTimestamp(durationSeconds)}</span>
      </div>

      <div className="relative min-h-0 flex-1 overflow-hidden rounded-b-lg bg-black">
        {canPreview ? (
          <div
            ref={viewportRef}
            className={
              theaterMode
                ? "absolute inset-x-4 bottom-10 top-4 flex items-center justify-center overflow-hidden"
                : "absolute inset-0 flex items-center justify-center overflow-hidden"
            }
          >
            <div className="flex max-h-full max-w-full items-center justify-center" style={fittedFrameStyle}>
              <video
                ref={videoRef}
                className="hit-video-preview block h-full w-full rounded-md bg-black object-contain"
                controls
                preload="metadata"
                onError={onPreviewFailed}
                onLoadedMetadata={(event) => {
                  const video = event.currentTarget;
                  if (video.videoWidth > 0 && video.videoHeight > 0) {
                    setIntrinsicSize({ width: video.videoWidth, height: video.videoHeight });
                  }
                }}
                onTimeUpdate={(event) => onTimeUpdate(event.currentTarget.currentTime)}
              >
                <source src={previewUrl ?? undefined} />
              </video>
            </div>
          </div>
        ) : (
          <div className="absolute inset-0 grid place-items-center">
            <div className="mx-auto max-w-lg px-8 text-center">
              <div className="mx-auto mb-5 grid h-16 w-16 place-items-center rounded-lg border border-white/10 bg-white/5 text-hit-300">
                {previewState === "preparing" ? (
                  <LoaderCircle size={30} className="animate-spin" />
                ) : (
                  <AlertCircle size={30} />
                )}
              </div>
              <h2 className="text-lg font-semibold text-white">
                {filePath
                  ? previewState === "preparing"
                    ? "Preparing playable preview..."
                    : "Preview is not available for this file."
                  : "Open a video to get started."}
              </h2>
              <p className="mt-2 text-sm leading-6 text-slate-400">
                {filePath
                  ? previewMessage ??
                    "This file may not preview in HitPlayer yet, but FFmpeg can still process it."
                  : "Common MP4, MOV, WebM, M4V, and FFmpeg-prepared MKV previews can play here."}
              </p>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
