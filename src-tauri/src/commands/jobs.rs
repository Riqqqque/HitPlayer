use crate::models::{JobPhase, JobProgress, JobResult};
use crate::paths::binary_path;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, State};

const MAX_FFMPEG_LOG_BYTES: usize = 512 * 1024;

#[derive(Clone, Default)]
pub struct JobManager {
    current: Arc<Mutex<Option<RunningJob>>>,
}

#[derive(Clone)]
struct RunningJob {
    job_id: String,
    pid: Option<u32>,
    canceled: Arc<AtomicBool>,
}

impl JobManager {
    fn reserve(&self, job_id: String, canceled: Arc<AtomicBool>) -> Result<(), String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "A job is already running.".to_string())?;
        if current.is_some() {
            return Err("A job is already running.".to_string());
        }
        *current = Some(RunningJob {
            job_id,
            pid: None,
            canceled,
        });
        Ok(())
    }

    fn set_pid(&self, job_id: &str, pid: u32) {
        if let Ok(mut current) = self.current.lock() {
            if let Some(job) = current.as_mut().filter(|job| job.job_id == job_id) {
                job.pid = Some(pid);
            }
        }
    }

    fn clear(&self, job_id: &str) {
        if let Ok(mut current) = self.current.lock() {
            if current.as_ref().is_some_and(|job| job.job_id == job_id) {
                *current = None;
            }
        }
    }

    fn cancel_current(&self) -> Result<(), String> {
        let current = self
            .current
            .lock()
            .map_err(|_| "Could not cancel the current job.".to_string())?
            .clone();

        if let Some(job) = current {
            job.canceled.store(true, Ordering::SeqCst);
            if let Some(pid) = job.pid {
                kill_process_tree(pid)?;
            }
        }

        Ok(())
    }
}

#[tauri::command]
pub fn cancel_job(state: State<'_, JobManager>) -> Result<(), String> {
    state.cancel_current()
}

pub struct FfmpegJob {
    pub name: String,
    pub args: Vec<String>,
    pub output_path: PathBuf,
    pub total_duration: Option<f64>,
}

pub fn run_ffmpeg_job(
    app: AppHandle,
    manager: JobManager,
    job: FfmpegJob,
) -> Result<JobResult, String> {
    let ffmpeg = binary_path(&app, "ffmpeg")?;
    let job_id = uuid::Uuid::new_v4().to_string();
    let canceled = Arc::new(AtomicBool::new(false));

    manager.reserve(job_id.clone(), canceled.clone())?;
    emit_progress(
        &app,
        JobProgress {
            job_id: job_id.clone(),
            phase: JobPhase::Starting,
            percent: 0.0,
            out_time_seconds: None,
            speed: None,
            fps: None,
            message: Some(format!("Starting {}.", job.name)),
        },
    );

    let mut command = command_no_window(ffmpeg);
    command
        .args(&job.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            manager.clear(&job_id);
            return Err(format!("Could not start FFmpeg: {error}"));
        }
    };

    let pid = child.id();
    manager.set_pid(&job_id, pid);

    if canceled.load(Ordering::SeqCst) {
        let _ = kill_process_tree(pid);
    }

    let log = Arc::new(Mutex::new(String::new()));
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_thread = stdout.map(|stdout| {
        let app = app.clone();
        let log = log.clone();
        let job_id = job_id.clone();
        let total_duration = job.total_duration;
        thread::spawn(move || read_progress(stdout, app, log, job_id, total_duration))
    });

    let stderr_thread = stderr.map(|stderr| {
        let log = log.clone();
        thread::spawn(move || read_log(stderr, log))
    });

    let status = child.wait().map_err(|error| {
        manager.clear(&job_id);
        format!("FFmpeg job failed: {error}")
    })?;

    if let Some(thread) = stdout_thread {
        let _ = thread.join();
    }
    if let Some(thread) = stderr_thread {
        let _ = thread.join();
    }

    let log_text = log.lock().map(|log| log.clone()).unwrap_or_default();
    let was_canceled = canceled.load(Ordering::SeqCst);
    manager.clear(&job_id);

    if was_canceled {
        remove_incomplete_output(&job.output_path);
        emit_progress(
            &app,
            JobProgress {
                job_id,
                phase: JobPhase::Canceled,
                percent: 0.0,
                out_time_seconds: None,
                speed: None,
                fps: None,
                message: Some("Canceled.".to_string()),
            },
        );
        return Ok(JobResult {
            success: false,
            output_path: job.output_path.to_string_lossy().to_string(),
            duration_seconds: job.total_duration,
            log: log_text,
            canceled: true,
            error: Some("Canceled.".to_string()),
        });
    }

    if status.success() && output_file_ready(&job.output_path) {
        emit_progress(
            &app,
            JobProgress {
                job_id,
                phase: JobPhase::Finished,
                percent: 100.0,
                out_time_seconds: job.total_duration,
                speed: None,
                fps: None,
                message: Some("Finished.".to_string()),
            },
        );
        return Ok(JobResult {
            success: true,
            output_path: job.output_path.to_string_lossy().to_string(),
            duration_seconds: job.total_duration,
            log: log_text,
            canceled: false,
            error: None,
        });
    }

    remove_incomplete_output(&job.output_path);
    emit_progress(
        &app,
        JobProgress {
            job_id,
            phase: JobPhase::Failed,
            percent: 0.0,
            out_time_seconds: None,
            speed: None,
            fps: None,
            message: Some("Export failed. Open details for FFmpeg log.".to_string()),
        },
    );
    Ok(JobResult {
        success: false,
        output_path: job.output_path.to_string_lossy().to_string(),
        duration_seconds: job.total_duration,
        log: log_text,
        canceled: false,
        error: Some("Export failed. Open details for FFmpeg log.".to_string()),
    })
}

