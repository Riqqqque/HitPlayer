use crate::models::{MediaKind, StreamInfo, VideoMetadata};
use crate::paths::binary_path;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

const FFPROBE_TIMEOUT: Duration = Duration::from_secs(60);

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
    let mut command = command_no_window(ffprobe);
    command.args(["-v", "error"]);
    if is_avi_like_path(input) {
        command.args(["-probesize", "20000000", "-analyzeduration", "20000000"]);
    }
    command
        .args([
            "-print_format",
            "json",
            "-show_entries",
            "format=format_name,duration,bit_rate:stream=index,codec_type,codec_name,width,height,channels,sample_rate,duration,pix_fmt,profile,field_order,sample_aspect_ratio,avg_frame_rate,r_frame_rate,nb_frames:stream_disposition=attached_pic",
        ])
        .arg(input);
    let output = output_with_timeout(command, FFPROBE_TIMEOUT)?;

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
            width: positive_u32(stream.width),
            height: positive_u32(stream.height),
            channels: positive_u32(stream.channels),
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

    let format = response.format.as_ref();
    let bitrate = format.and_then(|format| parse_optional_u64(format.bit_rate.as_deref()));
    let duration_seconds = if media_kind == MediaKind::Image {
        None
    } else {
        format
            .and_then(|format| parse_optional_f64(format.duration.as_deref()))
            .or_else(|| {
                visual_stream.and_then(|stream| parse_optional_f64(stream.duration.as_deref()))
            })
            .or_else(|| {
                audio_stream.and_then(|stream| parse_optional_f64(stream.duration.as_deref()))
            })
            .or_else(|| duration_from_frames(visual_stream.or(audio_stream)))
            .or_else(|| duration_from_bitrate(file_size_bytes, bitrate))
    };

    let is_image = media_kind == MediaKind::Image;

    VideoMetadata {
        duration_seconds,
        width: visual_stream.and_then(|stream| positive_u32(stream.width)),
        height: visual_stream.and_then(|stream| positive_u32(stream.height)),
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
        bitrate,
        file_size_bytes,
        streams,
        pix_fmt: if is_image {
            None
        } else {
            visual_stream.and_then(|stream| stream.pix_fmt.clone())
        },
        video_profile: if is_image {
            None
        } else {
            visual_stream.and_then(|stream| stream.profile.clone())
        },
        field_order: if is_image {
            None
        } else {
            visual_stream.and_then(|stream| stream.field_order.clone())
        },
        sample_aspect_ratio: if is_image {
            None
        } else {
            visual_stream.and_then(|stream| stream.sample_aspect_ratio.clone())
        },
    }
}

fn parse_optional_f64(value: Option<&str>) -> Option<f64> {
    value
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn parse_frame_rate(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("n/a") {
        return None;
    }

    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = numerator.parse::<f64>().ok()?;
        let denominator = denominator.parse::<f64>().ok()?;
        if numerator <= 0.0 || denominator <= 0.0 {
            return None;
        }
        let fps = numerator / denominator;
        return fps.is_finite().then_some(fps).filter(|fps| *fps > 0.0);
    }

    parse_optional_f64(Some(value))
}

fn duration_from_frames(stream: Option<&FfprobeStream>) -> Option<f64> {
    let stream = stream?;
    let frames = parse_optional_f64(stream.nb_frames.as_deref())?;
    let fps = parse_frame_rate(stream.avg_frame_rate.as_deref())
        .or_else(|| parse_frame_rate(stream.r_frame_rate.as_deref()))?;
    let duration = frames / fps;
    duration
        .is_finite()
        .then_some(duration)
        .filter(|duration| *duration > 0.0)
}

fn duration_from_bitrate(file_size_bytes: u64, bitrate: Option<u64>) -> Option<f64> {
    let bitrate = bitrate?;
    if file_size_bytes == 0 || bitrate == 0 {
        return None;
    }

    let duration = (file_size_bytes as f64 * 8.0) / bitrate as f64;
    duration
        .is_finite()
        .then_some(duration)
        .filter(|duration| *duration > 0.05)
}

pub(crate) fn is_avi_like_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "avi" | "divx" | "xvid"
            )
        })
}

