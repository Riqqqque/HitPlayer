use crate::commands::ffprobe::probe_video_internal;
use crate::commands::jobs::{path_arg, run_ffmpeg_job, FfmpegJob, JobManager};
use crate::models::{
    CompressOptions, CompressionPreset, ConvertOptions, EncoderSupport, JobResult, PreviewResult,
    TrimOptions, VideoMetadata,
};
use crate::paths::{binary_path, default_output_path};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};

const PREVIEW_CACHE_DAYS: u64 = 7;

#[tauri::command]
pub async fn detect_encoders(app: AppHandle) -> Result<EncoderSupport, String> {
    tauri::async_runtime::spawn_blocking(move || detect_encoders_internal(&app))
        .await
        .map_err(|error| format!("Could not detect encoders: {error}"))?
}

#[tauri::command]
pub async fn prepare_preview(
    app: AppHandle,
    state: State<'_, JobManager>,
    path: String,
    force_transcode: Option<bool>,
) -> Result<PreviewResult, String> {
    let manager = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        prepare_preview_internal(
            &app,
            manager,
            Path::new(&path),
            force_transcode.unwrap_or(false),
        )
    })
    .await
    .map_err(|error| format!("Preview preparation failed: {error}"))?
}

#[tauri::command]
pub async fn fast_trim(
    app: AppHandle,
    state: State<'_, JobManager>,
    options: TrimOptions,
) -> Result<JobResult, String> {
    let manager = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let input = Path::new(&options.input_path);
        let duration = validate_trim(&app, input, options.start_seconds, options.end_seconds)?;
        let fast_extension = input
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| !extension.trim().is_empty())
            .unwrap_or("mp4");
        let output_path = default_output_path(
            input,
            "trim_fast",
            options.output_path.as_deref(),
            options.output_directory.as_deref(),
            fast_extension,
        )?;
        let args = vec![
            "-y".to_string(),
            "-ss".to_string(),
            seconds_arg(options.start_seconds),
            "-to".to_string(),
            seconds_arg(options.end_seconds),
            "-i".to_string(),
            path_arg(input),
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "0:a?".to_string(),
            "-sn".to_string(),
            "-dn".to_string(),
            "-c".to_string(),
            "copy".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            "-nostats".to_string(),
            path_arg(&output_path),
        ];

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Fast Trim".to_string(),
                args,
                output_path,
                total_duration: Some(duration),
            },
        )
    })
    .await
    .map_err(|error| format!("Fast Trim failed: {error}"))?
}

#[tauri::command]
pub async fn precise_trim(
    app: AppHandle,
    state: State<'_, JobManager>,
    options: TrimOptions,
) -> Result<JobResult, String> {
    let manager = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let input = Path::new(&options.input_path);
        let duration = validate_trim(&app, input, options.start_seconds, options.end_seconds)?;
        let metadata = probe_video_internal(&app, input)?;
        let output_path = default_output_path(
            input,
            "trim_precise",
            options.output_path.as_deref(),
            options.output_directory.as_deref(),
            "mp4",
        )?;
        let plan = trim_encode_plan(&metadata);
        let args = vec![
            "-y".to_string(),
            "-ss".to_string(),
            seconds_arg(options.start_seconds),
            "-to".to_string(),
            seconds_arg(options.end_seconds),
            "-i".to_string(),
            path_arg(input),
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "0:a?".to_string(),
            "-sn".to_string(),
            "-dn".to_string(),
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "veryfast".to_string(),
            "-b:v".to_string(),
            bitrate_arg(plan.video_bitrate_bps),
            "-maxrate".to_string(),
            bitrate_arg(plan.maxrate_bps),
            "-bufsize".to_string(),
            bitrate_arg(plan.bufsize_bps),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            bitrate_arg(plan.audio_bitrate_bps),
            "-movflags".to_string(),
            "+faststart".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            "-nostats".to_string(),
            path_arg(&output_path),
        ];

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Precise Trim".to_string(),
                args,
                output_path,
                total_duration: Some(duration),
            },
        )
    })
    .await
    .map_err(|error| format!("Precise Trim failed: {error}"))?
}

