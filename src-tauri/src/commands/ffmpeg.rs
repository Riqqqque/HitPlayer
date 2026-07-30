use crate::commands::ffprobe::probe_video_internal;
use crate::commands::jobs::{path_arg, run_ffmpeg_job, FfmpegJob, JobManager};
use crate::models::{
    CompressOptions, CompressionPreset, ConvertOptions, EncoderSupport, JobResult, MediaKind,
    PhotoCompressOptions, PhotoCompressionFormat, PhotoCompressionPreset, PreviewResult,
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
const PREVIEW_CACHE_VERSION: u8 = 2;
const MIN_USEFUL_COMPRESSION_BITRATE_BPS: u64 = 96_000;

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
        let validation = validate_trim(&app, input, options.start_seconds, options.end_seconds)?;
        let fast_extension = source_extension(
            input,
            if has_video_stream(&validation.metadata) {
                "mp4"
            } else {
                "m4a"
            },
        );
        let output_path = default_output_path(
            input,
            "trim_fast",
            options.output_path.as_deref(),
            options.output_directory.as_deref(),
            fast_extension,
        )?;
        let mut args = vec![
            "-y".to_string(),
            "-ss".to_string(),
            seconds_arg(options.start_seconds),
            "-to".to_string(),
            seconds_arg(options.end_seconds),
            "-i".to_string(),
            path_arg(input),
        ];

        if has_video_stream(&validation.metadata) {
            args.extend([
                "-map".to_string(),
                "0:v:0".to_string(),
                "-map".to_string(),
                "0:a?".to_string(),
                "-sn".to_string(),
                "-dn".to_string(),
                "-c".to_string(),
                "copy".to_string(),
            ]);
        } else {
            args.extend([
                "-map".to_string(),
                "0:a:0".to_string(),
                "-vn".to_string(),
                "-sn".to_string(),
                "-dn".to_string(),
                "-c:a".to_string(),
                "copy".to_string(),
            ]);
        }
        if supports_faststart(fast_extension) {
            args.extend(["-movflags".to_string(), "+faststart".to_string()]);
        }
        args.extend(progress_args());
        args.push(path_arg(&output_path));

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Fast Trim".to_string(),
                args,
                output_path,
                total_duration: Some(validation.selected_duration),
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
        let validation = validate_trim(&app, input, options.start_seconds, options.end_seconds)?;
        let metadata = validation.metadata;
        let output_extension = if has_video_stream(&metadata) {
            "mp4"
        } else {
            "m4a"
        };
        let output_path = default_output_path(
            input,
            "trim_precise",
            options.output_path.as_deref(),
            options.output_directory.as_deref(),
            output_extension,
        )?;
        let mut args = vec![
            "-y".to_string(),
            "-ss".to_string(),
            seconds_arg(options.start_seconds),
            "-to".to_string(),
            seconds_arg(options.end_seconds),
            "-i".to_string(),
            path_arg(input),
        ];

        if has_video_stream(&metadata) {
            let plan = trim_encode_plan(&metadata);
            args.extend([
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
            ]);
            args.extend(low_impact_thread_args());
            args.extend([
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
            ]);
        } else {
            args.extend([
                "-map".to_string(),
                "0:a:0".to_string(),
                "-vn".to_string(),
                "-sn".to_string(),
                "-dn".to_string(),
                "-c:a".to_string(),
                "aac".to_string(),
                "-b:a".to_string(),
                "192k".to_string(),
                "-movflags".to_string(),
                "+faststart".to_string(),
            ]);
        }
        args.extend(progress_args());
        args.push(path_arg(&output_path));

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Precise Trim".to_string(),
                args,
                output_path,
                total_duration: Some(validation.selected_duration),
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

        let metadata = probe_video_internal(&app, input)?;
        if !has_video_stream(&metadata) {
            return Err(
                "Video compression is only available for video files right now.".to_string(),
            );
        }
        if source_is_too_small_to_compress(&metadata) {
            return Err(
                "This video is already too small to compress without increasing its size."
                    .to_string(),
            );
        }
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

        args.extend(compression_args(&options.preset, Some(&metadata)));
        args.extend(["-movflags".to_string(), "+faststart".to_string()]);
        args.extend(progress_args());
        args.push(path_arg(&output_path));

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Compress Video".to_string(),
                args,
                output_path,
                total_duration: metadata.duration_seconds,
            },
        )
    })
    .await
    .map_err(|error| format!("Compression failed: {error}"))?
}