fn parse_optional_u64(value: Option<&str>) -> Option<u64> {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn positive_u32(value: Option<u32>) -> Option<u32> {
    value.filter(|value| *value > 0)
}

fn command_no_window(program: PathBuf) -> Command {
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

fn output_with_timeout(mut command: Command, timeout: Duration) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not read media metadata: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not read media metadata.".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Could not read media metadata.".to_string())?;
    let stdout_thread = thread::spawn(move || read_pipe(stdout));
    let stderr_thread = thread::spawn(move || read_pipe(stderr));
    let started = Instant::now();

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Err(
                    "Could not read media metadata: FFprobe timed out while reading the file."
                        .to_string(),
                );
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Err(format!("Could not read media metadata: {error}"));
            }
        }
    };

    let stdout = stdout_thread
        .join()
        .map_err(|_| "Could not read media metadata.".to_string())?
        .map_err(|error| format!("Could not read media metadata: {error}"))?;
    let stderr = stderr_thread
        .join()
        .map_err(|_| "Could not read media metadata.".to_string())?
        .map_err(|error| format!("Could not read media metadata: {error}"))?;

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_pipe(mut pipe: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes)?;
    Ok(bytes)
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

#[derive(Debug, Default, Deserialize)]
struct FfprobeStream {
    index: Option<u32>,
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    channels: Option<u32>,
    sample_rate: Option<String>,
    duration: Option<String>,
    pix_fmt: Option<String>,
    profile: Option<String>,
    field_order: Option<String>,
    sample_aspect_ratio: Option<String>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    nb_frames: Option<String>,
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
    use std::path::Path;

    fn audio_stream(duration: Option<&str>) -> FfprobeStream {
        FfprobeStream {
            index: Some(0),
            codec_type: Some("audio".to_string()),
            codec_name: Some("mp3".to_string()),
            channels: Some(2),
            sample_rate: Some("48000".to_string()),
            duration: duration.map(str::to_string),
            ..Default::default()
        }
    }

    fn attached_picture_stream() -> FfprobeStream {
        FfprobeStream {
            index: Some(1),
            codec_type: Some("video".to_string()),
            codec_name: Some("mjpeg".to_string()),
            width: Some(800),
            height: Some(800),
            disposition: Some(FfprobeDisposition {
                attached_pic: Some(1),
            }),
            ..Default::default()
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
                    duration: Some("0.04".to_string()),
                    ..Default::default()
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

    #[test]
    fn invalid_numeric_metadata_is_ignored() {
        assert_eq!(parse_optional_f64(Some("-2")), None);
        assert_eq!(parse_optional_f64(Some("NaN")), None);
        assert_eq!(parse_optional_u64(Some("0")), None);
        assert_eq!(positive_u32(Some(0)), None);
    }

    #[test]
    fn avi_duration_falls_back_to_frame_count() {
        let metadata = metadata_from_response(
            FfprobeResponse {
                streams: vec![FfprobeStream {
                    index: Some(0),
                    codec_type: Some("video".to_string()),
                    codec_name: Some("mpeg4".to_string()),
                    width: Some(720),
                    height: Some(480),
                    pix_fmt: Some("yuv420p".to_string()),
                    avg_frame_rate: Some("30000/1001".to_string()),
                    nb_frames: Some("3000".to_string()),
                    ..Default::default()
                }],
                format: Some(FfprobeFormat {
                    format_name: Some("avi".to_string()),
                    duration: None,
                    bit_rate: Some("1500000".to_string()),
                }),
            },
            20 * 1024 * 1024,
            None,
        );

        let duration = metadata.duration_seconds.expect("duration");
        assert!((duration - 100.1).abs() < 0.1);
        assert_eq!(metadata.pix_fmt.as_deref(), Some("yuv420p"));
        assert_eq!(metadata.container.as_deref(), Some("avi"));
    }

    #[test]
    fn missing_duration_can_use_bitrate_and_file_size() {
        let metadata = metadata_from_response(
            FfprobeResponse {
                streams: vec![FfprobeStream {
                    index: Some(0),
                    codec_type: Some("video".to_string()),
                    codec_name: Some("mpeg4".to_string()),
                    width: Some(640),
                    height: Some(480),
                    ..Default::default()
                }],
                format: Some(FfprobeFormat {
                    format_name: Some("avi".to_string()),
                    duration: None,
                    bit_rate: Some("1000000".to_string()),
                }),
            },
            12_500_000,
            None,
        );

        assert_eq!(metadata.duration_seconds, Some(100.0));
    }

    #[test]
    fn avi_extensions_use_a_deeper_probe() {
        assert!(is_avi_like_path(Path::new(r"C:\Videos\clip.AVI")));
        assert!(is_avi_like_path(Path::new("movie.divx")));
        assert!(!is_avi_like_path(Path::new("clip.mp4")));
    }
}