fn read_progress<R: std::io::Read>(
    stdout: R,
    app: AppHandle,
    log: Arc<Mutex<String>>,
    job_id: String,
    total_duration: Option<f64>,
) {
    let mut out_time_seconds = None;
    let mut speed = None;
    let mut fps = None;

    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        append_log(&log, &format!("[progress] {line}\n"));

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        match key {
            "out_time_ms" | "out_time_us" => {
                out_time_seconds = value.parse::<f64>().ok().map(|value| value / 1_000_000.0);
            }
            "out_time" => {
                out_time_seconds = parse_ffmpeg_time(value);
            }
            "speed" => {
                speed = Some(value.trim().to_string());
            }
            "fps" => {
                fps = value.parse::<f64>().ok();
            }
            "progress" => {
                let phase = if value == "end" {
                    JobPhase::Finished
                } else {
                    JobPhase::Running
                };
                let percent = progress_percent(
                    total_duration,
                    out_time_seconds,
                    matches!(phase, JobPhase::Finished),
                );
                emit_progress(
                    &app,
                    JobProgress {
                        job_id: job_id.clone(),
                        phase,
                        percent,
                        out_time_seconds,
                        speed: speed.clone(),
                        fps,
                        message: Some("Running FFmpeg.".to_string()),
                    },
                );
            }
            _ => {}
        }
    }
}

fn read_log<R: std::io::Read>(stderr: R, log: Arc<Mutex<String>>) {
    for line in BufReader::new(stderr).lines().map_while(Result::ok) {
        append_log(&log, &format!("{line}\n"));
    }
}

fn append_log(log: &Arc<Mutex<String>>, line: &str) {
    if let Ok(mut log) = log.lock() {
        log.push_str(line);
        if log.len() > MAX_FFMPEG_LOG_BYTES {
            let marker = "[older FFmpeg log trimmed]\n";
            let keep_bytes = MAX_FFMPEG_LOG_BYTES.saturating_sub(marker.len());
            let mut drain_to = log.len().saturating_sub(keep_bytes);
            while drain_to < log.len() && !log.is_char_boundary(drain_to) {
                drain_to += 1;
            }
            log.drain(..drain_to);
            log.insert_str(0, marker);
        }
    }
}

fn progress_percent(
    total_duration: Option<f64>,
    out_time_seconds: Option<f64>,
    finished: bool,
) -> f64 {
    if finished {
        return 100.0;
    }

    match (total_duration, out_time_seconds) {
        (Some(total), Some(out_time)) if total > 0.0 => {
            ((out_time / total) * 100.0).clamp(0.0, 99.0)
        }
        _ => -1.0,
    }
}

fn parse_ffmpeg_time(value: &str) -> Option<f64> {
    let mut parts = value.split(':');
    let hours = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts.next()?.parse::<f64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    Some((hours * 3600.0) + (minutes * 60.0) + seconds)
}

fn emit_progress(app: &AppHandle, progress: JobProgress) {
    let _ = app.emit("ffmpeg-progress", progress);
}

fn command_no_window(program: PathBuf) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn output_file_ready(path: &Path) -> bool {
    path.is_file()
        && path
            .metadata()
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
}

fn remove_incomplete_output(path: &Path) {
    if path.is_file() {
        let _ = fs::remove_file(path);
    }
}

fn kill_process_tree(pid: u32) -> Result<(), String> {
    #[cfg(windows)]
    {
        let status = command_no_window(PathBuf::from("taskkill.exe"))
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|error| format!("Could not cancel the current job: {error}"))?;

        if !status.success() {
            return Err("Could not cancel the current job.".to_string());
        }
    }

    #[cfg(not(windows))]
    {
        let _ = pid;
        return Err("Cancel is only supported on Windows in this build.".to_string());
    }

    Ok(())
}

pub fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hitplayer-job-test-{}-{name}",
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn output_file_ready_rejects_missing_and_empty_files() {
        let missing = temp_file_path("missing.mp4");
        assert!(!output_file_ready(&missing));

        let empty = temp_file_path("empty.mp4");
        fs::write(&empty, []).unwrap();
        assert!(!output_file_ready(&empty));

        let _ = fs::remove_file(empty);
    }

    #[test]
    fn remove_incomplete_output_deletes_partial_file() {
        let partial = temp_file_path("partial.mp4");
        fs::write(&partial, b"partial").unwrap();

        remove_incomplete_output(&partial);

        assert!(!partial.exists());
    }
}
