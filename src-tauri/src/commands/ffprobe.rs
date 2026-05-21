use crate::models::{StreamInfo, VideoMetadata};
use crate::paths::binary_path;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::AppHandle;

#[tauri::command]
pub async fn probe_video(app: AppHandle, path: String) -> Result<VideoMetadata, String> {
    tauri::async_runtime::spawn_blocking(move || probe_video_internal(&app, Path::new(&path)))
        .await
        .map_err(|error| format!("Could not read video metadata: {error}"))?
}

pub fn probe_video_internal(app: &AppHandle, input: &Path) -> Result<VideoMetadata, String> {
    if !input.is_file() {
        return Err("Could not read video metadata.".to_string());
    }

    let file_size_bytes = std::fs::metadata(input)
        .map_err(|_| "Could not read video metadata.".to_string())?
        .len();

    let ffprobe = binary_path(app, "ffprobe")?;
    let output = command_no_window(ffprobe)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Could not read video metadata: {error}"))?;

    if !output.status.success() {
        return Err("Could not read video metadata.".to_string());
    }

    let response: FfprobeResponse = serde_json::from_slice(&output.stdout)
        .map_err(|_| "Could not read video metadata.".to_string())?;

    Ok(metadata_from_response(response, file_size_bytes))
}

fn metadata_from_response(response: FfprobeResponse, file_size_bytes: u64) -> VideoMetadata {
    let streams: Vec<StreamInfo> = response
        .streams
        .iter()
        .map(|stream| StreamInfo {
            index: stream.index.unwrap_or_default(),
            codec_type: stream.codec_type.clone(),
            codec_name: stream.codec_name.clone(),
            width: stream.width,
            height: stream.height,
            channels: stream.channels,
            sample_rate: stream.sample_rate.clone(),
        })
        .collect();

    let video_stream = response
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"));
    let audio_stream = response
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"));

    let duration_seconds = response
        .format
        .as_ref()
        .and_then(|format| parse_optional_f64(format.duration.as_deref()))
        .or_else(|| video_stream.and_then(|stream| parse_optional_f64(stream.duration.as_deref())));

    let format = response.format.as_ref();

    VideoMetadata {
        duration_seconds,
        width: video_stream.and_then(|stream| stream.width),
        height: video_stream.and_then(|stream| stream.height),
        video_codec: video_stream.and_then(|stream| stream.codec_name.clone()),
        audio_codec: audio_stream.and_then(|stream| stream.codec_name.clone()),
        container: format.and_then(|format| format.format_name.clone()),
        bitrate: format.and_then(|format| parse_optional_u64(format.bit_rate.as_deref())),
        file_size_bytes,
        streams,
    }
}

fn parse_optional_f64(value: Option<&str>) -> Option<f64> {
    value
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
}

fn parse_optional_u64(value: Option<&str>) -> Option<u64> {
    value.and_then(|value| value.parse::<u64>().ok())
}

fn command_no_window(program: PathBuf) -> Command {
    let mut command = Command::new(program);
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

#[derive(Debug, Deserialize)]
struct FfprobeResponse {
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    index: Option<u32>,
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    channels: Option<u32>,
    sample_rate: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    format_name: Option<String>,
    duration: Option<String>,
    bit_rate: Option<String>,
}
