use crate::models::{MediaKind, StreamInfo, VideoMetadata};
use crate::paths::binary_path;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::AppHandle;

#[tauri::command]
pub async fn probe_video(app: AppHandle, path: String) -> Result<VideoMetadata, String> {
    tauri::async_runtime::spawn_blocking(move || probe_video_internal(&app, Path::new(&path)))
        .await
        .map_err(|error| format!("Could not read media metadata: {error}"))?
}

pub fn probe_video_internal(app: &AppHandle, input: &Path) -> Result<VideoMetadata, String> {
    if !input.is_file() {
        return Err("Could not read media metadata.".to_string());
    }

    let file_size_bytes = std::fs::metadata(input)
        .map_err(|_| "Could not read media metadata.".to_string())?
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
        .map_err(|error| format!("Could not read media metadata: {error}"))?;

    if !output.status.success() {
        return Err("Could not read media metadata.".to_string());
    }

    let response: FfprobeResponse = serde_json::from_slice(&output.stdout)
        .map_err(|_| "Could not read media metadata.".to_string())?;

    Ok(metadata_from_response(
        response,
        file_size_bytes,
        media_kind_from_path(input),
    ))
}

fn metadata_from_response(
    response: FfprobeResponse,
    file_size_bytes: u64,
    path_kind: Option<MediaKind>,
) -> VideoMetadata {
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

    let visual_stream = response.streams.iter().find(|stream| {
        stream.codec_type.as_deref() == Some("video") && !stream.is_attached_picture()
    });
    let audio_stream = response
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"));
    let media_kind = path_kind.unwrap_or(match (visual_stream, audio_stream) {
        (Some(_), _) => MediaKind::Video,
        (None, Some(_)) => MediaKind::Audio,
        _ => MediaKind::Unknown,
    });

    let duration_seconds = if media_kind == MediaKind::Image {
        None
    } else {
        response
            .format
            .as_ref()
            .and_then(|format| parse_optional_f64(format.duration.as_deref()))
            .or_else(|| {
                visual_stream.and_then(|stream| parse_optional_f64(stream.duration.as_deref()))
            })
            .or_else(|| {
                audio_stream.and_then(|stream| parse_optional_f64(stream.duration.as_deref()))
            })
    };

    let format = response.format.as_ref();
    let is_image = media_kind == MediaKind::Image;

    VideoMetadata {
        duration_seconds,
        width: visual_stream.and_then(|stream| stream.width),
        height: visual_stream.and_then(|stream| stream.height),
        media_kind,
        video_codec: if is_image {
            None
        } else {
            visual_stream.and_then(|stream| stream.codec_name.clone())
        },
        audio_codec: if is_image {
            None
        } else {
            audio_stream.and_then(|stream| stream.codec_name.clone())
        },
        image_codec: if is_image {
            visual_stream.and_then(|stream| stream.codec_name.clone())
        } else {
            None
        },
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

fn media_kind_from_path(path: &Path) -> Option<MediaKind> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tif" | "tiff" => Some(MediaKind::Image),
        _ => None,
    }
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
    disposition: Option<FfprobeDisposition>,
}

impl FfprobeStream {
    fn is_attached_picture(&self) -> bool {
        self.disposition
            .as_ref()
            .and_then(|disposition| disposition.attached_pic)
            .unwrap_or_default()
            != 0
    }
}

#[derive(Debug, Deserialize)]
struct FfprobeDisposition {
    attached_pic: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    format_name: Option<String>,
    duration: Option<String>,
    bit_rate: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio_stream(duration: Option<&str>) -> FfprobeStream {
        FfprobeStream {
            index: Some(0),
            codec_type: Some("audio".to_string()),
            codec_name: Some("mp3".to_string()),
            width: None,
            height: None,
            channels: Some(2),
            sample_rate: Some("48000".to_string()),
            duration: duration.map(str::to_string),
            disposition: None,
        }
    }

    fn attached_picture_stream() -> FfprobeStream {
        FfprobeStream {
            index: Some(1),
            codec_type: Some("video".to_string()),
            codec_name: Some("mjpeg".to_string()),
            width: Some(800),
            height: Some(800),
            channels: None,
            sample_rate: None,
            duration: None,
            disposition: Some(FfprobeDisposition {
                attached_pic: Some(1),
            }),
        }
    }

    #[test]
    fn audio_stream_duration_is_used_when_format_duration_is_missing() {
        let metadata = metadata_from_response(
            FfprobeResponse {
                streams: vec![audio_stream(Some("12.5"))],
                format: Some(FfprobeFormat {
                    format_name: Some("mp3".to_string()),
                    duration: None,
                    bit_rate: Some("128000".to_string()),
                }),
            },
            1000,
            None,
        );

        assert_eq!(metadata.duration_seconds, Some(12.5));
        assert_eq!(metadata.media_kind, MediaKind::Audio);
    }

    #[test]
    fn attached_album_art_is_not_treated_as_video() {
        let metadata = metadata_from_response(
            FfprobeResponse {
                streams: vec![audio_stream(Some("20")), attached_picture_stream()],
                format: Some(FfprobeFormat {
                    format_name: Some("mp3".to_string()),
                    duration: Some("20".to_string()),
                    bit_rate: Some("128000".to_string()),
                }),
            },
            1000,
            None,
        );

        assert_eq!(metadata.video_codec, None);
        assert_eq!(metadata.width, None);
        assert_eq!(metadata.audio_codec, Some("mp3".to_string()));
        assert_eq!(metadata.media_kind, MediaKind::Audio);
    }

    #[test]
    fn image_path_is_treated_as_still_image() {
        let metadata = metadata_from_response(
            FfprobeResponse {
                streams: vec![FfprobeStream {
                    index: Some(0),
                    codec_type: Some("video".to_string()),
                    codec_name: Some("png".to_string()),
                    width: Some(1280),
                    height: Some(720),
                    channels: None,
                    sample_rate: None,
                    duration: Some("0.04".to_string()),
                    disposition: None,
                }],
                format: Some(FfprobeFormat {
                    format_name: Some("png_pipe".to_string()),
                    duration: Some("0.04".to_string()),
                    bit_rate: None,
                }),
            },
            2000,
            Some(MediaKind::Image),
        );

        assert_eq!(metadata.media_kind, MediaKind::Image);
        assert_eq!(metadata.duration_seconds, None);
        assert_eq!(metadata.image_codec, Some("png".to_string()));
        assert_eq!(metadata.video_codec, None);
        assert_eq!(metadata.width, Some(1280));
        assert_eq!(metadata.height, Some(720));
    }
}
