use crate::constants::WEBSERVER_CONFIG_DIR;
use crate::directive::{parse_directive, scan_directories, Directive};
use crate::systemd::Services;
use dusa_collection_utils::errors::{ErrorArrayItem, Errors};
use dusa_collection_utils::types::PathType;
use std::fs;
use std::path::Path;
use std::process::Command;

fn _read_existing_nginx_config(directive: &Directive) -> Option<String> {
    let config_path = Path::new(WEBSERVER_CONFIG_DIR).join(format!("{}.conf", directive.url));
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(config_path) {
            return Some(content);
        }
    }
    None
}

pub fn create_nginx_config(
    directive: &Directive,
    base_path: &PathType,
) -> Result<bool, ErrorArrayItem> {
    let config_path = Path::new(WEBSERVER_CONFIG_DIR).join(format!("{}.conf", directive.url));
    let temp_config_path = config_path.with_extension("new");
    let backup_config_path = config_path.with_extension("bak");

    let php_fpm_config = match &directive.php_fpm_version {
        Some(version) if &*version.as_arc_str() == "7.4" => {
            r#"fastcgi_pass unix:/var/run/php/php7.4-fpm.sock;"#
        }
        Some(version) if &*version.as_arc_str() == "8.1" => {
            r#"fastcgi_pass unix:/var/run/php/php8.1-fpm.sock;"#
        }
        Some(version) if &*version.as_arc_str() == "8.2" => {
            r#"fastcgi_pass unix:/var/run/php/php8.2-fpm.sock;"#
        }
        _ => r#"fastcgi_pass unix:/var/run/php/php8.1-fpm.sock;"#,
        // No PHP-FPM handler if version is not specified or not recognized
    };

    let config_content = format!(
        r#"server {{
    listen {};
    server_name {};
    root {};
    
    location / {{
        try_files $uri $uri/ /index.php$is_args$args;
    }}

    location ~ \.php$ {{
        include snippets/fastcgi-php.conf;
        {}
    }}

    error_log /var/log/nginx/{}.error.log;
    access_log /var/log/nginx/{}.access.log;
}}
        "#,
        directive.port, directive.url, base_path, php_fpm_config, directive.url, directive.url
    );

    // Step 1: Backup the existing config if it exists
    if config_path.exists() {
        fs::copy(&config_path, &backup_config_path)?;
    }

    // Step 2: Write the new config to a temporary file
    fs::write(&temp_config_path, config_content)?;

    // Step 3: Test the new Nginx configuration
    let test_result = Command::new("nginx")
        .arg("-t")
        .output()
        .expect("Failed to run nginx -t");

    if !test_result.status.success() {
        eprintln!(
            "Nginx configuration test failed:\n{}",
            String::from_utf8_lossy(&test_result.stderr)
        );

        // Step 4: Rollback to the old config if the test failed
        if backup_config_path.exists() {
            fs::rename(&backup_config_path, &config_path)?;
            println!("Rolled back to the previous Nginx configuration.");
        }

        return Err(ErrorArrayItem::new(
            Errors::InitializationError,
            "Nginx configuration test failed".to_owned(),
        ));
    }

    // Step 5: If the test succeeds, replace the old config with the new one
    fs::rename(&temp_config_path, &config_path)?;

    // Cleanup: Remove the backup if everything succeeded
    if backup_config_path.exists() {
        fs::remove_file(&backup_config_path)?;
    }

    Ok(true) // Configuration changed
}

pub async fn reload_nginx() -> Result<bool, ErrorArrayItem> {
    let nginx = Services::WebServer;
    nginx.reload()
}

pub async fn process_directives(base_path: &PathType) -> Result<bool, ErrorArrayItem> {
    let directive_paths: Vec<dusa_collection_utils::types::PathType> =
        scan_directories(base_path).await?;
    let mut config_changed: bool = false;

    for directive_path in directive_paths {
        match parse_directive(&directive_path).await {
            Ok(directive) => {
                if create_nginx_config(&directive, &PathType::Path(directive_path.parent().unwrap().into()))? {
                    config_changed = true;
                }
            }
            Err(e) => eprintln!(
                "Failed to parse directive file {}: {}",
                directive_path.display(),
                e
            ),
        }
    }

    Ok(config_changed)
}
