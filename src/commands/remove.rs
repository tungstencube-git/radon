use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use ansi_term::Colour::{Green, Red};

use crate::utils::{get_bin_path, get_privilege_command};

pub fn remove(package: &str) {
    let privilege_cmd = get_privilege_command();
    let bin_path = get_bin_path();
    let bin_name = format!("{}(radon)", package);
    let installed_bin = bin_path.join(&bin_name);

    if !installed_bin.exists() {
        eprintln!("{}", Red.paint("Package not installed"));
        return;
    }

    let status = Command::new(&privilege_cmd)
        .arg("rm")
        .arg("-f")
        .arg(&installed_bin)
        .status()
        .expect("Failed to remove binary");

    if !status.success() {
        eprintln!("{}", Red.paint("Failed to remove binary"));
        return;
    }

    let list_path = Path::new("/etc/radon/listinstalled");
    let temp_list = Path::new("/tmp/radon_listinstalled.tmp");

    match fs::read_to_string(list_path) {
        Ok(content) => {
            let new_content: String = content
                .lines()
                .filter(|line| line.trim() != package)
                .collect::<Vec<_>>()
                .join("\n");

            if let Err(e) = fs::write(temp_list, &new_content) {
                eprintln!("{}: {}", Red.paint("Failed to write temp list"), e);
                return;
            }

            let mv_status = Command::new(&privilege_cmd)
                .arg("mv")
                .arg(temp_list)
                .arg(list_path)
                .status()
                .expect("Failed to update list");

            if mv_status.success() {
                println!("{}", Green.paint("~> Removed successfully"));
            } else {
                eprintln!("{}", Red.paint("Failed to update installed list"));
            }
        }
        Err(e) => eprintln!("{}: {}", Red.paint("Failed to read installed list"), e),
    }
}
