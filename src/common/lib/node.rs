use core::str;
use std::{
    fs, io, process::{Command, ExitStatus, Stdio}
};

use dusa_collection_utils::{errors::{ErrorArrayItem, Errors}, stringy::Stringy, types::PathType};

use crate::version::Version;

/// Function to create a systemd service file dynamically
pub fn create_node_systemd_service(
    exec_start: &str,
    exec_pre: &str,
    working_dir: &PathType,
    description: &str,
) -> Result<Stringy, ErrorArrayItem> {
    // Setting environmental variables depending on the directive file
    let service_file_content: Stringy = format!(
        r#" # THIS SERVICE FILE IS MANAGED BY: Artisan Platform Version: {} DO NOT CHANGE
[Unit]
Description={}
After=network.target

[Service]
PermissionsStartOnly=false
ExecStart={}
{}
Restart=always
User=www-data
Group=www-data
Environment=PATH=/usr/bin:/usr/local/bin
WorkingDirectory={}

[Install]
WantedBy=multi-user.target
"#,
        Version::get(), description, exec_start, exec_pre, working_dir
    ).into();

    Ok(service_file_content)
}

pub fn check_and_install_node_version(version: Stringy) -> Result<(), ErrorArrayItem> {
    // Step 1: Check if Node version is installed
    let check_command = Command::new("bash")
        .arg("-c")
        .arg(format!("source ~/.nvm/nvm.sh && nvm ls {}", version))
        .output()?;
    
    let output_str = str::from_utf8(&check_command.stdout)?;

    // Step 2: If version is not found, install it
    if !output_str.contains(&*version) {
        println!("Node.js version {} is not installed. Installing...", version);

        let install_command = Command::new("bash")
            .arg("-c")
            .arg(format!("source ~/.nvm/nvm.sh && nvm install {}", version))
            .stdout(Stdio::inherit()) // Stream output directly to the terminal
            .stderr(Stdio::inherit()) // Stream errors directly to the terminal
            .output()?;

        if !install_command.status.success() {
            return Err(ErrorArrayItem::new(Errors::GeneralError, format!("Failed to install Node.js version {}", version).into()))
        }
        println!("Node.js version {} installed successfully.", version);
    } else {
        println!("Node.js version {} is already installed.", version);
    }

    Ok(())
}

/// Check if `npm install` is needed based on the state of node_modules and package-lock.json.
pub fn needs_npm_install(project_path: &PathType) -> Result<bool, ErrorArrayItem> {
    // Define paths
    let node_modules_path = project_path.join("node_modules");
    let package_lock_path = project_path.join("package-lock.json");
    let package_json_path = project_path.join("package.json");

    // Check if node_modules/ exists
    if !node_modules_path.exists() {
        println!("node_modules/ directory does not exist. npm install is needed.");
        return Ok(true);
    }

    // Get the modification time of package-lock.json (or package.json if package-lock.json does not exist)
    let lock_file_mod_time = if package_lock_path.exists() {
        fs::metadata(&package_lock_path)?.modified()?
    } else {
        fs::metadata(&package_json_path)?.modified()?
    };

    // Get the modification time of node_modules/
    let node_modules_mod_time = fs::metadata(&node_modules_path)?.modified()?;

    // Compare the modification times
    if lock_file_mod_time > node_modules_mod_time {
        println!("package-lock.json or package.json is more recent than node_modules/. npm install is needed.");
        return Ok(true);
    }

    // Check for incomplete node_modules/ (if node_modules is empty)
    let node_modules_contents = fs::read_dir(node_modules_path)?;
    if node_modules_contents.count() == 0 {
        println!("node_modules/ is empty. npm install is needed.");
        return Ok(true);
    }

    Ok(false)
}

pub fn run_npm_install(working_dir: &PathType) -> io::Result<ExitStatus> {
    // Use `Command` to run `npm install` in the specified directory
    let status = Command::new("npm")
        .arg("install")
        .current_dir(working_dir) // Set the working directory
        .status()?; // Run the command and capture its exit status

    Ok(status)
}
