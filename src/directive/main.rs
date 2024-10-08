// This file monitors the directive.ais files and runs things like node, build scripts and so on
// We copy the directive.ais file to a system directory, then we execute what needs to be done, Ie configure apache or node install whatever.
// When change the directive_executed bool to true on the version we copied.
// We save two hashes to ensure we aren't changing thing when they arent needed. We save a hash before copy. and we save a hash that we modify.

use ais_common::{
    apache::{create_apache_config, reload_apache}, common::{AppName, AppStatus, Status}, directive::{parse_directive, scan_directories}, messages::report_status, monitor::{create_monitoring_script, create_monitoring_service, MONITOR_DIR}, node::{create_node_systemd_service, run_npm_install}, system::current_timestamp, systemd::{enable_now, reload_systemd_daemon}, version::Version
};
use dusa_collection_utils::{
    errors::{ErrorArray, ErrorArrayItem},
    functions::{create_hash, make_file, open_file, truncate},
    types::{ClonePath, PathType},
};
use simple_pretty::{notice, warn};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    thread,
    time::Duration,
};

pub const SYSTEM_DIRECTIVE_PATH: &str = "/tmp";

fn generate_directive_hash(directive_path: PathType) -> Result<String, ErrorArrayItem> {
    let mut directive_file: std::fs::File = open_file(directive_path.clone(), false)?;

    let directive_parent = get_parent_dir(&directive_path);

    let service_id: String = directive_parent.to_string().replace("/var/www/ais/", "");

    let mut directive_buffer: Vec<u8> = Vec::new();

    directive_file
        .read_to_end(&mut directive_buffer)
        .map_err(|err| ErrorArrayItem::from(err))?;

    let directive_hash: String =
        String::from_utf8(directive_buffer).map_err(|err| ErrorArrayItem::from(err))?;

    Ok(create_hash(format!("{}_{}", directive_hash, service_id)))
}

fn store_directive(directive_path: PathType) -> Result<(), ErrorArrayItem> {
    if directive_path.exists() {
        let new_directive_path = PathType::Content(format!(
            "{}/{}",
            SYSTEM_DIRECTIVE_PATH,
            truncate(&generate_directive_hash(directive_path.clone_path())?, 8)
        ));

        print!("{}", new_directive_path);

        make_file(new_directive_path.clone_path(), ErrorArray::new_container())
            .uf_unwrap()
            .unwrap();

        let bytes_copied = fs::copy(directive_path, new_directive_path)
            .map_err(|err| ErrorArrayItem::from(err))?;

        // just for sanity
        if bytes_copied == 0 {
            return Err(ErrorArrayItem::new(
                dusa_collection_utils::errors::Errors::GeneralError,
                String::from(
                    "When coping the directive file,the operation reported the size was 0 ",
                ),
            ));
        }

        Ok(())
    } else {
        return Err(ErrorArrayItem::new(
            dusa_collection_utils::errors::Errors::GeneralError,
            String::from("There was no directive.ais file in the path given"),
        ));
    }
}

fn get_parent_dir(directive_path: &PathType) -> PathType {
    PathType::Path(
        directive_path
            .clone()
            .parent()
            .or_else(|| Some(Path::new("/tmp"))) // this unwrap call should be safe because we can never end up with None for this item
            .unwrap()
            .to_owned()
            .into_boxed_path(),
    )
}

fn check_directive(directive_path: PathType) -> Result<bool, ErrorArrayItem> {
    let new_directive_path = PathType::Content(format!(
        "{}/{}",
        SYSTEM_DIRECTIVE_PATH,
        truncate(&generate_directive_hash(directive_path.clone_path())?, 8)
    ));

    Ok(new_directive_path.exists())
}

