// src/lib.rs

mod windows_setup;
mod ubuntu_setup;
mod db;
mod models;

use std::env;
use log::{info, debug, error};
use anyhow::Result;
use tauri::{State, Manager};
use mysql_async::Pool;

use crate::ubuntu_setup::InstallationStage;
use crate::ubuntu_setup::UbuntuSystemSetup;
use crate::db::notes::NoteRepository;
use crate::models::Note;
use tauri::Emitter;

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

#[tauri::command]
async fn start_ubuntu_setup(app: tauri::AppHandle) -> Result<(), String> {
    // Add initial stage reset
    app.emit("installation-stage", InstallationStage::NotStarted)
        .map_err(|e| e.to_string())?;
        
    UbuntuSystemSetup::setup_ubuntu_system_with_events(&app)
        .await
        .map_err(|e| e.to_string())
}

/// Represents different operating systems
#[derive(Debug, PartialEq)]
pub enum OperatingSystem {
    Windows,
    MacOS,
    Linux,
    Unknown,
}

/// Detects the current operating system
pub fn detect_os() -> OperatingSystem {
    match env::consts::OS {
        "windows" => OperatingSystem::Windows,
        "macos" => OperatingSystem::MacOS,
        "linux" => OperatingSystem::Linux,
        _ => OperatingSystem::Unknown,
    }
}

/// Additional OS detection utilities
pub fn get_os_details() -> String {
    format!(
        "OS: {}, Family: {}, Architecture: {}",
        env::consts::OS,
        env::consts::FAMILY,
        env::consts::ARCH
    )
}

/// Check if the current OS is Windows
pub fn is_windows() -> bool {
    detect_os() == OperatingSystem::Windows
}

/// Check if the current OS is MacOS
pub fn is_macos() -> bool {
    detect_os() == OperatingSystem::MacOS
}

/// Check if the current OS is Linux
pub fn is_linux() -> bool {
    detect_os() == OperatingSystem::Linux
}

#[tauri::command]
async fn setup_ubuntu_system(app: tauri::AppHandle) -> Result<(), String> {
    UbuntuSystemSetup::setup_ubuntu_system_with_events(&app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn is_docker_installed() -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("which")
            .arg("docker")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
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

#[tauri::command]
async fn respond_to_sudo_password_request(
    app: tauri::AppHandle,
    request_id: String, 
    password: String
) -> Result<(), String> {
    // Validate inputs
    if request_id.is_empty() {
        return Err("Request ID cannot be empty".to_string());
    }

    // Use the request_id to create a unique event name
    let event_name = format!("sudo-password-response-{}", request_id);
    
    // Emit the password directly as a string
    app.emit(&event_name, password)
        .map_err(|e| e.to_string())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Enable logging
    env_logger::init();

    let current_os = detect_os();
    
    info!("Starting Tauri application");
    debug!("Detected OS: {:?}", current_os);
    debug!("Full OS Details: {}", get_os_details());

    // Example of OS-specific logic
    match current_os {
        OperatingSystem::Windows => debug!("Running on Windows"),
        OperatingSystem::MacOS => debug!("Running on macOS"),
        OperatingSystem::Linux => debug!("Running on Linux"),
        OperatingSystem::Unknown => debug!("Running on an unknown OS"),
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
            setup_ubuntu_system,
            is_docker_installed,
            get_os_type,
            start_ubuntu_setup,
            respond_to_sudo_password_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
