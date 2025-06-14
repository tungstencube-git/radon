use crate::commands::install::install_single;
use crate::utils;
use ansi_term::Colour::{Green, Red, Yellow};
use sha2::{Sha256, Digest};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub fn upgrade(package: Option<&str>, branch: Option<&str>, yes: bool) {
    let packages: Vec<String> = if let Some(pkg) = package {
        vec![pkg.to_string()]
    } else {
        utils::get_installed_packages()
            .iter()
            .map(|p| p.name.clone())
            .collect()
    };

    if packages.is_empty() {
        println!("No packages to upgrade");
        return;
    }

    if package.is_none() {
        println!(
            "{}", 
            Yellow.paint("WARNING: You are upgrading ALL packages. This may cause system instability!")
        );
        if !yes {
            print!("Continue? [y/N] ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("{}", Yellow.paint("Upgrade cancelled"));
                return;
            }
        }
    } else {
        println!(
            "{}", 
            Yellow.paint(&format!("WARNING: You are upgrading package: {}", package.unwrap()))
        );
        if !yes {
            print!("Continue? [y/N] ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("{}", Yellow.paint("Upgrade cancelled"));
                return;
            }
        }
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

        let stored_branch = metadata.lines()
            .find(|l| l.starts_with("branch = "))
            .and_then(|l| l.split('"').nth(1))
            .map(|s| s.to_string());

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

        let mut branch_to_use = branch.map(|s| s.to_string()).or(stored_branch);
        
        if branch_to_use.is_none() {
            println!("{}", Yellow.paint("No branch specified in metadata or command"));
            let branches = get_remote_branches(repo_url);
            if branches.is_empty() {
                println!("{}: Failed to get branches for {}", Red.paint("Error"), pkg);
                continue;
            }
            
            println!("Available branches:");
            for (i, b) in branches.iter().enumerate() {
                println!("{}. {}", i + 1, b);
            }
            
            print!("Select branch to upgrade (number): ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            
            if let Ok(num) = input.trim().parse::<usize>() {
                if num > 0 && num <= branches.len() {
                    branch_to_use = Some(branches[num - 1].clone());
                } else {
                    println!("{}: Invalid selection", Red.paint("Error"));
                    continue;
                }
            } else {
                println!("{}: Invalid input", Red.paint("Error"));
                continue;
            }
        }

        let status = Command::new("git")
            .arg("clone")
            .arg("--depth=1")
            .arg("--branch")
            .arg(branch_to_use.as_ref().unwrap())
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

        if !yes {
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
        }

        println!("Reinstalling {}...", pkg);
        install_single(&pkg, false, false, false, branch_to_use.as_deref(), None, &[], yes);

        let _ = Command::new(&utils::get_privilege_command())
            .arg("cp")
            .arg("-r")
            .arg(&tmp_build)
            .arg(&buildfile_dir)
            .status();
    }
}

fn get_remote_branches(repo_url: &str) -> Vec<String> {
    let output = match Command::new("git")
        .arg("ls-remote")
        .arg("--heads")
        .arg(repo_url)
        .output() {
            Ok(output) => output,
            Err(_) => return Vec::new(),
        };
    
    if !output.status.success() {
        return Vec::new();
    }
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    output_str
        .lines()
        .filter_map(|line| {
            line.split_whitespace().nth(1).and_then(|r| {
                r.strip_prefix("refs/heads/").map(|s| s.to_string())
            })
        })
        .collect()
}
