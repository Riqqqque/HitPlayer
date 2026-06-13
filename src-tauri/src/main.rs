#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod paths;

use commands::ffmpeg::{
    compress_photo, compress_video, convert_to_mp4, detect_encoders, fast_trim, precise_trim,
    prepare_preview,
};
use commands::ffprobe::probe_video;
use commands::jobs::{cancel_job, JobManager};
use commands::system::{open_output_folder, reveal_output_file};
use tauri::Manager;

fn main() {
    let result = tauri::Builder::default()
        .manage(JobManager::default())
        .setup(|app| {
            if let Some(icon) = app.default_window_icon().cloned() {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_icon(icon);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::dialogs::open_video_dialog,
            commands::dialogs::open_output_folder_dialog,
            commands::dialogs::get_launch_video_path,
            probe_video,
            detect_encoders,
            prepare_preview,
            fast_trim,
            precise_trim,
            compress_photo,
            compress_video,
            convert_to_mp4,
            cancel_job,
            commands::system::open_default_player_settings,
            commands::system::register_default_player,
            open_output_folder,
            reveal_output_file
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("HitPlayer failed to start: {error}");
    }
}
