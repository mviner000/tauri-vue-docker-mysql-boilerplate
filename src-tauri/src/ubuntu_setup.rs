// src/ubuntu_setup.rs

use crate::{detect_os, OperatingSystem};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::time::Duration;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use tauri_plugin_shell::process::CommandEvent;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tauri_plugin_shell::ShellExt;
use tauri::Manager;
use tauri::Emitter;
use serde::{Serialize, Deserialize};
use tokio::fs;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tauri::Url;
use tauri::Listener;
use dotenv::dotenv;
use std::env;
use tokio::fs as async_fs;

const REQUIRED_TOOLS: [&str; 4] = ["lsb_release", "curl", "nc", "ss"];
const MAX_PORT_CHECK_ATTEMPTS: u32 = 5;

#[derive(Serialize, Deserialize, Clone)]
struct SudoPasswordRequest {
    request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationStage {
    NotStarted,
    CheckingDocker,
    DockerNotInstalled,
    DockerInstalling,
    DockerInstallFailed,
    DockerInstalled,
    PreparingMySQLContainer,
    StartingMySQLContainer,
    MySQLContainerStarted,
    MySQLSetupFailed,
    SetupComplete
}

pub struct UbuntuSystemSetup;

impl UbuntuSystemSetup {
    pub async fn setup_ubuntu_system_with_events(app: &tauri::AppHandle) -> Result<()> {
        println!("DEBUG: Entering setup_ubuntu_system_with_events");
    
        // Emit initial stage
        app.emit("installation-stage", InstallationStage::NotStarted)?;

        // Only proceed if the OS is Linux (specifically Ubuntu)
        if detect_os() != OperatingSystem::Linux {
            println!("DEBUG: Not a Linux system, skipping setup");
            app.emit("installation-stage", InstallationStage::SetupComplete)?;
            return Ok(());
        }

        println!("DEBUG: Starting Ubuntu-specific system setup...");
        app.emit("installation-stage", InstallationStage::CheckingDocker)?;

        // Check Ubuntu version
        Self::check_ubuntu_version(app).await?;

        Self::check_system_dependencies(app).await?;

        // Check port availability
        if !Self::check_port_availability(app).await? {
            return Err(anyhow!("Port 3306 is already in use"));
        }

        println!("DEBUG: Checking Docker installation status");
        if !Self::check_docker(app).await {
            println!("DEBUG: Docker not installed, attempting to install");
            app.emit("installation-stage", InstallationStage::DockerNotInstalled)?;
            
            match Self::get_sudo_password(app).await {
                Ok(password) => {
                    println!("DEBUG: Password retrieved successfully");
                    
                    // Emit installing state BEFORE starting installation
                    app.emit("installation-stage", InstallationStage::DockerInstalling)?;
                    
                    match Self::install_docker_with_password(app, &password).await {
                        Ok(_) => {
                            println!("DEBUG: Docker installation completed");
                            app.emit("installation-stage", InstallationStage::DockerInstalled)?;
                        },
                        Err(e) => {
                            println!("DEBUG: Docker installation failed: {:?}", e);
                            app.emit("installation-stage", InstallationStage::DockerInstallFailed)?;
                            return Err(anyhow!("Docker installation failed: {}", e));
                        }
                    }
                },
                Err(e) => {
                    // Handle password failure
                    println!("DEBUG: Password retrieval failed: {:?}", e);
                    app.emit("installation-stage", InstallationStage::DockerInstallFailed)?;
                    return Err(anyhow!("Password retrieval failed: {}", e));
                }
            }
        } else {
            println!("DEBUG: Docker already installed");
            app.emit("installation-stage", InstallationStage::DockerInstalled)?;
        }

        // Prepare Docker Compose using existing .env
        app.emit("installation-stage", InstallationStage::PreparingMySQLContainer)?;
        Self::prepare_docker_compose(app).await?;

        // Check and start MySQL container
        app.emit("installation-stage", InstallationStage::StartingMySQLContainer)?;
        match Self::manage_mysql_container(app).await {
            Ok(_) => {
                app.emit("installation-stage", InstallationStage::MySQLContainerStarted)?;
                
                // Create database pool and table after container is ready
                match crate::db::create_database_pool().await {
                    Ok(pool) => {
                        app.manage(pool); // Store pool in app state
                        app.emit("installation-stage", InstallationStage::SetupComplete)?;
                        Ok(())
                    },
                    Err(e) => {
                        app.emit("installation-stage", InstallationStage::MySQLSetupFailed)?;
                        Err(anyhow!("Database setup failed: {}", e))
                    }
                }
            },
            Err(e) => {
                app.emit("installation-stage", InstallationStage::MySQLSetupFailed)?;
                Err(e)
            }
        }
    }

