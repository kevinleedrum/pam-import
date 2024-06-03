use async_recursion::async_recursion;
use chrono::{DateTime, Local, NaiveDateTime};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{emit_log, emit_progress, ProgressEventPayload};

#[async_recursion]
pub async fn scan(
    app_handle: tauri::AppHandle,
    path: String,
    path_queue: &mut Vec<PathBuf>,
    stop_flag: Arc<AtomicBool>,
) -> Result<Vec<PathBuf>, io::Error> {
    if stop_flag.load(Ordering::Relaxed) {
        return Ok(vec![]);
    }

    emit_progress(
        app_handle.clone(),
        ProgressEventPayload {
            status: Some(format!("Scanning {}...", path)),
            progress_value: None,
            progress_max: path_queue.len() as u32,
        },
    );

    let entries = fs::read_dir(&path)?;
    for entry in entries {
        if stop_flag.load(Ordering::Relaxed) {
            return Ok(vec![]);
        }
        let entry = entry?;
        let entry_path = entry.path();

        if stop_flag.load(Ordering::Relaxed) {
            return Ok(vec![]);
        }

        if entry_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with(".")
        {
            continue; // Skip hidden files
        }

        if entry_path.is_dir() {
            scan(
                app_handle.clone(),
                entry_path.to_str().unwrap().to_string(),
                path_queue,
                stop_flag.clone(),
            )
            .await?;
        } else if is_media_extension(entry_path.extension()) {
            path_queue.push(entry_path.clone());
            emit_log(
                app_handle.clone(),
                format!("Queued {}", entry_path.display()),
            );
            emit_progress(
                app_handle.clone(),
                ProgressEventPayload {
                    status: None,
                    progress_value: None,
                    progress_max: path_queue.len() as u32,
                },
            );
        }
    }
    Ok(path_queue.clone())
}

pub async fn import_files(
    app_handle: tauri::AppHandle,
    path_queue: &[PathBuf],
    destination: String,
    filename_template: &str,
    stop_flag: Arc<AtomicBool>,
) -> (u32, u32) {
    let mut imported_count = 0;
    let mut skipped_count = 0;

    for (i, path) in path_queue.iter().enumerate() {
        if stop_flag.load(Ordering::Relaxed) {
            return (imported_count, skipped_count);
        }

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            continue;
        }

        let ext = match path.extension().and_then(OsStr::to_str) {
            Some(ext) => ext,
            None => continue,
        };

        let datetime = get_exif_datetime(path).or_else(|| get_file_datetime(path));
        let datetime = match datetime {
            Some(dt) => dt,
            None => continue,
        };

        let new_filename = format!("{}.{}", datetime.format(filename_template), ext);
        let mut new_path = Path::new(&destination).join(&new_filename);

        if new_path.exists() && same_file(path, &new_path) {
            emit_log(
                app_handle.clone(),
                format!("Skipping {}", new_path.display()),
            );
            skipped_count += 1;
            continue;
        }

        let mut dup_counter = 1;
        while new_path.exists() {
            let new_filename_with_counter = format!("{}_{}.{}", new_filename, dup_counter, ext);
            new_path = Path::new(&destination).join(new_filename_with_counter);
            dup_counter += 1;
        }

        emit_progress(
            app_handle.clone(),
            ProgressEventPayload {
                status: Some(format!(
                    "Copying {} to {}...",
                    path.display(),
                    new_path.display()
                )),
                progress_value: Some(i as u32),
                progress_max: path_queue.len() as u32,
            },
        );

        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        if fs::copy(path, &new_path).is_ok() {
            imported_count += 1;
            emit_log(
                app_handle.clone(),
                format!("Copied {} to {}", path.display(), new_path.display()),
            );
        }
    }

    emit_progress(
        app_handle,
        ProgressEventPayload {
            status: Some("Finished".to_string()),
            progress_value: Some(path_queue.len() as u32),
            progress_max: path_queue.len() as u32,
        },
    );

    (imported_count, skipped_count)
}

fn get_exif_datetime(path: &Path) -> Option<NaiveDateTime> {
    let file = File::open(path).ok()?;
    let mut bufreader = BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader).ok()?;
    let field = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)?;
    let value = field.display_value().to_string();
    NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S").ok()
}

fn get_file_datetime(path: &Path) -> Option<NaiveDateTime> {
    let metadata = fs::metadata(path).ok()?;
    let created = metadata.created().ok()?;
    let modified = metadata.modified().ok()?;
    let older_time = std::cmp::min(created, modified);
    let duration = older_time.duration_since(std::time::UNIX_EPOCH).ok()?;
    let seconds = duration.as_secs() as i64;
    let nanos = duration.subsec_nanos() as u32;
    let datetime = DateTime::from_timestamp(seconds, nanos)
        .unwrap()
        .with_timezone(&Local)
        .naive_local();
    Some(datetime)
}

#[tauri::command]
pub fn get_filename_template_example(filename_template: String) -> String {
    if filename_template.is_empty() {
        return "".to_string();
    }
    let datetime = Local::now().naive_local();
    let result = panic::catch_unwind(|| datetime.format(&filename_template).to_string());
    match result {
        Ok(result) => format!("{}.jpg", result),
        Err(_) => "".to_string(),
    }
}

fn is_media_extension(ext: Option<&OsStr>) -> bool {
    const VALID_EXTENSIONS: &[&str] = &[
        "jpg", "jpeg", "png", "tif", "tiff", "mp4", "mov", "avi", "mpg", "wmv",
    ];
    match ext {
        Some(ext) => {
            let ext = ext.to_str().unwrap().to_lowercase();
            VALID_EXTENSIONS.contains(&ext.as_str())
        }
        None => false,
    }
}

fn same_file(path1: &Path, path2: &Path) -> bool {
    fs::metadata(path1)
        .and_then(|meta1| fs::metadata(path2).map(|meta2| meta1.len() == meta2.len()))
        .unwrap_or(false)
}
