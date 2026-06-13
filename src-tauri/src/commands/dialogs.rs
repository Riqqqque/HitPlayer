const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "webm", "m4v", "flv", "wmv", "ts", "m2ts",
];

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "m4a", "aac", "flac", "ogg", "oga", "opus", "wma", "aiff", "aif", "mka",
];

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"];

fn is_supported_extension(extension: &str) -> bool {
    VIDEO_EXTENSIONS
        .iter()
        .chain(AUDIO_EXTENSIONS.iter())
        .chain(IMAGE_EXTENSIONS.iter())
        .any(|supported| supported.eq_ignore_ascii_case(extension))
}

#[tauri::command]
pub fn open_video_dialog() -> Result<Option<String>, String> {
    let supported_extensions = [VIDEO_EXTENSIONS, AUDIO_EXTENSIONS, IMAGE_EXTENSIONS].concat();
    let picked = rfd::FileDialog::new()
        .set_title("Open Media")
        .add_filter("Supported media files", &supported_extensions)
        .add_filter("Video files", VIDEO_EXTENSIONS)
        .add_filter("Audio files", AUDIO_EXTENSIONS)
        .add_filter("Photo files", IMAGE_EXTENSIONS)
        .pick_file();

    Ok(picked.map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn open_output_folder_dialog(current_path: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new().set_title("Choose Output Folder");

    if let Some(path) = current_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_dir())
    {
        dialog = dialog.set_directory(path);
    }

    Ok(dialog
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn get_launch_video_path() -> Result<Option<String>, String> {
    let picked = std::env::args_os()
        .skip(1)
        .map(std::path::PathBuf::from)
        .find(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(is_supported_extension)
        });

    Ok(picked.map(|path| path.to_string_lossy().to_string()))
}