#[tauri::command]
pub async fn compress_photo(
    app: AppHandle,
    state: State<'_, JobManager>,
    options: PhotoCompressOptions,
) -> Result<JobResult, String> {
    let manager = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let input = Path::new(&options.input_path);
        if !input.is_file() {
            return Err("Select a photo first.".to_string());
        }

        if matches!(options.format, PhotoCompressionFormat::Webp) {
            let encoders = detect_encoders_internal(&app)?;
            if !encoders.has_libwebp {
                return Err("WebP encoder unavailable in this FFmpeg build.".to_string());
            }
        }

        let metadata = probe_video_internal(&app, input)?;
        if metadata.media_kind != MediaKind::Image {
            return Err("Photo compression is only available for image files.".to_string());
        }

        let output_path = default_output_path(
            input,
            options.preset.suffix(),
            options.output_path.as_deref(),
            options.output_directory.as_deref(),
            options.format.extension(),
        )?;
        let mut args = vec![
            "-y".to_string(),
            "-i".to_string(),
            path_arg(input),
            "-map".to_string(),
            "0:v:0".to_string(),
            "-frames:v".to_string(),
            "1".to_string(),
        ];
        args.extend(photo_scale_args(&metadata, &options.preset));
        args.extend(photo_compression_args(&options.preset, &options.format));
        args.extend(progress_args());
        args.push(path_arg(&output_path));

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Compress Photo".to_string(),
                args,
                output_path,
                total_duration: Some(1.0),
            },
        )
    })
    .await
    .map_err(|error| format!("Photo compression failed: {error}"))?
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

        let metadata = probe_video_internal(&app, input)?;
        if !has_video_stream(&metadata) {
            return Err("MP4 conversion is only available for video files right now.".to_string());
        }
        let output_path = default_output_path(
            input,
            "converted",
            options.output_path.as_deref(),
            options.output_directory.as_deref(),
            "mp4",
        )?;
        let plan = convert_encode_plan(Some(&metadata));
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
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "veryfast".to_string(),
        ];
        args.extend(low_impact_thread_args());
        args.extend([
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
        ]);
        args.extend(progress_args());
        args.push(path_arg(&output_path));

        run_ffmpeg_job(
            app,
            manager,
            FfmpegJob {
                name: "Convert to Compatible MP4".to_string(),
                args,
                output_path,
                total_duration: metadata.duration_seconds,
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
        has_libx264: encoder_available(&encoders, "libx264"),
        has_libx265: encoder_available(&encoders, "libx265"),
        has_libwebp: encoder_available(&encoders, "libwebp"),
        has_h264_nvenc: encoder_available(&encoders, "h264_nvenc"),
        has_hevc_nvenc: encoder_available(&encoders, "hevc_nvenc"),
        has_h264_amf: encoder_available(&encoders, "h264_amf"),
        has_h264_qsv: encoder_available(&encoders, "h264_qsv"),
    })
}

