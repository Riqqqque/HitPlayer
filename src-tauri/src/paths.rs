use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const TARGET_TRIPLE: &str = env!("TARGET_TRIPLE");

pub fn binary_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let label = match name {
        "ffmpeg" => "FFmpeg",
        "ffprobe" => "FFprobe",
        _ => name,
    };

    for candidate in binary_candidates(app, name) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "{label} was not found. Place {name}.exe in the required app binary folder."
    ))
}

fn binary_candidates(app: &AppHandle, name: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.push(parent.to_path_buf());
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        roots.push(resource_dir);
    }

    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir.clone());
        roots.push(current_dir.join("src-tauri"));
    }

    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let names = [format!("{name}.exe"), format!("{name}-{TARGET_TRIPLE}.exe")];
    let mut candidates = Vec::new();

    for root in roots {
        for binary_name in &names {
            candidates.push(root.join(binary_name));
            candidates.push(root.join("binaries").join(binary_name));
        }
    }

    candidates
}

pub fn default_output_path(
    input: &Path,
    suffix: &str,
    requested: Option<&str>,
    default_extension: &str,
) -> Result<PathBuf, String> {
    let input_parent = input
        .parent()
        .ok_or_else(|| "Output path is invalid.".to_string())?;

    let requested_path = requested
        .map(|path| PathBuf::from(path.trim()))
        .filter(|path| !path.as_os_str().is_empty());
    let output = if let Some(path) = requested_path {
        if path.extension().is_none() {
            path.with_extension(default_extension)
        } else {
            path
        }
    } else {
        let stem = input
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.trim().is_empty())
            .ok_or_else(|| "Output path is invalid.".to_string())?;

        input_parent
            .join("HitPlayerExports")
            .join(format!("{stem}_{suffix}.{default_extension}"))
    };

    if is_same_path(input, &output) {
        return Err("Cannot overwrite input file.".to_string());
    }

    let unique = unique_output_path(output);

    if let Some(parent) = unique.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Output path is invalid: {error}"))?;
    }

    Ok(unique)
}

fn unique_output_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("output")
        .to_string();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("mp4");

    for index in 1..1000 {
        let candidate = parent.join(format!("{stem}_{index:03}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{stem}_{}.{}", uuid::Uuid::new_v4(), extension))
}

fn is_same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy()),
    }
}
