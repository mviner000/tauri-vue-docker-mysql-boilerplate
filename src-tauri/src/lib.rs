// src/lib.rs

#[cfg(target_os = "windows")]
mod windows_setup;
#[cfg(target_os = "linux")]
mod ubuntu_setup;
mod db;
mod models;

use std::env;
use log::{info, debug, error};
use anyhow::Result;
use tauri::{State, Manager};
use tauri::Emitter;
use mysql_async::Pool;

#[cfg(target_os = "linux")]
use crate::ubuntu_setup::{InstallationStage, UbuntuSystemSetup};
use crate::db::notes::NoteRepository;
use crate::models::Note;

#[tauri::command]
async fn create_note(
    pool: State<'_, Pool>,
    note: Note
) -> Result<Note, String> {
    let repo = NoteRepository::new(pool.inner().clone());
    repo.create_note(&note)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_all_notes(
    pool: State<'_, Pool>
) -> Result<Vec<Note>, String> {
    let repo = NoteRepository::new(pool.inner().clone());
    repo.get_all_notes()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_note_by_id(
    pool: State<'_, Pool>, 
    id: i64
) -> Result<Note, String> {
    let repo = NoteRepository::new(pool.inner().clone());
    repo.get_note_by_id(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_note(
    pool: State<'_, Pool>, 
    id: i64, 
    note: Note
) -> Result<Note, String> {
    let repo = NoteRepository::new(pool.inner().clone());
    repo.update_note(id, &note)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_note(
    pool: State<'_, Pool>, 
    id: i64
) -> Result<bool, String> {
    let repo = NoteRepository::new(pool.inner().clone());
    repo.delete_note(id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
#[tauri::command]
async fn start_system_setup(app: tauri::AppHandle) -> Result<(), String> {
    app.emit("installation-stage", InstallationStage::NotStarted)
        .map_err(|e| e.to_string())?;
        
    UbuntuSystemSetup::setup_ubuntu_system_with_events(&app)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
#[tauri::command]
async fn start_system_setup(app: tauri::AppHandle) -> Result<(), String> {
    app.emit("installation-stage", "NotStarted")
        .map_err(|e| e.to_string())?;

    windows_setup::WindowsSystemSetup::setup_windows_system(&app)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, PartialEq)]
pub enum OperatingSystem {
    Windows,
    MacOS,
    Linux,
    Unknown,
}

pub fn detect_os() -> OperatingSystem {
    match env::consts::OS {
        "windows" => OperatingSystem::Windows,
        "macos" => OperatingSystem::MacOS,
        "linux" => OperatingSystem::Linux,
        _ => OperatingSystem::Unknown,
    }
}

#[tauri::command]
fn get_os_details() -> String {
    format!(
        "OS: {}, Family: {}, Architecture: {}",
        env::consts::OS,
        env::consts::FAMILY,
        env::consts::ARCH
    )
}

#[tauri::command]
fn is_windows() -> bool {
    detect_os() == OperatingSystem::Windows
}

#[tauri::command]
fn is_linux() -> bool {
    detect_os() == OperatingSystem::Linux
}

#[tauri::command]
fn is_docker_installed() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("which")
            .arg("docker")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("where")
            .arg("docker")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[tauri::command]
fn get_os_type() -> String {
    match detect_os() {
        OperatingSystem::Windows => "Windows".to_string(),
        OperatingSystem::MacOS => "MacOS".to_string(),
        OperatingSystem::Linux => "Linux".to_string(),
        OperatingSystem::Unknown => "Unknown".to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let current_os = detect_os();
    
    info!("Starting Tauri application");
    debug!("Detected OS: {:?}", current_os);
    debug!("Full OS Details: {}", get_os_details());

    match current_os {
        OperatingSystem::Windows => debug!("Running on Windows"),
        OperatingSystem::MacOS => debug!("Running on macOS"),
        OperatingSystem::Linux => debug!("Running on Linux"),
        OperatingSystem::Unknown => debug!("Running on unknown OS"),
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_note,
            get_all_notes,
            get_note_by_id,
            update_note,
            delete_note,
            start_system_setup,
            is_docker_installed,
            get_os_type,
            get_os_details,
            is_windows 
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}