#[tauri::command]
pub async fn compress_video(
    app: AppHandle,
    state: State<'_, JobManager>,
    options: CompressOptions,
) -> Result<JobResult, String> {
    let manager = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let input = Path::new(&options.input_path);
        if !input.is_file() {
            return Err("Select a video first.".to_string());
        }

        if matches!(options.preset, CompressionPreset::NvidiaFast) {
            let encoders = detect_encoders_internal(&app)?;
            if !encoders.has_h264_nvenc {
                return Err("NVIDIA encoder unavailable on this system.".to_string());
            }
        }

        let metadata = probe_video_internal(&app, input).ok();
        let output_path = default_output_path(
            input,
            options.preset.suffix(),
            options.output_path.as_deref(),
            options.output_directory.as_deref(),
            "mp4",
        )?;
        let mut args = vec![
            "-y".to_string(),
            "-i".to_string(),
            path_arg(input),
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "0:a?".to_string(),
            "-sn".to_string(),
            "-dn".to_string(),
        ];

        args.extend(compression_args(&options.preset, metadata.as_ref()));
        args.extend([
            "-movflags".to_string(),
            "+faststart".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            "-nostats".to_string(),
            path_arg(&output_path),
        ]);

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Compress Video".to_string(),
                args,
                output_path,
                total_duration: metadata.and_then(|metadata| metadata.duration_seconds),
            },
        )
    })
    .await
    .map_err(|error| format!("Compression failed: {error}"))?
}

#[tauri::command]
pub async fn convert_to_mp4(
    app: AppHandle,
    state: State<'_, JobManager>,
    options: ConvertOptions,
) -> Result<JobResult, String> {
    let manager = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let input = Path::new(&options.input_path);
        if !input.is_file() {
            return Err("Select a video first.".to_string());
        }

        let metadata = probe_video_internal(&app, input).ok();
        let output_path = default_output_path(
            input,
            "converted",
            options.output_path.as_deref(),
            options.output_directory.as_deref(),
            "mp4",
        )?;
        let plan = convert_encode_plan(metadata.as_ref());
        let args = vec![
            "-y".to_string(),
            "-i".to_string(),
            path_arg(input),
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "0:a?".to_string(),
            "-sn".to_string(),
            "-dn".to_string(),
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "veryfast".to_string(),
            "-b:v".to_string(),
            bitrate_arg(plan.video_bitrate_bps),
            "-maxrate".to_string(),
            bitrate_arg(plan.maxrate_bps),
            "-bufsize".to_string(),
            bitrate_arg(plan.bufsize_bps),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            bitrate_arg(plan.audio_bitrate_bps),
            "-movflags".to_string(),
            "+faststart".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            "-nostats".to_string(),
            path_arg(&output_path),
        ];

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Convert to Compatible MP4".to_string(),
                args,
                output_path,
                total_duration: metadata.and_then(|metadata| metadata.duration_seconds),
            },
        )
    })
    .await
    .map_err(|error| format!("Conversion failed: {error}"))?
}

