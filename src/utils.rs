use std::fs::{self, File};
use std::path::Path;
use std::process::{Command, Stdio};
use ansi_term::Colour::Red;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub source: Option<String>,
    pub build_system: String,
    pub location: String,
    pub build_file: Option<String>,
    pub hash: Option<String>,
    pub version: Option<String>,
}

pub fn check_deps(deps: &[String]) {
    let missing: Vec<&str> = deps
        .iter()
        .filter(|d| {
            !Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {}", d))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
        .map(|d| d.as_str())
        .collect();

    if !missing.is_empty() {
        eprintln!("{}", Red.paint("MISSING DEPENDENCIES:"));
        for d in &missing {
            println!("- {}", d);
        }
        let pm = detect_package_manager();
        println!("\nRUN:\nsudo {} {}", pm, missing.join(" "));
        std::process::exit(1);
    }
}

pub fn detect_package_manager() -> &'static str {
    if Path::new("/etc/apt/sources.list").exists() {
        "apt install"
    } else if Path::new("/etc/pacman.conf").exists() {
        "pacman -S"
    } else if Path::new("/etc/xbps.d").exists() {
        "xbps-install"
    } else if Path::new("/etc/dnf/dnf.conf").exists() {
        "dnf install"
    } else if Path::new("/etc/zypp/zypp.conf").exists() {
        "zypper install"
    } else {
        "your-package-manager install"
    }
}

pub fn get_privilege_command() -> String {
    let doas_available = Command::new("sh")
        .arg("-c")
        .arg("command -v doas")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if doas_available {
        "doas".into()
    } else {
        "sudo".into()
    }
}

pub fn setup_radon_dirs() {
    let etc_radon = Path::new("/etc/radon");
    let var_lib_radon = Path::new("/var/lib/radon");
    let buildfiles = var_lib_radon.join("buildfiles");
    
    if !etc_radon.exists() {
        let status = Command::new(&get_privilege_command())
            .arg("mkdir")
            .arg("-p")
            .arg(etc_radon)
            .status();
        if status.is_ok() && status.unwrap().success() {
            let _ = Command::new(&get_privilege_command())
                .arg("touch")
                .arg(etc_radon.join("installed.yaml"))
                .status();
        }
    }

    if !var_lib_radon.exists() {
        let _ = Command::new(&get_privilege_command())
            .arg("mkdir")
            .arg("-p")
            .arg(&buildfiles)
            .status();
    } else if !buildfiles.exists() {
        let _ = Command::new(&get_privilege_command())
            .arg("mkdir")
            .arg("-p")
            .arg(&buildfiles)
            .status();
    }
}

pub fn get_installed_packages() -> Vec<InstalledPackage> {
    let path = Path::new("/etc/radon/installed.yaml");
    if path.exists() {
        let file = File::open(path).expect("Failed to open installed.yaml");
        serde_yaml::from_reader(file).unwrap_or_else(|_| vec![])
    } else {
        Vec::new()
    }
}
