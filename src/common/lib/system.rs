use dusa_collection_utils::stringy::Stringy;
use gethostname::gethostname;
use std::{collections::HashMap, fs, path::Path};
use sysinfo::System;
use uuid::Uuid;

pub const MACHINE_ID_FILE: &str = "/etc/artisan_id";

pub fn get_system_stats() -> HashMap<Stringy, String> {
    let mut system = System::new_all();
    system.refresh_all();

    let mut stats: HashMap<Stringy, String> = HashMap::new();
    stats.insert(
        Stringy::new("CPU Usage"),
        format!("{:.2}%", system.global_cpu_info().cpu_usage()),
    );
    stats.insert(
        Stringy::new("Total RAM"),
        format!("{} MB", system.total_memory() / 1024),
    );
    stats.insert(
        Stringy::new("Used RAM"),
        format!("{} MB", system.used_memory() / 1024),
    );
    stats.insert(
        Stringy::new("Total Swap"),
        format!("{} MB", system.total_swap() / 1024),
    );
    stats.insert(
        Stringy::new("Used Swap"),
        format!("{} MB", system.used_swap() / 1024),
    );
    stats.insert(Stringy::new("Hostname"), format!("{:?}", gethostname()));

    stats
}

pub fn get_machine_id() -> Stringy {
    if Path::new(MACHINE_ID_FILE).exists() {
        let data: Stringy = fs::read_to_string(MACHINE_ID_FILE).unwrap_or_else(|_| generate_machine_id().to_string()).into();
        data
    } else {
        generate_machine_id()
    }
}

pub fn generate_machine_id() -> Stringy {
    let id = Uuid::new_v4().to_string();
    fs::write(MACHINE_ID_FILE, &id).expect("Unable to write machine ID file");
    id.into()
}
