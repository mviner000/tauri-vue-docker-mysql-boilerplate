// src/ubuntu_setup.rs

use crate::{detect_os, OperatingSystem};
use std::thread;
use std::time::Duration;
use anyhow::{Result, anyhow};
use tauri_plugin_shell::ShellExt;
use tauri::Manager;
use tauri::Emitter;
use serde::{Serialize, Deserialize};
use tokio::fs;

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
        // Emit initial stage
        app.emit("installation-stage", InstallationStage::NotStarted)?;

        // Only proceed if the OS is Linux (specifically Ubuntu)
        if detect_os() != OperatingSystem::Linux {
            app.emit("installation-stage", InstallationStage::SetupComplete)?;
            return Ok(());
        }

        println!("Starting Ubuntu-specific system setup...");
        app.emit("installation-stage", InstallationStage::CheckingDocker)?;

        // Check Ubuntu version
        Self::check_ubuntu_version(app).await?;

        // Check and install Docker
        if !Self::check_docker(app).await {
            app.emit("installation-stage", InstallationStage::DockerNotInstalled)?;
            
            match Self::open_docker_installation_terminal(app).await {
                Ok(_) => app.emit("installation-stage", InstallationStage::DockerInstalling)?,
                Err(_) => app.emit("installation-stage", InstallationStage::DockerInstallFailed)?,
            }
        } else {
            app.emit("installation-stage", InstallationStage::DockerInstalled)?;
            println!("Docker is already installed. Skipping installation.");
        }

        // Prepare Docker Compose file for MySQL
        app.emit("installation-stage", InstallationStage::PreparingMySQLContainer)?;
        Self::prepare_docker_compose(app).await?;

        // Check and start MySQL container
        app.emit("installation-stage", InstallationStage::StartingMySQLContainer)?;
            match Self::manage_mysql_container(app).await {
                Ok(_) => {
                    app.emit("installation-stage", InstallationStage::MySQLContainerStarted)?;
                    println!("✓ Ubuntu system setup completed");
                    
                    // Add a 30-second delay before emitting SetupComplete
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    
                    app.emit("installation-stage", InstallationStage::SetupComplete)?;
                    Ok(())
                },
                Err(_) => {
                    app.emit("installation-stage", InstallationStage::MySQLSetupFailed)?;
                    Err(anyhow!("MySQL container setup failed"))
                }
            }
    }

    pub async fn check_docker(app: &tauri::AppHandle) -> bool {
        app.shell().command("docker")
            .args(["--version"])
            .output()
            .await
            .is_ok()
    }

    pub async fn open_docker_installation_terminal(app: &tauri::AppHandle) -> Result<()> {
        let docker_install_script = r#"
        #!/bin/bash

        # Update package index
        echo "Updating package index..."
        sudo apt-get update

        # Install dependencies
        echo "Installing required dependencies..."
        sudo apt-get install -y \
            ca-certificates \
            curl \
            gnupg \
            lsb-release

        # Create directory for Docker's GPG key
        echo "Setting up Docker GPG key..."
        sudo mkdir -p /etc/apt/keyrings

        # Download and install Docker's official GPG key
        curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg

        # Set up the repository
        echo "Adding Docker repository..."
        echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
        $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

        # Update package index again
        echo "Updating package index with Docker repository..."
        sudo apt-get update

        # Install Docker Engine, Containerd, and Docker Compose
        echo "Installing Docker Engine..."
        sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

        # Add current user to docker group
        echo "Adding current user to docker group..."
        sudo usermod -aG docker $USER

        # Start Docker service
        echo "Starting Docker service..."
        sudo systemctl start docker

        # Enable Docker to start on boot
        echo "Enabling Docker to start on system boot..."
        sudo systemctl enable docker

        # Verify Docker installation
        echo "Verifying Docker installation..."
        docker --version

        echo "Docker installation complete! Please log out and log back in, or run 'newgrp docker' to use Docker without sudo."
        read -p "Press Enter to close this terminal..."
        "#;

    // Write the script to a temporary file
    let script_path = std::env::temp_dir().join("docker_install.sh");
    tokio::fs::write(&script_path, docker_install_script).await?;

    // Make the script executable
    app.shell().command("chmod")
        .args(["+x", &script_path.to_string_lossy()])
        .output()
        .await?;

    // Open terminal with the installation script
    #[cfg(target_os = "linux")]
    {
        // Use x-terminal-emulator for Ubuntu/Debian
        app.shell().command("x-terminal-emulator")
            .args(["-e", &format!("bash {}", script_path.to_string_lossy())])
            .output()
            .await?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        return Err(anyhow!("Terminal opening supported only on Linux"));
    }

    // Wait a bit to allow the terminal to open
    thread::sleep(Duration::from_secs(2));

    Ok(())

    }

    async fn prepare_docker_compose(app: &tauri::AppHandle) -> Result<()> {
        // Define Docker Compose content
        let docker_compose_content = r#"
version: '3.8'
services:
  mysql:
    image: mysql:8.0
    container_name: mysql
    volumes:
      - mysql_data:/var/lib/mysql
    environment:
      MYSQL_ROOT_PASSWORD: password
      MYSQL_DATABASE: app_db
      MYSQL_USER: appuser
      MYSQL_PASSWORD: apppassword
    ports:
      - "3306:3306"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost", "-u", "root", "-ppassword"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  mysql_data:
    name: mysql_data
    "#;

        // Create docker-compose directory in app's local data directory
        let docker_compose_dir = app.path().local_data_dir()?.join("docker");
        fs::create_dir_all(&docker_compose_dir).await?;

        // Write Docker Compose file
        let compose_path = docker_compose_dir.join("docker-compose.yml");
        fs::write(&compose_path, docker_compose_content).await?;

        println!("Docker Compose file created at: {}", compose_path.display());
        Ok(())
    }

    async fn verify_database_creation(app: &tauri::AppHandle) -> Result<()> {
        // Additional connection test using netcat
        let nc_output = app.shell().command("nc")
            .args(["-zv", "localhost", "3306"])
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

    async fn manage_mysql_container(app: &tauri::AppHandle) -> Result<()> {
        // Add more verbose logging
        println!("Attempting to manage MySQL container...");
    
        // Check if MySQL container is already running
        let container_list = app.shell().command("docker")
            .args(["ps", "-f", "name=mysql", "--format", "{{.Status}}"])
            .output()
            .await?;
    
        let container_status = String::from_utf8_lossy(&container_list.stdout);
    
        if container_status.trim().is_empty() || container_status.contains("Exited") {
            println!("MySQL container not running. Attempting to start...");
            
            let docker_compose_dir = app.path().local_data_dir()?.join("docker");
            let compose_path = docker_compose_dir.join("docker-compose.yml");
    
            // Run Docker Compose with output capture for better error logging
            let compose_output = app.shell().command("docker")
                .args(["compose", "-f", compose_path.to_string_lossy().as_ref(), "up", "-d"])
                .output()
                .await?;
    
            // Log stdout and stderr
            println!("Docker Compose Stdout: {}", String::from_utf8_lossy(&compose_output.stdout));
            println!("Docker Compose Stderr: {}", String::from_utf8_lossy(&compose_output.stderr));
    
            // Wait for container to be fully healthy
            for attempt in 0..30 {  // 30 attempts with 2-second intervals
                let health_output = app.shell().command("docker")
                    .args(["inspect", "--format={{.State.Health.Status}}", "mysql"])
                    .output()
                    .await?;
                
                let health_status = String::from_utf8_lossy(&health_output.stdout).trim().to_string();
                println!("Container health status (Attempt {}): {}", attempt + 1, health_status);
    
                if health_status == "healthy" {
                    break;
                }
    
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
    
            // Additional container check
            let check_output = app.shell().command("docker")
                .args(["ps", "-f", "name=mysql"])
                .output()
                .await?;
            
            println!("Container Check Output: {}", String::from_utf8_lossy(&check_output.stdout));
    
            // Verify database creation with more detailed error handling
            match Self::verify_database_creation(app).await {
                Ok(_) => {
                    println!("✓ MySQL container started and database verified");
                    Ok(())
                },
                Err(e) => {
                    println!("Database verification failed: {}", e);
                    Err(anyhow!("Failed to verify MySQL database: {}", e))
                }
            }
        } else {
            println!("MySQL container is already running");
            Ok(())
        }
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