fn encoder_available(encoders: &str, name: &str) -> bool {
    encoders.lines().any(|line| {
        let mut fields = line.split_whitespace();
        let _flags = fields.next();
        fields.next() == Some(name)
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
            "Could not open this media file. Check that the network drive is connected and the file is still available."
                .to_string(),
        );
    }

    let metadata = probe_video_internal(app, input)?;
    let is_image = metadata.media_kind == MediaKind::Image;
    if !is_image && !has_video_stream(&metadata) && !has_audio_stream(&metadata) {
        return Err("This file does not contain a supported audio or video stream.".to_string());
    }

    let audio_only = is_audio_only(&metadata);
    let can_copy_video = !is_image
        && !audio_only
        && !force_transcode
        && metadata
            .video_codec
            .as_deref()
            .is_some_and(|codec| codec.eq_ignore_ascii_case("h264"));
    let method = if is_image {
        "image_preview"
    } else if audio_only {
        "audio_preview"
    } else if can_copy_video {
        "stream_copy"
    } else {
        "transcode"
    };
    let preview_extension = if is_image {
        "png"
    } else if audio_only {
        "m4a"
    } else {
        "mp4"
    };
    let output_path = preview_cache_path(input, method, preview_extension)?;

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
    let work_path = preview_work_path(&output_path)?;
    let _ = fs::remove_file(&work_path);

    let mut args = vec!["-y".to_string(), "-i".to_string(), path_arg(input)];

    if is_image {
        args.extend([
            "-map".to_string(),
            "0:v:0".to_string(),
            "-frames:v".to_string(),
            "1".to_string(),
        ]);
        args.extend(preview_scale_args(&metadata));
        args.extend([
            "-c:v".to_string(),
            "png".to_string(),
            "-update".to_string(),
            "1".to_string(),
        ]);
    } else if audio_only {
        args.extend([
            "-map".to_string(),
            "0:a:0".to_string(),
            "-vn".to_string(),
            "-sn".to_string(),
            "-dn".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
        ]);
    } else if can_copy_video {
        args.extend([
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "0:a?".to_string(),
            "-sn".to_string(),
            "-dn".to_string(),
            "-c:v".to_string(),
            "copy".to_string(),
        ]);
        args.extend(preview_audio_args(metadata.audio_codec.as_deref()));
    } else {
        args.extend([
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "0:a?".to_string(),
            "-sn".to_string(),
            "-dn".to_string(),
        ]);
        args.extend(preview_scale_args(&metadata));
        args.extend([
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "veryfast".to_string(),
        ]);
        args.extend(low_impact_thread_args());
        args.extend([
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

    if !is_image {
        args.extend(["-movflags".to_string(), "+faststart".to_string()]);
    }
    args.extend(progress_args());
    args.push(path_arg(&work_path));

    let result = run_ffmpeg_job(
        app.clone(),
        manager,
        FfmpegJob {
            name: "Preparing Preview".to_string(),
            args,
            output_path: work_path.clone(),
            total_duration: if is_image {
                Some(1.0)
            } else {
                metadata.duration_seconds
            },
        },
    )?;

    if result.canceled {
        return Err("Preview preparation was canceled.".to_string());
    }

    if !result.success {
        let _ = fs::remove_file(&work_path);
        return Err(
            "This file cannot be previewed yet, but FFmpeg may still process it.".to_string(),
        );
    }

    publish_preview(&work_path, &output_path)?;

    Ok(PreviewResult {
        preview_path: output_path.to_string_lossy().to_string(),
        used_cached_file: false,
        method: method.to_string(),
        log: result.log,
    })
}

struct ValidatedTrim {
    selected_duration: f64,
    metadata: VideoMetadata,
}

fn validate_trim(
    app: &AppHandle,
    input: &Path,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<ValidatedTrim, String> {
    if !input.is_file() {
        return Err("Select a media file first.".to_string());
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
    if !has_video_stream(&metadata) && !has_audio_stream(&metadata) {
        return Err("This file does not contain a supported audio or video stream.".to_string());
    }

    if let Some(duration) = metadata.duration_seconds {
        if end_seconds > duration + 0.001 {
            return Err("End must be within the media duration.".to_string());
        }
    }

    Ok(ValidatedTrim {
        selected_duration,
        metadata,
    })
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

    if !matches!(preset, CompressionPreset::NvidiaFast) {
        args.extend(low_impact_thread_args());
    }

    args.extend([
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        bitrate_arg(plan.audio_bitrate_bps),
    ]);
    args
}

fn photo_compression_args(
    preset: &PhotoCompressionPreset,
    format: &PhotoCompressionFormat,
) -> Vec<String> {
    match format {
        PhotoCompressionFormat::Jpeg => vec![
            "-c:v".to_string(),
            "mjpeg".to_string(),
            "-q:v".to_string(),
            photo_jpeg_quality(preset).to_string(),
            "-pix_fmt".to_string(),
            "yuvj420p".to_string(),
            "-update".to_string(),
            "1".to_string(),
        ],
        PhotoCompressionFormat::Webp => vec![
            "-c:v".to_string(),
            "libwebp".to_string(),
            "-quality".to_string(),
            photo_webp_quality(preset).to_string(),
            "-compression_level".to_string(),
            photo_webp_compression_level(preset).to_string(),
        ],
    }
}

fn photo_scale_args(metadata: &VideoMetadata, preset: &PhotoCompressionPreset) -> Vec<String> {
    let Some((width, height)) = photo_scaled_dimensions(
        metadata.width,
        metadata.height,
        photo_long_edge_limit(preset),
    ) else {
        return Vec::new();
    };

    vec![
        "-vf".to_string(),
        format!("scale={width}:{height}:flags=lanczos"),
    ]
}

fn photo_long_edge_limit(preset: &PhotoCompressionPreset) -> u32 {
    match preset {
        PhotoCompressionPreset::Balanced => 2560,
        PhotoCompressionPreset::Small => 1920,
        PhotoCompressionPreset::HighQuality => 3840,
    }
}

fn photo_jpeg_quality(preset: &PhotoCompressionPreset) -> u8 {
    match preset {
        PhotoCompressionPreset::Balanced => 5,
        PhotoCompressionPreset::Small => 8,
        PhotoCompressionPreset::HighQuality => 3,
    }
}

fn photo_webp_quality(preset: &PhotoCompressionPreset) -> u8 {
    match preset {
        PhotoCompressionPreset::Balanced => 78,
        PhotoCompressionPreset::Small => 62,
        PhotoCompressionPreset::HighQuality => 88,
    }
}

fn photo_webp_compression_level(preset: &PhotoCompressionPreset) -> u8 {
    match preset {
        PhotoCompressionPreset::Balanced => 4,
        PhotoCompressionPreset::Small => 6,
        PhotoCompressionPreset::HighQuality => 4,
    }
}

fn photo_scaled_dimensions(
    width: Option<u32>,
    height: Option<u32>,
    long_edge_limit: u32,
) -> Option<(u32, u32)> {
    let width = width?;
    let height = height?;
    let longest = width.max(height);

    if width == 0 || height == 0 {
        return None;
    }

    let (scaled_width, scaled_height) = if longest > long_edge_limit {
        let scale = f64::from(long_edge_limit) / f64::from(longest);
        (
            ((f64::from(width) * scale).round() as u32).max(1),
            ((f64::from(height) * scale).round() as u32).max(1),
        )
    } else {
        (width, height)
    };
    let scaled_width = make_even(scaled_width);
    let scaled_height = make_even(scaled_height);

    if scaled_width == width && scaled_height == height {
        return None;
    }

    Some((scaled_width, scaled_height))
}

fn make_even(value: u32) -> u32 {
    value.saturating_sub(value % 2).max(2)
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
            let source_cap = bitrate.saturating_mul(95) / 100;
            let floor_bps = (audio_bps + 250_000).min(bitrate.saturating_mul(9) / 10);
            target_bps
                .min(cap_bps)
                .max(floor_bps)
                .min(source_cap.max(1))
                .max(48_000.min(source_cap.max(1)))
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

fn source_is_too_small_to_compress(metadata: &VideoMetadata) -> bool {
    source_bitrate_bps(Some(metadata))
        .is_some_and(|bitrate| bitrate < MIN_USEFUL_COMPRESSION_BITRATE_BPS)
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

fn has_video_stream(metadata: &VideoMetadata) -> bool {
    metadata.video_codec.is_some()
}

fn has_audio_stream(metadata: &VideoMetadata) -> bool {
    metadata.audio_codec.is_some()
}

fn is_audio_only(metadata: &VideoMetadata) -> bool {
    !has_video_stream(metadata) && has_audio_stream(metadata)
}

fn source_extension<'a>(input: &'a Path, fallback: &'a str) -> &'a str {
    input
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::trim)
        .filter(|extension| !extension.is_empty())
        .unwrap_or(fallback)
}

fn supports_faststart(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "mp4" | "m4v" | "mov" | "m4a"
    )
}

fn progress_args() -> Vec<String> {
    vec![
        "-progress".to_string(),
        "pipe:1".to_string(),
        "-stats_period".to_string(),
        "1".to_string(),
        "-nostats".to_string(),
    ]
}

fn low_impact_thread_args() -> Vec<String> {
    vec![
        "-threads".to_string(),
        low_impact_thread_count().to_string(),
    ]
}

fn low_impact_thread_count() -> usize {
    let cores = std::thread::available_parallelism()
        .map(|cores| cores.get())
        .unwrap_or(4);

    match cores {
        0..=4 => 1,
        5..=8 => 2,
        _ => (cores / 3).clamp(2, 4),
    }
}

fn preview_scale_args(metadata: &VideoMetadata) -> Vec<String> {
    preview_scaled_dimensions(metadata.width, metadata.height)
        .map(|(width, height)| vec!["-vf".to_string(), format!("scale={width}:{height}")])
        .unwrap_or_default()
}

fn preview_scaled_dimensions(width: Option<u32>, height: Option<u32>) -> Option<(u32, u32)> {
    let width = width?;
    let height = height?;

    if width == 0 || height == 0 || (width <= 1920 && height <= 1080) {
        return None;
    }

    let scale = (1920.0 / f64::from(width)).min(1080.0 / f64::from(height));
    let mut scaled_width = ((f64::from(width) * scale).round() as u32).max(2);
    let mut scaled_height = ((f64::from(height) * scale).round() as u32).max(2);

    if !scaled_width.is_multiple_of(2) {
        scaled_width -= 1;
    }
    if !scaled_height.is_multiple_of(2) {
        scaled_height -= 1;
    }

    Some((scaled_width.max(2), scaled_height.max(2)))
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
        command.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
    command
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;

fn preview_cache_path(
    input: &Path,
    method: &str,
    extension: &str,
) -> Result<std::path::PathBuf, String> {
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

    Ok(cache_dir.join(format!("{stem}_{method}_{key}.{extension}")))
}

fn preview_cache_key(input: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    PREVIEW_CACHE_VERSION.hash(&mut hasher);
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

fn preview_work_path(output_path: &Path) -> Result<std::path::PathBuf, String> {
    let parent = output_path
        .parent()
        .ok_or_else(|| "Could not create preview cache.".to_string())?;
    let stem = output_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("preview");
    let extension = output_path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("mp4");

    Ok(parent.join(format!(
        "{stem}.part-{}.{}",
        uuid::Uuid::new_v4(),
        extension
    )))
}

fn publish_preview(work_path: &Path, output_path: &Path) -> Result<(), String> {
    if output_file_ready(output_path) {
        let _ = fs::remove_file(work_path);
        return Ok(());
    }

    match fs::rename(work_path, output_path) {
        Ok(()) => Ok(()),
        Err(_) if output_file_ready(output_path) => {
            let _ = fs::remove_file(work_path);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(work_path);
            Err(format!("Could not save the prepared preview: {error}"))
        }
    }
}

fn output_file_ready(path: &Path) -> bool {
    path.is_file()
        && path
            .metadata()
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
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
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("mp4")
                    || extension.eq_ignore_ascii_case("m4a")
                    || extension.eq_ignore_ascii_case("png")
            });
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
            media_kind: MediaKind::Video,
            video_codec: Some("h264".to_string()),
            audio_codec: Some("aac".to_string()),
            image_codec: None,
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
    fn extremely_low_bitrate_sources_are_rejected_before_compression() {
        let metadata = metadata(120.0, 512 * 1024, Some(35_000));

        assert!(source_is_too_small_to_compress(&metadata));
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

    #[test]
    fn faststart_is_only_used_for_iso_media_outputs() {
        assert!(supports_faststart("mp4"));
        assert!(supports_faststart("MOV"));
        assert!(supports_faststart("m4a"));
        assert!(!supports_faststart("mkv"));
        assert!(!supports_faststart("mp3"));
    }

    #[test]
    fn encoder_detection_requires_an_exact_encoder_name() {
        let output = " V..... libx264rgb RGB encoder\n V..... libwebp WebP encoder";

        assert!(!encoder_available(output, "libx264"));
        assert!(encoder_available(output, "libwebp"));
    }

    #[test]
    fn low_impact_thread_count_keeps_cpu_headroom() {
        let threads = low_impact_thread_count();

        assert!((1..=4).contains(&threads));
    }

    #[test]
    fn progress_args_emit_at_one_second_intervals() {
        let args = progress_args();

        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "-progress" && pair[1] == "pipe:1"));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "-stats_period" && pair[1] == "1"));
    }

    #[test]
    fn preview_transcode_scales_4k_to_1080p() {
        assert_eq!(
            preview_scaled_dimensions(Some(3840), Some(2160)),
            Some((1920, 1080))
        );
    }

    #[test]
    fn preview_transcode_leaves_1080p_alone() {
        assert_eq!(preview_scaled_dimensions(Some(1920), Some(1080)), None);
    }

    #[test]
    fn small_photo_preset_scales_long_edge_to_1920() {
        assert_eq!(
            photo_scaled_dimensions(
                Some(4000),
                Some(3000),
                photo_long_edge_limit(&PhotoCompressionPreset::Small)
            ),
            Some((1920, 1440))
        );
    }

    #[test]
    fn photo_dimensions_are_even_for_encoder_compatibility() {
        assert_eq!(
            photo_scaled_dimensions(Some(801), Some(601), 1920),
            Some((800, 600))
        );
        assert_eq!(
            photo_scaled_dimensions(Some(4001), Some(3000), 1920),
            Some((1920, 1440))
        );
    }

    #[test]
    fn photo_presets_get_more_aggressive_for_small_files() {
        assert!(
            photo_jpeg_quality(&PhotoCompressionPreset::Small)
                > photo_jpeg_quality(&PhotoCompressionPreset::Balanced)
        );
        assert!(
            photo_webp_quality(&PhotoCompressionPreset::Small)
                < photo_webp_quality(&PhotoCompressionPreset::Balanced)
        );
    }
}
