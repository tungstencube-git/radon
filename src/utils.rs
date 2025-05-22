use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use ansi_term::Colour::Red;

pub fn check_deps(deps: &[String]) {
    let missing: Vec<&str> = deps
        .iter()
        .filter(|d| {
            !Command::new("which")
                .arg(d)
                .output()
                .map(|o| o.status.success())
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
        "apt-get install"
    } else if Path::new("/etc/pacman.conf").exists() {
        "pacman -S"
    } else if Path::new("/etc/xbps.d").exists() {
        "xbps-install"
    } else {
        "your-package-manager install"
    }
}

pub fn get_bin_path() -> PathBuf {
    let path = env::var("PATH").unwrap_or_default();
    if path.contains("/usr/local/bin") {
        PathBuf::from("/usr/local/bin")
    } else {
        PathBuf::from("/usr/bin")
    }
}