/// This need the directive in the project folder
async fn executing_directive(directive_path: PathType) -> Result<(), ErrorArrayItem> {
    let directive: ais_common::directive::Directive = parse_directive(&directive_path).await?;
    let directive_parent = get_parent_dir(&directive_path);
    notice(&format!("Executing directive: {}", directive_parent));

    // Checking if we need to reconfigure apache
    if directive.apache {
        let changed = create_apache_config(&directive, &directive_parent)?;
        match changed {
            true => {
                match reload_apache().await {
                    Ok(b) => {
                        if !b {
                            eprintln!("My god we killed apache, quick email the admin");
                            eprintln!("The apache config we rolled out most likely killed apache");
                        }
                    }
                    Err(e) => return Err(e),
                }
                print!("Apache config updated for {:#?}", directive_parent);
            }
            false => print!("The project {} needs no changes", directive_parent),
        }
    }

    // Checking if the project is a node thing.
    if directive.nodejs_bool {
        let _version = match directive.nodejs_version {
            Some(d) => d,
            None => String::from("22"),
        };

        // TODO add check with nvm to ensure the correct version is installed.

        // build application
        if let Ok(_) = run_npm_install(&directive_parent) {
            println!("Npm dependencies installed for {}", directive_path);
        } else {
            return Err(ErrorArrayItem::new(
                dusa_collection_utils::errors::Errors::GeneralError,
                String::from("An error occurred while installing npm dependencies"),
            ));
        };

        // TODO MITOBYTE HAS THE WRONG VERSION of directive.ais
        // // create system d service file
        // let exec_start = match directive.nodejs_exec_command {
        //     Some(d) => d,
        //     None => format!("/usr/bin/npm dev run"),
        // };

        let exec_start = format!("/usr/bin/npm run dev");

        let description: &str = &format!("Ais project id {}", &directive_parent);

        // Create the systemd service file content
        let service_file_data =
            create_node_systemd_service(&exec_start, &directive_parent, description)?;

        // Write the service file
        let service_id: String = directive_parent.to_string().replace("/var/www/ais/", "");

        let service_path: PathType =
            PathType::Content(format!("/etc/systemd/system/{}.service", service_id));

        if service_path.exists() {
            fs::remove_file(service_path.clone())?;
        }

        let mut service_file = File::create(service_path.clone())?;
        service_file
            .write(service_file_data.as_bytes())
            .map_err(|err| ErrorArrayItem::from(err))?;

        // Setting up monitoring
        create_monitoring_script(&directive_parent.to_string(), &service_id)?;
        create_monitoring_service(
            &service_id,
            &format!("{}{}.monitor", MONITOR_DIR, &service_id),
        )?;

        // Reload systemd daemon
        reload_systemd_daemon()?;

        // Enable and start the service and the path unit
        enable_now(format!("{}", service_id))?;
        enable_now(format!("{}_monitor", service_id))?;
    }

    // report to the aggregator
    let status = Status {
        app_name: AppName::Directive,
        app_status: AppStatus::Running,
        timestamp: current_timestamp(),
        version: Version::get(),
    };

    if let Err(err) = report_status(status).await {
        Err(err)
    } else {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let base_path = "/var/www/ais";

    loop {
        let directive_paths = match scan_directories(base_path).await {
            Ok(d) => d,
            Err(e) => {
                // Set the application status to warning in the aggregator as it's running with faults
                let status: Status = Status {
                    app_name: AppName::Directive,
                    app_status: AppStatus::Warning,
                    timestamp: current_timestamp(),
                    version: Version::get(),
                };
                if let Err(err) = report_status(status).await {
                    ErrorArray::new(vec![e, err]).display(true)
                }
                unreachable!("Error scanning dirs")
            }
        };

        for directive_path_string in directive_paths {
            let directive_path: PathType = PathType::PathBuf(directive_path_string);

            // If we haven't already stored the directive data
            if !check_directive(directive_path.clone())
                .expect("Error while opening the directive path")
            {
                match executing_directive(directive_path.clone_path()).await {
                    Ok(_) => (),
                    Err(e1) => {
                        let status: Status = Status {
                            app_name: AppName::Directive,
                            app_status: AppStatus::Warning,
                            timestamp: current_timestamp(),
                            version: Version::get(),
                        };

                        let e2 = report_status(status).await;

                        match e2 {
                            Ok(_) => warn(&format!(
                                "Error executing directive, {}: {}",
                                directive_path, e1
                            )),
                            Err(e2) => ErrorArray::new(vec![e1, e2]).display(true),
                        }
                    }
                }

                if store_directive(directive_path).is_ok() {
                    return;
                } else {
                    print!("we have executed the directive but cannot store that we have. The directive may be in a loop");
                    // Set the application status to warning in the aggregator as it's running with faults
                    let status: Status = Status {
                        app_name: AppName::Directive,
                        app_status: AppStatus::Warning,
                        timestamp: current_timestamp(),
                        version: Version::get(),
                    };
                    if let Err(err) = report_status(status).await {
                        ErrorArray::new(vec![err]).display(false)
                    }
                    return;
                }
            }
        }

        // Send okay
        let status: Status = Status {
            app_name: AppName::Directive,
            app_status: AppStatus::Running,
            timestamp: current_timestamp(),
            version: Version::get(),
        };

        if let Err(err) = report_status(status).await {
            ErrorArray::new(vec![err]).display(false)
        }

        thread::sleep(Duration::from_secs(10));
    }
}
