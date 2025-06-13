use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use ansi_term::Colour::{Green, Red, Yellow};
use crate::utils;

fn get_local_bin_path() -> std::path::PathBuf {
    env::var("HOME")
        .map(|home| std::path::PathBuf::from(home).join(".local/bin"))
        .expect("HOME environment variable not set")
}

pub fn remove(package: &str) {
    let privilege_cmd = utils::get_privilege_command();
    let mut installed = utils::get_installed_packages();
    
    if let Some(index) = installed.iter().position(|p| p.name == package) {
        let pkg = installed.remove(index);
        let bin_path = Path::new(&pkg.location);
        
        if bin_path.exists() {
            if pkg.location.starts_with("/usr") {
                Command::new(&privilege_cmd)
                    .arg("rm")
                    .arg("-f")
                    .arg(bin_path)
                    .status()
                    .expect("Failed to remove system binary");
            } else {
                fs::remove_file(bin_path)
                    .unwrap_or_else(|_| panic!("Failed to remove local binary: {:?}", bin_path));
            }
            println!("Removed: {}", bin_path.display());
        } else {
            println!("{}: Binary not found at {}", Yellow.paint("Warning"), bin_path.display());
        }
        
        let temp_path = Path::new("/tmp").join("radon-installed.yaml");
        fs::write(&temp_path, serde_yaml::to_string(&installed).unwrap()).unwrap();
        
        Command::new(&privilege_cmd)
            .arg("mv")
            .arg(&temp_path)
            .arg("/etc/radon/installed.yaml")
            .status()
            .expect("Failed to update package list");
        
        println!("{}", Green.paint("~> Removed successfully"));
    } else {
        eprintln!("{}: Package '{}' not found", Red.paint("Error"), package);
    }
}

