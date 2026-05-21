const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "webm", "m4v", "flv", "wmv", "ts", "m2ts",
];

#[tauri::command]
pub fn open_video_dialog() -> Result<Option<String>, String> {
    let picked = rfd::FileDialog::new()
        .set_title("Open Video")
        .add_filter("Supported video files", SUPPORTED_EXTENSIONS)
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
                    .is_some_and(|extension| {
                        SUPPORTED_EXTENSIONS
                            .iter()
                            .any(|supported| supported.eq_ignore_ascii_case(extension))
                    })
        });

    Ok(picked.map(|path| path.to_string_lossy().to_string()))
}
