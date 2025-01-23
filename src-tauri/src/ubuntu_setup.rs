// src/ubuntu_setup.rs

use crate::{detect_os, OperatingSystem};
use std::thread;
use std::time::Duration;
use anyhow::{Result, anyhow};
use tauri_plugin_shell::ShellExt;
use tauri::Manager;
use tokio::process::Command;
use tokio::fs;

pub struct UbuntuSystemSetup;

impl UbuntuSystemSetup {
    pub async fn setup_ubuntu_system(app: &tauri::AppHandle) -> Result<()> {
        // Only proceed if the OS is Linux (specifically Ubuntu)
        if detect_os() != OperatingSystem::Linux {
            println!("Skipping Ubuntu-specific setup: Not running on Linux");
            return Ok(());
        }

        println!("Starting Ubuntu-specific system setup...");

        // Check Ubuntu version
        Self::check_ubuntu_version(app).await?;

        // Open terminal with Docker installation script only if Docker is not installed
        if !Self::check_docker(app).await {
            Self::open_docker_installation_terminal(app).await?;
        } else {
            println!("Docker is already installed. Skipping installation.");
        }

        // Prepare Docker Compose file for MySQL
        Self::prepare_docker_compose(app).await?;

        // Check and start MySQL container
        Self::manage_mysql_container(app).await?;

        println!("✓ Ubuntu system setup completed");
        Ok(())
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
    ports:
      - "3306:3306"
    restart: unless-stopped

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
        // Use MySQL client to check database existence
        let output = app.shell().command("docker")
            .args([
                "exec", 
                "mysql", 
                "mysql", 
                "-u", "root", 
                "-ppassword", 
                "-e", 
                "SHOW DATABASES LIKE 'app_db'"
            ])
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        if output_str.contains("app_db") {
            Ok(())
        } else {
            Err(anyhow!("Database 'app_db' not found"))
        }
    }

    async fn manage_mysql_container(app: &tauri::AppHandle) -> Result<()> {
        // Check if MySQL container is already running
        let container_list = app.shell().command("docker")
            .args(["ps", "-f", "name=mysql", "--format", "{{.Status}}"])
            .output()
            .await?;
    
        let container_status = String::from_utf8_lossy(&container_list.stdout);
    
        if container_status.trim().is_empty() || container_status.contains("Exited") {
            println!("Starting MySQL container...");
            
            let docker_compose_dir = app.path().local_data_dir()?.join("docker");
            let compose_path = docker_compose_dir.join("docker-compose.yml");
    
            // Create a shell script for verbose Docker Compose
            let verbose_script = format!(r#"#!/bin/bash
    cd {}
    echo "Running Docker Compose up with verbose logging..."
    docker compose -f {} up -d
    echo "Press Enter to close this terminal..."
    read
    "#, 
                docker_compose_dir.to_string_lossy(), 
                compose_path.to_string_lossy()
            );
    
            // Write the verbose script
            let script_path = std::env::temp_dir().join("docker_compose_up.sh");
            tokio::fs::write(&script_path, verbose_script).await?;
    
            // Make script executable
            app.shell().command("chmod")
                .args(["+x", &script_path.to_string_lossy()])
                .output()
                .await?;
    
            // Open terminal with the script
            app.shell().command("x-terminal-emulator")
                .args(["-e", &script_path.to_string_lossy()])
                .output()
                .await?;
    
            // Wait for potential container startup
            thread::sleep(Duration::from_secs(20));
    
            // Verify database creation
            Self::verify_database_creation(app).await?;
    
            println!("✓ MySQL container started successfully");
        } else {
            println!("MySQL container is already running");
        }
    
        Ok(())
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