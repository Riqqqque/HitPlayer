import { FileVideo, Image as ImageIcon, Music } from "lucide-react";
import { filenameFromPath, formatBitrate, formatDuration, formatFileSize } from "../lib/format";
import type { VideoMetadata } from "../lib/types";

type FileInfoCardProps = {
  filePath: string | null;
  metadata: VideoMetadata | null;
};

function value(text: string | number | null | undefined): string {
  if (text == null || text === "") {
    return "Unknown";
  }

  return String(text);
}

export function FileInfoCard({ filePath, metadata }: FileInfoCardProps) {
  const audioStream = metadata?.streams.find((stream) => stream.codecType === "audio");
  const isImage = metadata?.mediaKind === "image";
  const audioOnly = metadata?.mediaKind === "audio";
  const parsedSampleRate = audioStream?.sampleRate ? Number(audioStream.sampleRate) : null;
  const resolution =
    metadata?.width && metadata?.height
      ? `${metadata.width} x ${metadata.height}`
      : audioOnly
        ? "Audio only"
        : "Unknown";
  const sampleRate = audioStream?.sampleRate
    ? `${parsedSampleRate != null && Number.isFinite(parsedSampleRate) ? parsedSampleRate.toLocaleString() : audioStream.sampleRate} Hz`
    : "Unknown";
  const channels = audioStream?.channels ? `${audioStream.channels} ch` : "Unknown";
  const Icon = isImage ? ImageIcon : audioOnly ? Music : FileVideo;

  return (
    <section className="rounded-lg border border-white/10 bg-ink-850 p-4 shadow-panel">
      <div className="mb-4 flex items-center gap-2">
        <Icon size={18} className="text-hit-300" />
        <h2 className="text-sm font-semibold uppercase text-slate-300">File Info</h2>
      </div>

      <dl className="space-y-3 text-sm">
        <InfoRow label="Filename" value={filenameFromPath(filePath)} />
        <InfoRow label="Container" value={value(metadata?.container)} />
        {!isImage ? <InfoRow label="Duration" value={formatDuration(metadata?.durationSeconds)} /> : null}
        <InfoRow label="Resolution" value={resolution} />
        {isImage ? <InfoRow label="Image codec" value={value(metadata?.imageCodec)} /> : null}
        {!isImage ? <InfoRow label="Video codec" value={value(metadata?.videoCodec)} /> : null}
        {!isImage ? <InfoRow label="Audio codec" value={value(metadata?.audioCodec)} /> : null}
        {!isImage && audioStream ? <InfoRow label="Channels" value={channels} /> : null}
        {!isImage && audioStream ? <InfoRow label="Sample rate" value={sampleRate} /> : null}
        <InfoRow label="File size" value={formatFileSize(metadata?.fileSizeBytes)} />
        {!isImage ? <InfoRow label="Bitrate" value={formatBitrate(metadata?.bitrate)} /> : null}
      </dl>
    </section>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[98px_1fr] gap-3">
      <dt className="text-slate-500">{label}</dt>
      <dd className="min-w-0 truncate text-right text-slate-200" title={value}>
        {value}
      </dd>
    </div>
  );
}
