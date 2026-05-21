use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const VIDEO_EXTENSIONS: &[&str] = &[
    ".mp4", ".mkv", ".mov", ".avi", ".webm", ".m4v", ".flv", ".wmv", ".ts", ".m2ts",
];

#[tauri::command]
pub fn register_default_player() -> Result<(), String> {
    register_default_player_internal()
}

#[tauri::command]
pub fn open_default_player_settings() -> Result<(), String> {
    register_default_player_internal()?;
    open_system_uri("ms-settings:defaultapps?registeredAppUser=HitPlayer")
        .or_else(|_| open_system_uri("ms-settings:defaultapps"))
        .map_err(|error| format!("Could not open Windows Default Apps settings: {error}"))
}

#[tauri::command]
pub fn open_output_folder(path: String) -> Result<(), String> {
    let target = PathBuf::from(path);
    let folder = if target.is_dir() {
        target
    } else {
        target
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "Output path is invalid.".to_string())?
    };

    if !folder.is_dir() {
        return Err("Output path is invalid.".to_string());
    }

    command_no_window("explorer.exe")
        .arg(folder)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not open output folder: {error}"))?;

    Ok(())
}

#[tauri::command]
pub fn reveal_output_file(path: String) -> Result<(), String> {
    let target = PathBuf::from(path);
    if !target.is_file() {
        return open_output_folder(target.to_string_lossy().to_string());
    }

    let select_arg = format!("/select,{}", target.to_string_lossy());
    command_no_window("explorer.exe")
        .arg(select_arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not reveal output file: {error}"))?;

    Ok(())
}

#[cfg(windows)]
fn register_default_player_internal() -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let exe_path = std::env::current_exe()
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    let exe_name = exe_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Could not register HitPlayer with Windows.".to_string())?;
    let exe_display = exe_path.to_string_lossy().to_string();
    let command = format!("\"{exe_display}\" \"%1\"");
    let icon = format!("\"{exe_display}\",0");

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (registered_apps, _) = hkcu
        .create_subkey("Software\\RegisteredApplications")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    registered_apps
        .set_value("HitPlayer", &"Software\\Rique\\HitPlayer\\Capabilities")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;

    let (capabilities, _) = hkcu
        .create_subkey("Software\\Rique\\HitPlayer\\Capabilities")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    capabilities
        .set_value("ApplicationName", &"HitPlayer")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    capabilities
        .set_value(
            "ApplicationDescription",
            &"HitPlayer media player and video utility",
        )
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    capabilities
        .set_value("ApplicationIcon", &icon)
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;

    let (file_associations, _) = capabilities
        .create_subkey("FileAssociations")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    for extension in VIDEO_EXTENSIONS {
        file_associations
            .set_value(*extension, &"HitPlayer.Video")
            .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    }

    let (prog_id, _) = hkcu
        .create_subkey("Software\\Classes\\HitPlayer.Video")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    prog_id
        .set_value("", &"HitPlayer Video")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    prog_id
        .set_value("FriendlyTypeName", &"HitPlayer Video")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;

    let (default_icon, _) = prog_id
        .create_subkey("DefaultIcon")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    default_icon
        .set_value("", &icon)
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;

    let (open_command, _) = prog_id
        .create_subkey("shell\\open\\command")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    open_command
        .set_value("", &command)
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;

    let application_key = format!("Software\\Classes\\Applications\\{exe_name}");
    let (application, _) = hkcu
        .create_subkey(&application_key)
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    application
        .set_value("FriendlyAppName", &"HitPlayer")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;

    let (application_command, _) = application
        .create_subkey("shell\\open\\command")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    application_command
        .set_value("", &command)
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;

    let (supported_types, _) = application
        .create_subkey("SupportedTypes")
        .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    for extension in VIDEO_EXTENSIONS {
        supported_types
            .set_value(*extension, &"")
            .map_err(|error| format!("Could not register HitPlayer with Windows: {error}"))?;
    }

    Ok(())
}

#[cfg(not(windows))]
fn register_default_player_internal() -> Result<(), String> {
    Err("Default player registration is only supported on Windows.".to_string())
}

#[cfg(windows)]
fn open_system_uri(uri: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation = wide_null("open");
    let target = wide_null(uri);
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    if result as isize <= 32 {
        Err("Windows could not open Default Apps settings.".to_string())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(value).encode_wide().chain([0]).collect()
}

#[cfg(not(windows))]
fn open_system_uri(_uri: &str) -> Result<(), String> {
    Err("Default Apps settings are only available on Windows.".to_string())
}

fn command_no_window(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
