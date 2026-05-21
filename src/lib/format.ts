export function secondsToTimestamp(seconds: number | null | undefined): string {
  if (seconds == null || Number.isNaN(seconds) || !Number.isFinite(seconds)) {
    return "--:--:--.---";
  }

  const clamped = Math.max(0, seconds);
  const hours = Math.floor(clamped / 3600);
  const minutes = Math.floor((clamped % 3600) / 60);
  const wholeSeconds = Math.floor(clamped % 60);
  const millis = Math.floor((clamped - Math.floor(clamped)) * 1000);

  return `${hours.toString().padStart(2, "0")}:${minutes
    .toString()
    .padStart(2, "0")}:${wholeSeconds.toString().padStart(2, "0")}.${millis
    .toString()
    .padStart(3, "0")}`;
}

export function formatDuration(seconds: number | null | undefined): string {
  return secondsToTimestamp(seconds);
}

export function formatSelectedDuration(startSeconds: number, endSeconds: number): string {
  return secondsToTimestamp(Math.max(0, endSeconds - startSeconds));
}

export function formatFileSize(bytes: number | null | undefined): string {
  if (bytes == null || bytes < 0) {
    return "Unknown";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }

  const digits = unitIndex === 0 ? 0 : 1;
  return `${size.toFixed(digits)} ${units[unitIndex]}`;
}

export function formatBitrate(bitsPerSecond: number | null | undefined): string {
  if (!bitsPerSecond || bitsPerSecond <= 0) {
    return "Unknown";
  }

  if (bitsPerSecond >= 1_000_000) {
    return `${(bitsPerSecond / 1_000_000).toFixed(2)} Mbps`;
  }

  return `${Math.round(bitsPerSecond / 1000)} Kbps`;
}

export function filenameFromPath(path: string | null): string {
  if (!path) {
    return "No file selected";
  }

  const normalized = path.replaceAll("\\", "/");
  return normalized.slice(normalized.lastIndexOf("/") + 1);
}

export function dirnameFromPath(path: string | null): string {
  if (!path) {
    return "";
  }

  const normalized = path.replaceAll("\\", "/");
  const index = normalized.lastIndexOf("/");
  return index >= 0 ? normalized.slice(0, index) : "";
}

export function extensionFromPath(path: string): string {
  const filename = filenameFromPath(path);
  const index = filename.lastIndexOf(".");
  return index >= 0 ? filename.slice(index + 1).toLowerCase() : "";
}
