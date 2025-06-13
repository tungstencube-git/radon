use crate::commands::install::install;
use crate::utils;
use ansi_term::Colour::{Green, Red, Yellow};
use sha2::{Sha256, Digest};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub fn upgrade(all: bool, package: Option<&str>) {
    let packages: Vec<String> = if all {
        utils::get_installed_packages()
            .iter()
            .map(|p| p.name.clone())
            .collect()
    } else if let Some(pkg) = package {
        vec![pkg.to_string()]
    } else {
        eprintln!("{}", Red.paint("Specify --all or --package"));
        return;
    };

    if packages.is_empty() {
        println!("No packages to upgrade");
        return;
    }

    for pkg in packages {
        println!("Checking {} for updates...", pkg);
        let installed = utils::get_installed_packages();
        let pkg_index = match installed.iter().position(|p| p.name == pkg) {
            Some(idx) => idx,
            None => {
                println!("{}: Package not installed", Yellow.paint("Warning"));
                continue;
            }
        };
        
        let buildfile_dir = Path::new("/var/lib/radon/buildfiles").join(&pkg);
        
        if !buildfile_dir.exists() {
            println!("{}: No build files found for {}", Yellow.paint("Warning"), pkg);
            continue;
        }

        let metadata_path = buildfile_dir.join("metadata.toml");
        if !metadata_path.exists() {
            println!("{}: No metadata found for {}", Yellow.paint("Warning"), pkg);
            continue;
        }

        let metadata = fs::read_to_string(&metadata_path).unwrap_or_default();
        
        let stored_hash = metadata.lines()
            .find(|l| l.starts_with("hash = "))
            .and_then(|l| l.split('"').nth(1))
            .unwrap_or("");
        
        let stored_version = metadata.lines()
            .find(|l| l.starts_with("version = "))
            .and_then(|l| l.split('"').nth(1))
            .unwrap_or("");

        let tmp_build = Path::new("/tmp/radon/upgrade").join(&pkg);
        if tmp_build.exists() {
            fs::remove_dir_all(&tmp_build).unwrap_or_default();
        }
        fs::create_dir_all(&tmp_build).unwrap_or_default();

        let repo_url = metadata.lines()
            .find(|l| l.starts_with("repo_url = "))
            .and_then(|l| l.split('"').nth(1))
            .unwrap_or("");

        if repo_url.is_empty() {
            println!("{}: No repo URL for {}", Red.paint("Error"), pkg);
            continue;
        }

        let status = Command::new("git")
            .arg("clone")
            .arg("--depth=1")
            .arg(repo_url)
            .arg(&tmp_build)
            .status();

        if status.is_err() || !status.unwrap().success() {
            println!("{}: Failed to clone {}", Red.paint("Error"), pkg);
            continue;
        }

        let build_file_name = metadata.lines()
            .find(|l| l.starts_with("build_file = "))
            .and_then(|l| l.split('"').nth(1))
            .unwrap_or("");

        if build_file_name.is_empty() {
            println!("{}: No build file for {}", Red.paint("Error"), pkg);
            continue;
        }

        let build_file_path = tmp_build.join(build_file_name);
        if !build_file_path.exists() {
            println!("{}: Build file not found", Red.paint("Error"));
            continue;
        }

        let content = fs::read(&build_file_path).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let new_hash = format!("{:x}", hasher.finalize());

        let new_version = if build_file_name == "Cargo.toml" {
            let cargo_toml = fs::read_to_string(&build_file_path).unwrap_or_default();
            cargo_toml.lines()
                .find(|l| l.starts_with("version = "))
                .and_then(|l| l.split('"').nth(1))
                .unwrap_or("")
                .to_string()
        } else {
            "".to_string() 
        };

        let changed = new_hash != stored_hash || new_version != stored_version;
        
        if !changed {
            println!("{} is up to date", pkg);
            continue;
        }

        println!("\n{} update available for {}", Green.paint("NEW"), pkg);
        println!("Old version: {}", stored_version);
        println!("New version: {}", new_version);
        println!("Old hash: {}", stored_hash);
        println!("New hash: {}\n", new_hash);

        print!("Show diff? [y/N] ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().eq_ignore_ascii_case("y") {
            let _ = Command::new("diff")
                .arg("-u")
                .arg(buildfile_dir.join(build_file_name))
                .arg(&build_file_path)
                .status();
        }

        print!("\nUpgrade {}? [Y/n] ", pkg);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().eq_ignore_ascii_case("n") {
            println!("Skipping {}", pkg);
            continue;
        }

        println!("Reinstalling {}...", pkg);
        install(&pkg, None, false, None, None, &[]);

        let _ = Command::new(&utils::get_privilege_command())
            .arg("cp")
            .arg("-r")
            .arg(&tmp_build)
            .arg(&buildfile_dir)
            .status();
    }
}
