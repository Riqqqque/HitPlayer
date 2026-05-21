import { FileVideo } from "lucide-react";
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
  const resolution =
    metadata?.width && metadata?.height ? `${metadata.width} x ${metadata.height}` : "Unknown";

  return (
    <section className="rounded-lg border border-white/10 bg-ink-850 p-4 shadow-panel">
      <div className="mb-4 flex items-center gap-2">
        <FileVideo size={18} className="text-hit-300" />
        <h2 className="text-sm font-semibold uppercase text-slate-300">File Info</h2>
      </div>

      <dl className="space-y-3 text-sm">
        <InfoRow label="Filename" value={filenameFromPath(filePath)} />
        <InfoRow label="Container" value={value(metadata?.container)} />
        <InfoRow label="Duration" value={formatDuration(metadata?.durationSeconds)} />
        <InfoRow label="Resolution" value={resolution} />
        <InfoRow label="Video codec" value={value(metadata?.videoCodec)} />
        <InfoRow label="Audio codec" value={value(metadata?.audioCodec)} />
        <InfoRow label="File size" value={formatFileSize(metadata?.fileSizeBytes)} />
        <InfoRow label="Bitrate" value={formatBitrate(metadata?.bitrate)} />
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
