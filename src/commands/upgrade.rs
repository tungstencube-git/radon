use crate::commands::install::install;
use crate::utils::{get_installed_packages, get_privilege_command};
use crate::cli::UpgradeTarget;
use ansi_term::Colour::{Green, Red, Yellow};
use sha2::{Sha256, Digest};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::env;
use std::time::Instant;

pub fn upgrade(target: UpgradeTarget) {
    match target {
        UpgradeTarget::All => upgrade_packages(true, None),
        UpgradeTarget::Package { package } => upgrade_packages(false, Some(package)),
        UpgradeTarget::SelfUpgrade => upgrade_self(),
    }
}

fn upgrade_self() {
    println!("{}", Yellow.paint("Upgrading radon..."));
    let start = Instant::now();
    let tmp = Path::new("/tmp/radon/self-upgrade");
    let repo_url = "https://github.com/plyght/spine";

    if tmp.exists() {
        fs::remove_dir_all(tmp).expect("Failed to clean temp dir");
    }
    fs::create_dir_all(tmp).expect("Failed to create temp dir");

    let clone_status = Command::new("git")
        .arg("clone")
        .arg("--depth=1")
        .arg(repo_url)
        .arg(tmp)
        .status()
        .expect("Git command failed");

    if !clone_status.success() {
        eprintln!("{}", Red.paint("Failed to clone radon repository"));
        return;
    }

    let build_status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(tmp.join("Cargo.toml"))
        .status()
        .expect("Cargo command failed");

    if !build_status.success() {
        eprintln!("{}", Red.paint("Failed to build radon"));
        return;
    }

    let current_exe = env::current_exe().expect("Failed to get current executable path");
    let new_bin = tmp.join("target/release/spn");

    if !new_bin.exists() {
        eprintln!("{}", Red.paint("Failed to find built binary"));
        return;
    }

    let privilege_cmd = get_privilege_command();
    let install_status = Command::new(&privilege_cmd)
        .arg("install")
        .arg("-m755")
        .arg(&new_bin)
        .arg(&current_exe)
        .status()
        .expect("Installation failed");

    if !install_status.success() {
        eprintln!("{}", Red.paint("Failed to install new binary"));
        return;
    }

    println!("{} in {}s", Green.paint("Radon upgraded successfully"), start.elapsed().as_secs());
}

fn upgrade_packages(all: bool, package: Option<String>) {
    let packages: Vec<String> = if all {
        get_installed_packages()
    } else if let Some(pkg) = package {
        vec![pkg]
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

        print!("\nUpgrade {}? [Y/n] ", pkg);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().eq_ignore_ascii_case("n") {
            println!("Skipping {}", pkg);
            continue;
        }

        println!("Reinstalling {}...", pkg);
        install(&pkg, None, false, None, None);

        let _ = Command::new(&get_privilege_command())
            .arg("cp")
            .arg("-r")
            .arg(&tmp_build)
            .arg(&buildfile_dir)
            .status();
    }
}
