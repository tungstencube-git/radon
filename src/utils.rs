use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use serde_yaml;
use crate::commands::install::InstalledPackage;

pub fn get_privilege_command() -> String {
    if Path::new("/usr/bin/doas").exists() {
        "doas".to_string()
    } else {
        "sudo".to_string()
    }
}

pub fn check_deps(deps: &[String]) {
    let missing = deps.iter().filter(|d| {
        !Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", d))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }).collect::<Vec<_>>();

    if !missing.is_empty() {
        eprintln!("Missing dependencies: {:?}", missing);
        std::process::exit(1);
    }
}

pub fn setup_radon_dirs() {
    let etc_radon = Path::new("/etc/radon");
    let var_lib_radon = Path::new("/var/lib/radon");
    let buildfiles = var_lib_radon.join("buildfiles");

    if !etc_radon.exists() {
        Command::new(&get_privilege_command())
            .arg("mkdir")
            .arg("-p")
            .arg(etc_radon)
            .status()
            .expect("Failed to create /etc/radon/");
    }

    if !var_lib_radon.exists() {
        Command::new(&get_privilege_command())
            .arg("mkdir")
            .arg("-p")
            .arg(var_lib_radon)
            .status()
            .expect("Failed to create /var/lib/radon/");
    }

    if !buildfiles.exists() {
        Command::new(&get_privilege_command())
            .arg("mkdir")
            .arg("-p")
            .arg(buildfiles)
            .status()
            .expect("Failed to create /var/lib/radon/buildfiles/");
    }
}

pub fn get_installed_packages() -> Vec<InstalledPackage> {
    let path = Path::new("/etc/radon/installed.yaml");
    if path.exists() {
        let file = fs::File::open(path).expect("Failed to open installed.yaml");
        return serde_yaml::from_reader(file).unwrap_or_default();
    }
    Vec::new()
}
