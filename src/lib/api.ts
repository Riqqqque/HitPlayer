import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type {
  CompressionPreset,
  EncoderSupport,
  JobResult,
  PreviewResult,
  TrimOptions,
  VideoMetadata,
} from "./types";

export function openVideoDialog(): Promise<string | null> {
  return invoke("open_video_dialog");
}

export function openOutputFolderDialog(currentPath?: string | null): Promise<string | null> {
  return invoke("open_output_folder_dialog", { currentPath });
}

export function getLaunchVideoPath(): Promise<string | null> {
  return invoke("get_launch_video_path");
}

export function probeVideo(path: string): Promise<VideoMetadata> {
  return invoke("probe_video", { path });
}

export function detectEncoders(): Promise<EncoderSupport> {
  return invoke("detect_encoders");
}

export function preparePreview(path: string, forceTranscode = false): Promise<PreviewResult> {
  return invoke("prepare_preview", { path, forceTranscode });
}

export function fastTrim(options: TrimOptions): Promise<JobResult> {
  return invoke("fast_trim", { options });
}

export function preciseTrim(options: TrimOptions): Promise<JobResult> {
  return invoke("precise_trim", { options });
}

export function compressVideo(
  inputPath: string,
  preset: CompressionPreset,
  outputPath?: string,
  outputDirectory?: string,
): Promise<JobResult> {
  return invoke("compress_video", { options: { inputPath, preset, outputPath, outputDirectory } });
}

export function convertToMp4(
  inputPath: string,
  outputPath?: string,
  outputDirectory?: string,
): Promise<JobResult> {
  return invoke("convert_to_mp4", { options: { inputPath, outputPath, outputDirectory } });
}

export function cancelJob(): Promise<void> {
  return invoke("cancel_job");
}

export function registerDefaultPlayer(): Promise<void> {
  return invoke("register_default_player");
}

export function openDefaultPlayerSettings(): Promise<void> {
  return invoke("open_default_player_settings");
}

export function openOutputFolder(path: string): Promise<void> {
  return invoke("open_output_folder", { path });
}

export function revealOutputFile(path: string): Promise<void> {
  return invoke("reveal_output_file", { path });
}

export function toAssetUrl(path: string): string {
  return convertFileSrc(path);
}