    async fn parse_database_url(app: &tauri::AppHandle) -> Result<(String, String, String)> {
        let env_path = app.path().local_data_dir()?.join(".env");
        let env_content = fs::read_to_string(env_path).await?;
    
        let mut root_pass = String::new();
        let mut db_user = String::new();
        let mut db_pass = String::new();
        let mut db_name = String::new();
    
        for line in env_content.lines() {
            if line.starts_with("MYSQL_ROOT_PASSWORD=") {
                root_pass = line.split('=').nth(1).unwrap_or_default().trim_matches('"').to_string();
            }
            if line.starts_with("DATABASE_URL=") {
                let url_str = line.split_once('=')
                    .map(|(_, v)| v.trim().trim_matches('"'))
                    .ok_or_else(|| anyhow!("Malformed DATABASE_URL line"))?;
                
                let url = Url::parse(url_str)?;
                db_user = url.username().to_string();
                db_pass = url.password().unwrap_or_default().to_string();
                db_name = url.path().trim_start_matches('/').to_string();
            }
        }
    
        if db_user.is_empty() || db_pass.is_empty() || db_name.is_empty() {
            return Err(anyhow!("Missing required database credentials in .env"));
        }
    
        Ok((db_user, db_pass, db_name))
    }
    

    async fn check_port_availability(app: &tauri::AppHandle) -> Result<bool> {
        // Should be a single shell command
        let output = app.shell().command("sh")
            .args(["-c", "ss -tulpn | grep :3306"])
            .output()
            .await?;
    
        Ok(output.stdout.is_empty())
    }

    async fn get_sudo_password(app: &tauri::AppHandle) -> Result<String> {
        println!("DEBUG: Attempting to get sudo password");
        
        // Create a oneshot channel for password communication
        let (tx, rx) = oneshot::channel();
        let tx = Arc::new(tokio::sync::Mutex::new(Some(tx))); // Changed to tokio's Mutex
    
        // Generate a unique request ID
        let request_id = uuid::Uuid::new_v4().to_string();
        println!("DEBUG: Generated Request ID: {}", request_id);
    
        // Prepare password request
        let request = SudoPasswordRequest {
            request_id: request_id.clone(),
        };
    
        // Emit event to frontend requesting password
        app.emit("sudo-password-request", request)
            .map_err(|e| anyhow!("Failed to emit password request: {}", e))?;
    
        // Prepare event name
        let event_name = format!("sudo-password-response-{}", request_id);
    
        // Create a listener that will send the password through the oneshot channel
        let tx_clone = tx.clone();
        let handler = app.listen(event_name, move |event| {
            let tx_clone = tx_clone.clone();
            tauri::async_runtime::spawn(async move { // Run in async context
                // Directly use the payload as a string
                let password = event.payload().to_string();
                
                let mut guard = tx_clone.lock().await; // Use async lock
                if let Some(tx) = guard.take() {
                    let _ = tx.send(password);
                }
            });
        });
    
        // Wait for password with timeout
        let password = tokio::time::timeout(
            std::time::Duration::from_secs(120), // 2-minute timeout
            rx
        ).await??;
    
        // Remove the listener
        app.unlisten(handler);
    
        Ok(password)
    }

    async fn install_docker_with_password(
        app: &tauri::AppHandle,
        password: &str
    ) -> Result<()> {
        let commands = vec![
            ("sudo -S apt-get remove --purge -y docker docker-engine docker.io containerd runc || true", "Removing old Docker packages"),
            ("sudo -S apt-get autoremove -y || true", "Cleaning up unused dependencies"),
            ("sudo -S rm -rf /var/lib/docker || true", "Removing Docker data"),
            ("sudo -S rm -rf /var/lib/containerd || true", "Removing containerd data"),
            ("sudo -S rm -rf /etc/docker || true", "Removing Docker config"),
            ("sudo -S apt-get update", "Updating package list"),
            ("curl -fsSL https://get.docker.com | sudo -S sh", "Installing Docker engine"),
            ("sudo -S usermod -aG docker $USER", "Configuring user permissions"),
            ("sudo -S systemctl enable --now docker", "Enabling Docker service"),
            ("sudo -S chmod 666 /var/run/docker.sock || true", "Setting Docker socket permissions"),
        ];
    
        for (cmd, description) in commands {
            let full_cmd = format!("echo {} | {}", password, cmd);
            
            app.emit("docker-install-log", format!("\n▶ {}...", description))?;
    
            // Create channel for command completion notification
            let (tx, mut rx) = mpsc::channel(1);
    
            // Spawn command and get event receiver
            let (mut cmd_rx, _child) = app.shell()
                .command("bash")
                .args(["-c", &full_cmd])
                .spawn()?;
    
            // Clone app handle for async task
            let app_handle = app.clone();
    
            // Process command output in separate task
            tauri::async_runtime::spawn(async move {
                let mut exit_code = None;
    
                while let Some(event) = cmd_rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) => {
                            if let Ok(line_str) = String::from_utf8(line) {
                                app_handle.emit("docker-install-log", format!("{}\n", line_str)).unwrap();
                            }
                        }
                        CommandEvent::Stderr(line) => {
                            if let Ok(line_str) = String::from_utf8(line) {
                                app_handle.emit("docker-install-log", format!("[ERROR] {}\n", line_str)).unwrap();
                            }
                        }
                        CommandEvent::Terminated(status) => {
                            exit_code = Some(status);
                            break;
                        }
                        // Add default handler for other events
                        _ => {
                            // Handle unexpected events or ignore them
                            #[cfg(debug_assertions)]
                            println!("Unexpected command event: {:?}", event);
                        }
                    }
                }
    
                // Send exit code to main task
                let _ = tx.send(exit_code).await;
            });
    
