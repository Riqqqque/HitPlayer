export type CompressionPreset = "balanced" | "small" | "high_quality" | "nvidia_fast";
export type PhotoCompressionPreset = "balanced" | "small" | "high_quality";
export type PhotoCompressionFormat = "jpeg" | "webp";
export type MediaKind = "video" | "audio" | "image" | "unknown";

export type StreamInfo = {
  index: number;
  codecType: string | null;
  codecName: string | null;
  width: number | null;
  height: number | null;
  channels: number | null;
  sampleRate: string | null;
};

export type VideoMetadata = {
  durationSeconds: number | null;
  width: number | null;
  height: number | null;
  mediaKind: MediaKind;
  videoCodec: string | null;
  audioCodec: string | null;
  imageCodec: string | null;
  container: string | null;
  bitrate: number | null;
  fileSizeBytes: number;
  streams: StreamInfo[];
};

export type EncoderSupport = {
  hasLibx264: boolean;
  hasLibx265: boolean;
  hasLibwebp: boolean;
  hasH264Nvenc: boolean;
  hasHevcNvenc: boolean;
  hasH264Amf: boolean;
  hasH264Qsv: boolean;
};

export type JobPhase = "starting" | "running" | "finished" | "failed" | "canceled";

export type JobProgress = {
  jobId: string;
  phase: JobPhase;
  percent: number;
  outTimeSeconds?: number;
  speed?: string;
  fps?: number;
  message?: string;
};

export type JobResult = {
  success: boolean;
  outputPath: string;
  durationSeconds: number | null;
  log: string;
  canceled: boolean;
  error?: string | null;
};

export type PreviewResult = {
  previewPath: string;
  usedCachedFile: boolean;
  method: "stream_copy" | "transcode" | string;
  log: string;
};

export type TrimOptions = {
  inputPath: string;
  startSeconds: number;
  endSeconds: number;
  outputPath?: string;
  outputDirectory?: string;
};

export type PhotoCompressOptions = {
  inputPath: string;
  preset: PhotoCompressionPreset;
  format: PhotoCompressionFormat;
  outputPath?: string;
  outputDirectory?: string;
};
