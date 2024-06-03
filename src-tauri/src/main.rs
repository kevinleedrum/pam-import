// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod import;

use crate::import::get_filename_template_example;
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use sysinfo::Disks;
use tauri::Manager;
use tauri::State;

struct AppState {
    stop_flag: Arc<AtomicBool>,
}

#[derive(Clone, Serialize)]
pub struct ProgressEventPayload {
    pub status: Option<String>,
    pub progress_value: Option<u32>,
    pub progress_max: u32,
}

#[derive(Clone, Serialize)]
struct LogEventPayload {
    pub message: String,
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            stop_flag: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            get_default_source,
            get_filename_template_example,
            start,
            stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_default_source() -> Result<String, String> {
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        if disk.is_removable() {
            return Ok(disk.mount_point().display().to_string());
        }
    }
    disks
        .list()
        .last()
        .map(|disk| disk.mount_point().display().to_string())
        .ok_or_else(|| "No disk found".to_string())
}

#[tauri::command]
async fn start(
    app_handle: tauri::AppHandle,
    source: String,
    destination: String,
    filename_template: String,
    state: State<'_, AppState>,
) -> Result<(u32, u32), String> {
    state.stop_flag.store(false, Ordering::Relaxed);
    let mut path_queue = vec![];
    let found_files = import::scan(
        app_handle.clone(),
        source,
        &mut path_queue,
        state.stop_flag.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;

    if found_files.is_empty() {
        return Ok((0, 0));
    }

    let (imported_count, skipped_count) = import::import_files(
        app_handle,
        &found_files,
        destination,
        &filename_template,
        state.stop_flag.clone(),
    )
    .await;

    Ok((imported_count, skipped_count))
}

#[tauri::command]
async fn stop(state: State<'_, AppState>) -> Result<(), ()> {
    state.stop_flag.store(true, Ordering::Relaxed);
    Ok(())
}

pub fn emit_progress(app_handle: tauri::AppHandle, payload: ProgressEventPayload) {
    app_handle.emit_all("progress", payload).ok();
}

pub fn emit_log(app_handle: tauri::AppHandle, message: String) {
    app_handle.emit_all("log", LogEventPayload { message }).ok();
}
