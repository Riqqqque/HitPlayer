use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    pub index: u32,
    pub codec_type: Option<String>,
    pub codec_name: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub channels: Option<u32>,
    pub sample_rate: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    pub duration_seconds: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub media_kind: MediaKind,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub image_codec: Option<String>,
    pub container: Option<String>,
    pub bitrate: Option<u64>,
    pub file_size_bytes: u64,
    pub streams: Vec<StreamInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Video,
    Audio,
    Image,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncoderSupport {
    pub has_libx264: bool,
    pub has_libx265: bool,
    pub has_libwebp: bool,
    pub has_h264_nvenc: bool,
    pub has_hevc_nvenc: bool,
    pub has_h264_amf: bool,
    pub has_h264_qsv: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub job_id: String,
    pub phase: JobPhase,
    pub percent: f64,
    pub out_time_seconds: Option<f64>,
    pub speed: Option<String>,
    pub fps: Option<f64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobPhase {
    Starting,
    Running,
    Finished,
    Failed,
    Canceled,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub success: bool,
    pub output_path: String,
    pub duration_seconds: Option<f64>,
    pub log: String,
    pub canceled: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub preview_path: String,
    pub used_cached_file: bool,
    pub method: String,
    pub log: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimOptions {
    pub input_path: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub output_path: Option<String>,
    pub output_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    pub input_path: String,
    pub preset: CompressionPreset,
    pub output_path: Option<String>,
    pub output_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOptions {
    pub input_path: String,
    pub output_path: Option<String>,
    pub output_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoCompressOptions {
    pub input_path: String,
    pub preset: PhotoCompressionPreset,
    pub format: PhotoCompressionFormat,
    pub output_path: Option<String>,
    pub output_directory: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionPreset {
    Balanced,
    Small,
    HighQuality,
    NvidiaFast,
}

impl CompressionPreset {
    pub fn suffix(&self) -> &'static str {
        match self {
            CompressionPreset::Balanced => "compressed_balanced",
            CompressionPreset::Small => "compressed_small",
            CompressionPreset::HighQuality => "compressed_high_quality",
            CompressionPreset::NvidiaFast => "compressed_nvidia_fast",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotoCompressionPreset {
    Balanced,
    Small,
    HighQuality,
}

impl PhotoCompressionPreset {
    pub fn suffix(&self) -> &'static str {
        match self {
            PhotoCompressionPreset::Balanced => "photo_compressed_balanced",
            PhotoCompressionPreset::Small => "photo_compressed_small",
            PhotoCompressionPreset::HighQuality => "photo_compressed_high_quality",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotoCompressionFormat {
    Jpeg,
    Webp,
}

impl PhotoCompressionFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            PhotoCompressionFormat::Jpeg => "jpg",
            PhotoCompressionFormat::Webp => "webp",
        }
    }
}
