import { AlertCircle, Film, Image as ImageIcon, LoaderCircle, Music } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { extensionFromPath, filenameFromPath, secondsToTimestamp } from "../lib/format";

const VIDEO_PREVIEW_EXTENSIONS = new Set(["mp4", "mov", "webm", "m4v"]);
const AUDIO_PREVIEW_EXTENSIONS = new Set(["mp3", "wav", "m4a", "aac", "ogg", "oga", "opus", "flac"]);
const IMAGE_PREVIEW_EXTENSIONS = new Set(["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"]);

type VideoPlayerProps = {
  filePath: string | null;
  previewUrl: string | null;
  previewState: "idle" | "native" | "preparing" | "ready" | "failed";
  previewMessage: string | null;
  durationSeconds: number | null;
  previewFailed: boolean;
  theaterMode: boolean;
  isAudioOnly: boolean;
  isImage: boolean;
  width: number | null;
  height: number | null;
  onPreviewFailed: () => void;
  onTimeUpdate: (seconds: number) => void;
};

export function canTryPreview(path: string | null): boolean {
  if (!path) {
    return false;
  }

  const extension = extensionFromPath(path);
  return (
    VIDEO_PREVIEW_EXTENSIONS.has(extension) ||
    AUDIO_PREVIEW_EXTENSIONS.has(extension) ||
    IMAGE_PREVIEW_EXTENSIONS.has(extension)
  );
}

export function VideoPlayer({
  filePath,
  previewUrl,
  previewState,
  previewMessage,
  durationSeconds,
  previewFailed,
  theaterMode,
  isAudioOnly,
  isImage,
  width,
  height,
  onPreviewFailed,
  onTimeUpdate,
}: VideoPlayerProps) {
  const mediaRef = useRef<HTMLVideoElement | HTMLAudioElement | null>(null);
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const [viewportSize, setViewportSize] = useState({ width: 0, height: 0 });
  const [intrinsicSize, setIntrinsicSize] = useState<{ width: number; height: number } | null>(null);
  const canPreview = !!previewUrl && !previewFailed;
  const HeaderIcon = isImage ? ImageIcon : isAudioOnly ? Music : Film;

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
    if (isImage || !mediaRef.current || !previewUrl) {
      return;
    }

    setIntrinsicSize(null);
    mediaRef.current.load();
  }, [isAudioOnly, isImage, previewUrl]);

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
  }, [canPreview, isAudioOnly, isImage, theaterMode]);

  return (
    <section className="flex h-full min-h-0 flex-col rounded-lg border border-white/10 bg-black shadow-panel">
      <div className="flex shrink-0 items-center justify-between border-b border-white/10 px-4 py-3">
        <div className="flex min-w-0 items-center gap-2 text-sm text-slate-300">
          <HeaderIcon size={17} className="shrink-0 text-hit-300" />
          <span className="truncate">{filenameFromPath(filePath)}</span>
        </div>
        <span className="text-xs text-slate-500">{secondsToTimestamp(durationSeconds)}</span>
      </div>

      <div className="relative min-h-0 flex-1 overflow-hidden rounded-b-lg bg-black">
        {canPreview ? (
          isImage ? (
            <div
              ref={viewportRef}
              className={
                theaterMode
                  ? "absolute inset-x-4 bottom-10 top-4 flex items-center justify-center overflow-hidden"
                  : "absolute inset-0 flex items-center justify-center overflow-hidden"
              }
            >
              <div className="flex max-h-full max-w-full items-center justify-center" style={fittedFrameStyle}>
                <img
                  src={previewUrl}
                  alt={filenameFromPath(filePath)}
                  className="block h-full w-full rounded-md bg-black object-contain"
                  onError={onPreviewFailed}
                  onLoad={(event) => {
                    const image = event.currentTarget;
                    if (image.naturalWidth > 0 && image.naturalHeight > 0) {
                      setIntrinsicSize({ width: image.naturalWidth, height: image.naturalHeight });
                    }
                  }}
                />
              </div>
            </div>
          ) : isAudioOnly ? (
            <div className="absolute inset-0 grid place-items-center overflow-hidden px-6">
              <div className="w-full max-w-3xl rounded-lg border border-white/10 bg-ink-900/80 p-6 shadow-panel">
                <div className="mb-5 flex items-center gap-4">
                  <div className="grid h-14 w-14 shrink-0 place-items-center rounded-lg border border-white/10 bg-white/[0.06] text-hit-300">
                    <Music size={28} />
                  </div>
                  <div className="min-w-0">
                    <p className="truncate text-base font-semibold text-white">{filenameFromPath(filePath)}</p>
                    <p className="mt-1 text-sm text-slate-500">Audio preview</p>
                  </div>
                </div>
                <audio
                  ref={(node) => {
                    mediaRef.current = node;
                  }}
                  className="hit-audio-preview w-full"
                  controls
                  preload="metadata"
                  onError={onPreviewFailed}
                  onTimeUpdate={(event) => onTimeUpdate(event.currentTarget.currentTime)}
                >
                  <source src={previewUrl ?? undefined} />
                </audio>
              </div>
            </div>
          ) : (
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
                  ref={(node) => {
                    mediaRef.current = node;
                  }}
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
          )
        ) : (
          <div className="absolute inset-0 grid place-items-center">
            <div className="mx-auto max-w-lg px-8 text-center">
              <div className="mx-auto mb-5 grid h-16 w-16 place-items-center rounded-lg border border-white/10 bg-white/5 text-hit-300">
                {previewState === "preparing" ? (
                  <LoaderCircle size={30} className="animate-spin" />
                ) : filePath && isImage ? (
                  <ImageIcon size={30} />
                ) : (
                  <AlertCircle size={30} />
                )}
              </div>
              <h2 className="text-lg font-semibold text-white">
                {filePath
                  ? previewState === "preparing"
                    ? "Preparing playable preview..."
                    : "Preview is not available for this file."
                  : "Open media to get started."}
              </h2>
              <p className="mt-2 text-sm leading-6 text-slate-400">
                {filePath
                  ? previewMessage ??
                    "This file may not preview in HitPlayer yet, but FFmpeg may still process it."
                  : "Open a video, audio clip, or photo to preview it here."}
              </p>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
