use std::env;
use std::path::Path;
use std::process::{Command, Stdio};
use ansi_term::Colour::Red;

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
        return "doas".into();
    }

    let sudo_available = Command::new("sh")
        .arg("-c")
        .arg("command -v sudo")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if sudo_available {
        return "sudo".into();
    }

    "sudo".into()
}

pub fn execute_privileged_command(args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    let privilege_cmd = get_privilege_command();

    if privilege_cmd == "su" {
        Command::new("su")
            .arg("-c")
            .arg(args.join(" "))
            .status()
    } else {
        Command::new(privilege_cmd)
            .args(args)
            .status()
    }
}
