export type CompressionPreset = "balanced" | "small" | "high_quality" | "nvidia_fast";

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
  videoCodec: string | null;
  audioCodec: string | null;
  container: string | null;
  bitrate: number | null;
  fileSizeBytes: number;
  streams: StreamInfo[];
};

export type EncoderSupport = {
  hasLibx264: boolean;
  hasLibx265: boolean;
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

export type TrimOptions = {
  inputPath: string;
  startSeconds: number;
  endSeconds: number;
  outputPath?: string;
  outputDirectory?: string;
};
