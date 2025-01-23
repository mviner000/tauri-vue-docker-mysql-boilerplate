// src/lib.rs

mod windows_setup;
mod ubuntu_setup;
mod db;
mod models;

use std::env;
use log::{info, debug};
use anyhow::Result;
use tauri::{State, Manager};
use mysql_async::Pool;

use crate::windows_setup::WindowsSystemSetup;
use crate::ubuntu_setup::UbuntuSystemSetup;
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
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // OS setup
                let os_result = match detect_os() {
                    OperatingSystem::Windows => {
                        WindowsSystemSetup::setup_windows_system(&app_handle).await
                    },
                    OperatingSystem::Linux => {
                        UbuntuSystemSetup::setup_ubuntu_system(&app_handle).await
                    },
                    OperatingSystem::MacOS => Ok(()),
                    OperatingSystem::Unknown => {
                        Err(anyhow::anyhow!("Unsupported operating system"))
                    }
                };

                // Database pool setup
                if let Ok(_) = os_result {
                    match db::create_database_pool().await {
                        Ok(pool) => {
                            app_handle.manage(pool);
                            println!("Database pool created successfully");
                        },
                        Err(e) => eprintln!("Database pool creation failed: {}", e),
                    }
                } else {
                    if let Err(e) = os_result {
                        eprintln!("OS setup failed: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_note,
            get_all_notes,
            get_note_by_id,
            update_note,
            delete_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