            // Wait for command completion
            let status = rx.recv().await
                .ok_or_else(|| anyhow!("Command terminated without status"))?
                .ok_or_else(|| anyhow!("Command exited without status"))?;

            // Check exit code using struct field instead of method
            if status.code != Some(0) && !cmd.contains("|| true") {
                return Err(anyhow!("Docker installation failed at step: {}", description));
            }
        }
    
    
        // Verify Docker installation
        let verify = app.shell()
            .command("docker")
            .args(["info"])
            .output()
            .await?;
    
        if !verify.status.success() {
            let error = String::from_utf8_lossy(&verify.stderr);
            app.emit("docker-install-log", format!("\n✖ Verification failed: {}", error))?;
            return Err(anyhow!("Docker verification failed"));
        }
    
        app.emit("docker-install-log", "\n✓ Docker installed successfully")?;
        Ok(())
    }

    pub async fn check_docker(app: &tauri::AppHandle) -> bool {
        // Check with both normal and sudo permissions
        let checks = [
            "docker --version",
            "sudo docker --version",
            "sg docker -c 'docker --version'"
        ];
    
        for check in checks {
            if app.shell().command("sh")
                .args(["-c", check])
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }

    async fn prepare_docker_compose(app: &tauri::AppHandle) -> Result<()> {
    // Load environment variables from the `.env` file
    dotenv().ok();

    // Read variables from the `.env` file or use defaults
    let root_pass = env::var("MYSQL_ROOT_PASSWORD").unwrap_or_else(|_| "rootpass".to_string());
    let db_user = env::var("MYSQL_USER").unwrap_or_else(|_| "melvin".to_string());
    let user_pass = env::var("MYSQL_PASSWORD").unwrap_or_else(|_| "Ezeh4glamXgkaSeh".to_string());
    let db_name = env::var("MYSQL_DATABASE").unwrap_or_else(|_| "app_db".to_string());

    let docker_compose_content = format!(
        r#"version: '3.8'
services:
  mysql:
    image: mysql:8.0
    container_name: docker-mysql-1
    environment:
      MYSQL_ROOT_PASSWORD: "{}"
      MYSQL_DATABASE: "{}"
      MYSQL_USER: "{}"
      MYSQL_PASSWORD: "{}"
    ports:
      - "3307:3306"
    volumes:
      - mysql_data:/var/lib/mysql
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-u", "{}", "-p{}"]
      interval: 5s
      timeout: 5s
      retries: 10
    command: 
      - --default-authentication-plugin=mysql_native_password

volumes:
  mysql_data:"#,
        root_pass, db_name, db_user, user_pass, db_user, user_pass
    );

    let docker_compose_dir = app.path().local_data_dir()?.join("docker");
    async_fs::create_dir_all(&docker_compose_dir).await?;

    let compose_path = docker_compose_dir.join("docker-compose.yml");
    async_fs::write(&compose_path, docker_compose_content).await?;

    println!("Docker Compose file created at: {}", compose_path.display());
    Ok(())
}

    async fn verify_database_creation(app: &tauri::AppHandle) -> Result<()> {
        // Additional connection test using netcat
        let nc_output = app.shell().command("nc")
            .args(["-zv", "localhost", "3307"])
            .output()
            .await;
    
        if nc_output.is_err() {
            return Err(anyhow!("MySQL port not accessible"));
        }
    
        // Try multiple database connection methods
        let connection_commands = vec![
            // Direct MySQL command
            vec![
                "docker", "exec", "mysql", 
                "mysql", "-u", "root", 
                "-ppassword", 
                "-e", "CREATE DATABASE IF NOT EXISTS app_db; SHOW DATABASES LIKE 'app_db'"
            ],
            // Alternative connection method
            vec![
                "docker", "exec", "mysql", 
                "bash", "-c", 
                "mysql -u root -ppassword -e 'CREATE DATABASE IF NOT EXISTS app_db; SHOW DATABASES LIKE \"app_db\"'"
            ]
        ];
    
        for cmd in connection_commands {
            let output = app.shell().command(cmd[0])
                .args(&cmd[1..])
                .output()
                .await
                .map_err(|e| anyhow!("Failed to execute MySQL command: {}", e))?;
    
            let output_str = String::from_utf8_lossy(&output.stdout);
            let error_str = String::from_utf8_lossy(&output.stderr);
            
            println!("Database check output: {}", output_str);
            println!("Database check error: {}", error_str);
            
            if output_str.contains("app_db") {
                return Ok(());
            }
        }
    
        Err(anyhow!("Database 'app_db' creation failed after multiple attempts"))
    }

    async fn check_system_dependencies(app: &tauri::AppHandle) -> Result<()> {
        for tool in REQUIRED_TOOLS.iter() {
            let output = app.shell().command("which")
                .args([tool])
                .output()
                .await
                .map_err(|e| anyhow!("Missing dependency: {} - {}", tool, e))?;

            if !output.status.success() {
                return Err(anyhow!("Required tool {} not found", tool));
            }
        }
        Ok(())
    }


    async fn manage_mysql_container(app: &tauri::AppHandle) -> Result<()> {
        let emit_log = |message: &str| {
            let _ = app.emit("mysql-container-log", message);
            println!("{}", message);
        };
    
        emit_log("Starting MySQL container management...");
    
        // Hardcoded container name from compose file
        let container_name = "docker-mysql-1";
    
        // Check if container exists and is running
        let status_output = app.shell().command("docker")
            .args(["ps", "-a", "--filter", &format!("name={}", container_name), "--format", "{{.Status}}"])
            .output()
            .await?;
    
        let status = String::from_utf8_lossy(&status_output.stdout);
        let is_running = status.contains("Up");
    
        if !is_running {
            emit_log("Starting MySQL container...");
            let compose_path = app.path().local_data_dir()?.join("docker/docker-compose.yml");
            
            let compose_output = app.shell().command("docker")
                .args(["compose", "-f", &compose_path.to_string_lossy(), "up", "-d"])
                .output()
                .await?;
            
            emit_log(&format!("Docker compose output: {}", 
                String::from_utf8_lossy(&compose_output.stdout)));

            // Additional log for container pull and startup
            emit_log("Pulling MySQL container image...");
            let pull_output = app.shell().command("docker")
                .args(["pull", "mysql:8.0"])
                .output()
                .await?;
            
            emit_log(&format!("Pull output: {}", 
                String::from_utf8_lossy(&pull_output.stdout)));
        }
    
        emit_log("Starting MySQL container management...");

        let container_name = "docker-mysql-1";

        // Parse database credentials from .env
        let (db_user, db_pass, db_name) = Self::parse_database_url(app).await?;
        let mut db_attempts = 0;
        let max_db_attempts = 10;

        emit_log("Verifying MySQL connectivity...");

        while db_attempts < max_db_attempts {
            let check = app.shell().command("docker")
                .args([
                    "exec", 
                    container_name, 
                    "mysql", 
                    "-u", &db_user, 
                    &format!("-p{}", db_pass), 
                    "-e", &format!("USE {}; SELECT 1", db_name)
                ])
                .output()
                .await;
        
            match check {
                Ok(output) if output.status.success() => {
                    emit_log("✓ Successfully connected to MySQL");
                    return Ok(());
                }
                Ok(output) => {
                    emit_log(&format!("Connection failed: {}", 
                        String::from_utf8_lossy(&output.stderr)));
                }
                Err(e) => {
                    emit_log(&format!("Command error: {}", e));
                }
            }
        
            tokio::time::sleep(Duration::from_secs(3)).await;
            db_attempts += 1;
        }
    
        Err(anyhow!("Failed to connect after {} attempts", max_db_attempts))
    }

    pub async fn check_ubuntu_version(app: &tauri::AppHandle) -> Result<()> {
        // Check Ubuntu version and ensure it's a supported version
        let output = app.shell().command("lsb_release")
            .args(["-r", "-s"])
            .output()
            .await?;

        let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        // List of supported Ubuntu versions (LTS releases)
        let supported_versions = vec![
            "20.04", "22.04", "24.04"  // Add or modify as needed
        ];

        if supported_versions.contains(&version_str.as_str()) {
            Ok(())
        } else {
            Err(anyhow!("Unsupported Ubuntu version: {}", version_str))
        }
    }
}