pub fn detect_encoders_internal(app: &AppHandle) -> Result<EncoderSupport, String> {
    let ffmpeg = binary_path(app, "ffmpeg")?;
    let output = command_no_window(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Could not detect encoders: {error}"))?;

    if !output.status.success() {
        return Err("Could not detect encoders.".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let encoders = format!("{stdout}\n{stderr}");

    Ok(EncoderSupport {
        has_libx264: encoders.contains("libx264"),
        has_libx265: encoders.contains("libx265"),
        has_h264_nvenc: encoders.contains("h264_nvenc"),
        has_hevc_nvenc: encoders.contains("hevc_nvenc"),
        has_h264_amf: encoders.contains("h264_amf"),
        has_h264_qsv: encoders.contains("h264_qsv"),
    })
}

fn prepare_preview_internal(
    app: &AppHandle,
    manager: JobManager,
    input: &Path,
    force_transcode: bool,
) -> Result<PreviewResult, String> {
    if !input.is_file() {
        return Err(
            "Could not open this video. Check that the network drive is connected and the file is still available."
                .to_string(),
        );
    }

    let metadata = probe_video_internal(app, input)?;
    let can_copy_video = !force_transcode
        && metadata
            .video_codec
            .as_deref()
            .is_some_and(|codec| codec.eq_ignore_ascii_case("h264"));
    let method = if can_copy_video {
        "stream_copy"
    } else {
        "transcode"
    };
    let output_path = preview_cache_path(input, method)?;

    cleanup_preview_cache(output_path.parent().unwrap_or_else(|| Path::new("")));

    if output_path.is_file()
        && output_path
            .metadata()
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
    {
        return Ok(PreviewResult {
            preview_path: output_path.to_string_lossy().to_string(),
            used_cached_file: true,
            method: method.to_string(),
            log: String::new(),
        });
    }

    let _ = fs::remove_file(&output_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create preview cache: {error}"))?;
    }

    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        path_arg(input),
        "-map".to_string(),
        "0:v:0".to_string(),
        "-map".to_string(),
        "0:a?".to_string(),
        "-sn".to_string(),
        "-dn".to_string(),
    ];

    if can_copy_video {
        args.extend(["-c:v".to_string(), "copy".to_string()]);
        args.extend(preview_audio_args(metadata.audio_codec.as_deref()));
    } else {
        args.extend([
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "veryfast".to_string(),
            "-crf".to_string(),
            "23".to_string(),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
        ]);
    }

    args.extend([
        "-movflags".to_string(),
        "+faststart".to_string(),
        "-progress".to_string(),
        "pipe:1".to_string(),
        "-nostats".to_string(),
        path_arg(&output_path),
    ]);

    let result = run_ffmpeg_job(
        app.clone(),
        manager,
        FfmpegJob {
            name: "Preparing Preview".to_string(),
            args,
            output_path: output_path.clone(),
            total_duration: metadata.duration_seconds,
        },
    )?;

    if result.canceled {
        return Err("Preview preparation was canceled.".to_string());
    }

    if !result.success {
        let _ = fs::remove_file(&output_path);
        return Err(
            "This file cannot be previewed yet, but FFmpeg can still process it.".to_string(),
        );
    }

    Ok(PreviewResult {
        preview_path: output_path.to_string_lossy().to_string(),
        used_cached_file: false,
        method: method.to_string(),
        log: result.log,
    })
}

fn validate_trim(
    app: &AppHandle,
    input: &Path,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<f64, String> {
    if !input.is_file() {
        return Err("Select a video first.".to_string());
    }
    if !start_seconds.is_finite() || !end_seconds.is_finite() {
        return Err("Invalid trim range.".to_string());
    }
    if start_seconds < 0.0 {
        return Err("Start must be at least 0.".to_string());
    }
    if end_seconds <= start_seconds {
        return Err("End must be greater than start.".to_string());
    }

    let selected_duration = end_seconds - start_seconds;
    if selected_duration <= 0.1 {
        return Err("Selected duration must be greater than 0.1 seconds.".to_string());
    }

    let metadata = probe_video_internal(app, input)?;
    if let Some(duration) = metadata.duration_seconds {
        if end_seconds > duration + 0.001 {
            return Err("End must be within the video duration.".to_string());
        }
    }

    Ok(selected_duration)
}

#[derive(Debug, Clone, Copy)]
struct EncodePlan {
    video_bitrate_bps: u64,
    audio_bitrate_bps: u64,
    maxrate_bps: u64,
    bufsize_bps: u64,
}

fn compression_args(preset: &CompressionPreset, metadata: Option<&VideoMetadata>) -> Vec<String> {
    let plan = compression_encode_plan(preset, metadata);
    let mut args = match preset {
        CompressionPreset::NvidiaFast => vec![
            "-c:v".to_string(),
            "h264_nvenc".to_string(),
            "-preset".to_string(),
            "p5".to_string(),
            "-b:v".to_string(),
            bitrate_arg(plan.video_bitrate_bps),
            "-maxrate".to_string(),
            bitrate_arg(plan.maxrate_bps),
            "-bufsize".to_string(),
            bitrate_arg(plan.bufsize_bps),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
        ],
        CompressionPreset::HighQuality => vec![
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "fast".to_string(),
            "-b:v".to_string(),
            bitrate_arg(plan.video_bitrate_bps),
            "-maxrate".to_string(),
            bitrate_arg(plan.maxrate_bps),
            "-bufsize".to_string(),
            bitrate_arg(plan.bufsize_bps),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
        ],
        _ => vec![
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "veryfast".to_string(),
            "-b:v".to_string(),
            bitrate_arg(plan.video_bitrate_bps),
            "-maxrate".to_string(),
            bitrate_arg(plan.maxrate_bps),
            "-bufsize".to_string(),
            bitrate_arg(plan.bufsize_bps),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
        ],
    };

    args.extend([
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        bitrate_arg(plan.audio_bitrate_bps),
    ]);
    args
}

fn compression_encode_plan(
    preset: &CompressionPreset,
    metadata: Option<&VideoMetadata>,
) -> EncodePlan {
    let source_bitrate = source_bitrate_bps(metadata);
    let (source_factor, cap_bps, fallback_bps, audio_bps) = match preset {
        CompressionPreset::Balanced => (0.58, 3_050_000, 3_050_000, 128_000),
        CompressionPreset::Small => (0.40, 1_800_000, 1_800_000, 96_000),
        CompressionPreset::HighQuality => (0.78, 4_600_000, 4_000_000, 160_000),
        CompressionPreset::NvidiaFast => (0.58, 3_050_000, 3_050_000, 128_000),
    };

    let total_bps = match source_bitrate {
        Some(bitrate) => {
            let target_bps = ((bitrate as f64) * source_factor) as u64;
            let floor_bps = (audio_bps + 250_000).min(bitrate.saturating_mul(9) / 10);
            target_bps
                .min(cap_bps)
                .max(floor_bps)
                .min(bitrate.saturating_mul(95) / 100)
                .max(96_000)
        }
        None => fallback_bps.min(cap_bps).max(audio_bps + 250_000),
    };

    encode_plan_from_total(total_bps, audio_bps)
}

fn trim_encode_plan(metadata: &VideoMetadata) -> EncodePlan {
    let source_bitrate = source_bitrate_bps(Some(metadata))
        .unwrap_or_else(|| resolution_default_bitrate(metadata.width, metadata.height, 3_000_000));
    let total_bps = capped_reencode_total(source_bitrate, 0.82, 600_000, 5_000_000);
    encode_plan_from_total(total_bps, 128_000)
}

fn convert_encode_plan(metadata: Option<&VideoMetadata>) -> EncodePlan {
    let source_bitrate = source_bitrate_bps(metadata);
    let fallback = metadata
        .map(|metadata| resolution_default_bitrate(metadata.width, metadata.height, 3_050_000))
        .unwrap_or(3_050_000);
    let total_bps = source_bitrate
        .map(|bitrate| capped_reencode_total(bitrate, 0.72, 500_000, 3_400_000))
        .unwrap_or(fallback.clamp(500_000, 3_400_000));

    encode_plan_from_total(total_bps, 128_000)
}

fn capped_reencode_total(
    source_bitrate: u64,
    source_factor: f64,
    floor_bps: u64,
    cap_bps: u64,
) -> u64 {
    let target_bps = ((source_bitrate as f64) * source_factor) as u64;
    let source_cap = source_bitrate.saturating_mul(95) / 100;

    target_bps
        .min(cap_bps)
        .max(floor_bps.min(source_cap))
        .min(source_cap.max(1))
        .max(48_000.min(source_cap.max(1)))
}

fn source_bitrate_bps(metadata: Option<&VideoMetadata>) -> Option<u64> {
    let metadata = metadata?;
    metadata.bitrate.or_else(|| {
        metadata.duration_seconds.and_then(|duration| {
            if duration > 0.0 {
                Some(((metadata.file_size_bytes as f64 * 8.0) / duration) as u64)
            } else {
                None
            }
        })
    })
}

fn resolution_default_bitrate(width: Option<u32>, height: Option<u32>, fallback: u64) -> u64 {
    match (width.unwrap_or_default(), height.unwrap_or_default()) {
        (w, h) if w >= 1900 || h >= 1000 => 3_050_000,
        (w, h) if w >= 1200 || h >= 700 => 2_200_000,
        (w, h) if w >= 700 || h >= 400 => 1_350_000,
        (w, h) if w > 0 || h > 0 => 850_000,
        _ => fallback,
    }
}

fn encode_plan_from_total(total_bps: u64, audio_bps: u64) -> EncodePlan {
    let total_bps = total_bps.max(48_000);
    let minimum_video_bps = 64_000.min(total_bps.saturating_sub(32_000).max(1));
    let maximum_audio_bps = total_bps.saturating_sub(minimum_video_bps).max(32_000);
    let preferred_audio_bps = if total_bps < 500_000 {
        64_000
    } else {
        audio_bps
    };
    let usable_audio_bps = preferred_audio_bps
        .min(maximum_audio_bps)
        .max(32_000.min(maximum_audio_bps));
    let video_bitrate_bps = total_bps
        .saturating_sub(usable_audio_bps)
        .max(minimum_video_bps);

    EncodePlan {
        video_bitrate_bps,
        audio_bitrate_bps: usable_audio_bps,
        maxrate_bps: ((video_bitrate_bps as f64) * 1.35) as u64,
        bufsize_bps: video_bitrate_bps.saturating_mul(2),
    }
}

fn bitrate_arg(bits_per_second: u64) -> String {
    format!("{}k", (bits_per_second / 1000).max(1))
}

fn preview_audio_args(codec: Option<&str>) -> Vec<String> {
    if codec.is_some_and(is_mp4_copy_safe_audio_codec) {
        vec!["-c:a".to_string(), "copy".to_string()]
    } else {
        vec![
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
        ]
    }
}

fn is_mp4_copy_safe_audio_codec(codec: &str) -> bool {
    matches!(
        codec.to_ascii_lowercase().as_str(),
        "aac" | "mp3" | "alac" | "ac3" | "eac3"
    )
}

fn seconds_arg(seconds: f64) -> String {
    format!("{seconds:.3}")
}

fn command_no_window(program: std::path::PathBuf) -> Command {
    let mut command = Command::new(program);
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn preview_cache_path(input: &Path, method: &str) -> Result<std::path::PathBuf, String> {
    let key = preview_cache_key(input);
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(safe_cache_stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "video".to_string());
    let cache_dir = std::env::temp_dir().join("HitPlayer").join("PreviewCache");

    if cache_dir.as_os_str().is_empty() {
        return Err("Could not create preview cache.".to_string());
    }

    Ok(cache_dir.join(format!("{stem}_{method}_{key}.mp4")))
}

fn preview_cache_key(input: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    input.to_string_lossy().to_lowercase().hash(&mut hasher);

    if let Ok(metadata) = input.metadata() {
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.duration_since(UNIX_EPOCH) {
                elapsed.as_secs().hash(&mut hasher);
                elapsed.subsec_nanos().hash(&mut hasher);
            }
        }
    }

    format!("{:016x}", hasher.finish())
}

fn safe_cache_stem(stem: &str) -> String {
    stem.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn cleanup_preview_cache(cache_dir: &Path) {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    let cutoff = Duration::from_secs(PREVIEW_CACHE_DAYS * 24 * 60 * 60);
    let now = SystemTime::now();

    for entry in entries.flatten() {
        let path = entry.path();
        let is_preview = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"));
        if !is_preview {
            continue;
        }

        let is_old = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > cutoff);
        if is_old {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(
        duration_seconds: f64,
        file_size_bytes: u64,
        bitrate: Option<u64>,
    ) -> VideoMetadata {
        VideoMetadata {
            duration_seconds: Some(duration_seconds),
            width: Some(1920),
            height: Some(1080),
            video_codec: Some("h264".to_string()),
            audio_codec: Some("aac".to_string()),
            container: Some("mov,mp4,m4a,3gp,3g2,mj2".to_string()),
            bitrate,
            file_size_bytes,
            streams: Vec::new(),
        }
    }

    fn total_bitrate(plan: EncodePlan) -> u64 {
        plan.video_bitrate_bps + plan.audio_bitrate_bps
    }

    #[test]
    fn balanced_preset_targets_under_fifty_mb_for_two_minute_eighty_mb_clip() {
        let metadata = metadata(120.0, 80 * 1024 * 1024, None);
        let plan = compression_encode_plan(&CompressionPreset::Balanced, Some(&metadata));
        let estimated_bytes = (total_bitrate(plan) as f64 * 120.0 / 8.0) as u64;

        assert!(estimated_bytes < 50 * 1024 * 1024);
    }

    #[test]
    fn compression_plan_does_not_inflate_low_bitrate_sources() {
        let metadata = metadata(120.0, 4 * 1024 * 1024, Some(250_000));
        let plan = compression_encode_plan(&CompressionPreset::Balanced, Some(&metadata));

        assert!(total_bitrate(plan) <= 250_000);
    }

    #[test]
    fn small_preset_is_smaller_than_balanced() {
        let metadata = metadata(120.0, 80 * 1024 * 1024, None);
        let balanced = compression_encode_plan(&CompressionPreset::Balanced, Some(&metadata));
        let small = compression_encode_plan(&CompressionPreset::Small, Some(&metadata));

        assert!(total_bitrate(small) < total_bitrate(balanced));
    }

    #[test]
    fn precise_trim_plan_does_not_inflate_low_bitrate_sources() {
        let metadata = metadata(120.0, 4 * 1024 * 1024, Some(250_000));
        let plan = trim_encode_plan(&metadata);

        assert!(total_bitrate(plan) <= 250_000);
    }

    #[test]
    fn convert_plan_does_not_inflate_low_bitrate_sources() {
        let metadata = metadata(120.0, 4 * 1024 * 1024, Some(250_000));
        let plan = convert_encode_plan(Some(&metadata));

        assert!(total_bitrate(plan) <= 250_000);
    }

    #[test]
    fn preview_remux_copies_mp4_safe_audio() {
        let args = preview_audio_args(Some("aac"));

        assert_eq!(args, vec!["-c:a".to_string(), "copy".to_string()]);
    }
}
