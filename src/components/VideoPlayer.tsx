import { AlertCircle, Film, Image as ImageIcon, LoaderCircle, Music } from "lucide-react";
import { memo, useEffect, useRef } from "react";
import { extensionFromPath, filenameFromPath, secondsToTimestamp } from "../lib/format";
import type { PlaybackAudioState } from "../lib/types";

const VIDEO_PREVIEW_EXTENSIONS = new Set(["mp4", "mov", "webm", "m4v"]);
const AUDIO_PREVIEW_EXTENSIONS = new Set(["mp3", "wav", "m4a", "aac", "ogg", "oga", "opus", "flac"]);
const IMAGE_PREVIEW_EXTENSIONS = new Set(["jpg", "jpeg", "png", "webp", "gif", "bmp"]);

type VideoPlayerProps = {
  filePath: string | null;
  previewUrl: string | null;
  previewState: "idle" | "native" | "preparing" | "ready" | "failed";
  previewMessage: string | null;
  durationSeconds: number | null;
  previewFailed: boolean;
  isAudioOnly: boolean;
  isImage: boolean;
  playbackAudio: PlaybackAudioState;
  onPlaybackAudioChange: (audio: PlaybackAudioState) => void;
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

function VideoPlayerComponent({
  filePath,
  previewUrl,
  previewState,
  previewMessage,
  durationSeconds,
  previewFailed,
  isAudioOnly,
  isImage,
  playbackAudio,
  onPlaybackAudioChange,
  onPreviewFailed,
  onTimeUpdate,
}: VideoPlayerProps) {
  const mediaRef = useRef<HTMLVideoElement | HTMLAudioElement | null>(null);
  const canPreview = !!previewUrl && !previewFailed;
  const HeaderIcon = isImage ? ImageIcon : isAudioOnly ? Music : Film;

  useEffect(() => {
    if (isImage || !mediaRef.current || !previewUrl) {
      return;
    }

    mediaRef.current.load();
  }, [isAudioOnly, isImage, previewUrl]);

  useEffect(() => {
    const media = mediaRef.current;
    if (!media || isImage) {
      return;
    }

    if (Math.abs(media.volume - playbackAudio.volume) >= 0.001) {
      media.volume = playbackAudio.volume;
    }
    if (media.muted !== playbackAudio.muted) {
      media.muted = playbackAudio.muted;
    }
  }, [isAudioOnly, isImage, playbackAudio.muted, playbackAudio.volume, previewUrl]);

  function handleVolumeChange(media: HTMLMediaElement) {
    onPlaybackAudioChange({ volume: media.volume, muted: media.muted });
  }

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
            <div className="absolute inset-0 flex min-h-0 items-center justify-center overflow-hidden bg-black">
              <img
                src={previewUrl}
                alt={filenameFromPath(filePath)}
                className="block h-full w-full bg-black object-contain"
                onError={onPreviewFailed}
              />
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
                  preload="auto"
                  onError={onPreviewFailed}
                  onTimeUpdate={(event) => onTimeUpdate(event.currentTarget.currentTime)}
                  onVolumeChange={(event) => handleVolumeChange(event.currentTarget)}
                >
                  <source src={previewUrl ?? undefined} />
                </audio>
              </div>
            </div>
          ) : (
            <div className="absolute inset-0 flex min-h-0 items-center justify-center overflow-hidden bg-black">
              <video
                ref={(node) => {
                  mediaRef.current = node;
                }}
                className="hit-video-preview block h-full w-full bg-black object-contain"
                controls
                playsInline
                preload="auto"
                onError={onPreviewFailed}
                onTimeUpdate={(event) => onTimeUpdate(event.currentTarget.currentTime)}
                onVolumeChange={(event) => handleVolumeChange(event.currentTarget)}
              >
                <source src={previewUrl ?? undefined} />
              </video>
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

export const VideoPlayer = memo(
  VideoPlayerComponent,
  (previous, next) =>
    previous.filePath === next.filePath &&
    previous.previewUrl === next.previewUrl &&
    previous.previewState === next.previewState &&
    previous.previewMessage === next.previewMessage &&
    previous.durationSeconds === next.durationSeconds &&
    previous.previewFailed === next.previewFailed &&
    previous.isAudioOnly === next.isAudioOnly &&
    previous.isImage === next.isImage &&
    previous.playbackAudio.volume === next.playbackAudio.volume &&
    previous.playbackAudio.muted === next.playbackAudio.muted,
);
