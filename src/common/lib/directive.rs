use dusa_collection_utils::errors::ErrorArrayItem;
use dusa_collection_utils::functions::open_file;
use dusa_collection_utils::stringy::Stringy;
use dusa_collection_utils::types::PathType;
use serde::{Deserialize, Serialize};
use std::io;
use std::io::BufRead;
use walkdir::WalkDir;

// The directive functions will parse dependencies or programs that need to be ran when new data is pulled down.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Directive {
    // pub version: crate::version::Version,
    pub url: Stringy,
    pub track_directory: bool, // Triggering service restart if dir changes
    pub webserver: bool,          // This will determine if a new apache config is needed
    pub port: u16,
    pub php_fpm_version: Option<Stringy>, // Add this field to specify PHP-FPM version
    pub nodejs_bool: bool,
    pub nodejs_version: Option<Stringy>,
    pub nodejs_exec_command: Option<Stringy>, // This field will change what is written to the service file
    pub nodejs_pre_exec_command: Option<Stringy>, // This field will change what is written to the service file
    pub directive_executed: bool,                 // This should never be changed
}

impl Directive {
    // Function to create a default pre-filled directive
    pub fn default_prefilled() -> Self {
        Directive {
            // This will be configured with the proxy server
            url: Stringy::new("http://example.com"),
            // Enables directory tracking if the project 
            // creats a service file the tracker will auto
            // automatically restart the service file when 
            // changes are downloaded
            track_directory: true, 
            // Weather or not to auto configure a apache/nginx
            // config file.
            webserver: false,
            // ! CURRENTLY ONLY WORKS ON APACHE
            // Defines what port the application will listen on
            // ensures ufw has allowed the port for communication 
            // or the proxy routes correctly
            port: 8080,
            // 
            php_fpm_version: Some(Stringy::new("8.2")),
            nodejs_bool: true,
            nodejs_version: Some(Stringy::new("22.6.0")),
            nodejs_exec_command: Some(Stringy::new("npm start")),
            nodejs_pre_exec_command: Some(Stringy::new("npm run build")),
            // nodejs_pre_exec_command: None,
            directive_executed: false, // Should never change
        }
    }
}

pub async fn scan_directories(base_path: &PathType) -> Result<Vec<PathType>, ErrorArrayItem> {
    let mut directive_paths: Vec<PathType> = Vec::new();

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "directive.ais" {
            directive_paths.push(PathType::Path(entry.path().into()));
        }
    }

    Ok(directive_paths)
}

pub async fn parse_directive(path: &PathType) -> Result<Directive, ErrorArrayItem> {
    let content =
        read_json_without_comments(path.clone()).map_err(|err| ErrorArrayItem::from(err))?;
    let directive: Directive =
        serde_json::from_str(&content).map_err(|err| ErrorArrayItem::from(err))?;
    Ok(directive)
}

/// Reads a JSON file and removes lines starting with `#`
fn read_json_without_comments(file_path: PathType) -> Result<Stringy, ErrorArrayItem> {
    let file = open_file(file_path, false)?;
    let reader = io::BufReader::new(file);

    let mut json_string = String::new();

    for line in reader.lines() {
        let line = line?;
        // Skip lines that start with a `#`
        if !line.trim_start().starts_with('#') {
            json_string.push_str(&line);
            json_string.push('\n');
        }
    }

    Ok(json_string.into())
}
