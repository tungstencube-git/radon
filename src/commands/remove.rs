use std::fs;
use std::path::Path;
use std::process::Command;
use ansi_term::Colour::{Green, Red};
use crate::utils::get_bin_path;

pub fn remove(package: &str) {
    let list = Path::new("/etc/radon/listinstalled");
    let content = fs::read_to_string(list).unwrap_or_default();

    if !content.contains(package) {
        eprintln!("{}", Red.paint("Package not found!"));
        return;
    }

    let bin = format!("{}(radon)", package);
    for dir in ["/usr/local/bin", "/usr/bin"] {
        let path = Path::new(dir).join(&bin);
        if path.exists() {
            Command::new("sudo")
                .arg("rm")
                .arg("-f")
                .arg(path)
                .status()
                .expect("Failed to remove binary");
            break;
        }
    }

    let remaining: Vec<String> = content
        .lines()
        .filter(|l| *l != package)
        .map(|l| l.to_string())
        .collect();
    fs::write(list, remaining.join("\n")).expect("Failed to update package list");
    println!("{}", Green.paint("PACKAGE UNINSTALLED"));
}
