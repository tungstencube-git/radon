use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use ansi_term::Colour::{Green, Red};
use crate::cli::RemoveTarget;

use crate::utils::get_privilege_command;

fn get_local_bin_path() -> PathBuf {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".local/bin"))
        .expect("HOME environment variable not set")
}

pub fn remove(target: RemoveTarget) {
    match target {
        RemoveTarget::Package { package } => remove_package(&package),
        RemoveTarget::Cache => remove_cache(),
    }
}

pub fn remove_cache() {
    let tmp_radon = Path::new("/tmp/radon");
    if tmp_radon.exists() {
        fs::remove_dir_all(tmp_radon).expect("Failed to remove cache");
        println!("{}", Green.paint("Cache removed successfully"));
    } else {
        println!("{}", Green.paint("Cache already clean"));
    }
}

pub fn remove_package(package: &str) {
    let privilege_cmd = get_privilege_command();
    let system_bin_path = Path::new("/usr/local/bin");
    let local_bin_path = get_local_bin_path();
    let list_path = Path::new("/etc/radon/installed");
    let temp_list = Path::new("/tmp/radon_installed.tmp");

    let installed_bins: Vec<PathBuf> = [system_bin_path, &local_bin_path]
        .iter()
        .flat_map(|path| {
            fs::read_dir(path).ok().map(|entries| {
                entries.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name()
                            .and_then(|f| f.to_str())
                            .map(|name| name.contains(package) && name.ends_with("(radon)"))
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect();

    if installed_bins.is_empty() {
        eprintln!("{}: Package '{}' not found", Red.paint("Error"), package);
        return;
    }

    for bin_path in &installed_bins {
        if bin_path.starts_with("/usr/local/bin") {
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
    }

    match fs::read_to_string(list_path) {
        Ok(content) => {
            let new_content: String = content
                .lines()
                .filter(|line| !line.contains(package))
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
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("{}", Green.paint("~> Removed successfully"));
        }
        Err(e) => eprintln!("{}: {}", Red.paint("Failed to read installed list"), e),
    }
}
