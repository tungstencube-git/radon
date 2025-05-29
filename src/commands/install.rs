use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use ansi_term::Colour::{Green, Red, Yellow};
use toml::{Value, map::Map};
use crate::utils;

pub fn install(
    package: &str,
    source: Option<&str>,
    local: bool,
    branch: Option<&str>,
    patches: Option<&Path>,
) {
    let start = Instant::now();
    let tmp = Path::new("/tmp/radon");
    let builds = tmp.join("builds");
    let etc = Path::new("/etc/radon");

    for dir in [tmp, &builds] {
        if !dir.exists() {
            fs::create_dir_all(dir).expect("Failed to create temp directory");
        }
    }

    if !local && !etc.exists() {
        Command::new(&utils::get_privilege_command())
            .arg("mkdir")
            .arg("-p")
            .arg(etc)
            .status()
            .expect("Failed to create system directory");
    }

    let (source_str, domain) = match source {
        Some("gitlab") => ("gitlab", "gitlab.com"),
        Some("codeberg") => ("codeberg", "codeberg.org"),
        _ => {
            if source.is_none() {
                println!(
                    "{}: No source specified, falling back to GitHub",
                    Yellow.paint("Warning")
                );
            }
            ("github", "github.com")
        }
    };

    let repo = package.split('/').last().unwrap();
    let build_dir = builds.join(repo);

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir).expect("Failed to clean previous build");
    }

    println!("\x1b[1m~> Cloning repository\x1b[0m");
    let mut git_clone = Command::new("git");
    git_clone
        .arg("clone")
        .arg("--depth=1")
        .arg(format!("https://{}/{}", domain, package));

    if let Some(b) = branch {
        git_clone.arg("--branch").arg(b);
    }

    let status = git_clone
        .arg(&build_dir)
        .stdout(Stdio::null())
        .status()
        .expect("Git command failed");

    if !status.success() {
        eprintln!("{}", Red.paint("Failed to clone repository"));
        return;
    }

    if let Some(patches_dir) = patches {
        apply_patches(&build_dir, patches_dir);
    }

    println!("\x1b[1m~> Searching for build file\x1b[0m");
    let makefiles = ["Makefile", "makefile", "GNUMakefile"];
    let has_makefile = makefiles.iter().any(|f| build_dir.join(f).exists());

    let (build_system, mut deps) = if has_makefile {
        ("make", parse_make_deps(&build_dir, &makefiles))
    } else if build_dir.join("Cargo.toml").exists() {
        ("cargo", parse_cargo_deps(&build_dir))
    } else if build_dir.join("CMakeLists.txt").exists() {
        ("cmake", vec!["cmake".to_string()])
    } else {
        eprintln!("{}", Red.paint("No build system found"));
        return;
    };

    if build_system == "make" || build_system == "cmake" {
        deps.push("make".to_string());
    }

    println!("~> Build file is {}", match build_system {
        "make" => Green.paint("Make"),
        "cargo" => Green.paint("Cargo"),
        "cmake" => Green.paint("CMake"),
        _ => unreachable!()
    });

    utils::check_deps(&deps);

    println!("~> Building...");
    let build_status = match build_system {
        "make" => {
            let makefile = makefiles.iter()
                .find(|f| build_dir.join(f).exists())
                .unwrap_or(&"Makefile");

            Command::new("make")
                .arg("-f")
                .arg(makefile)
                .current_dir(&build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("Make command failed")
        }
        "cmake" => {
            let cmake_build_dir = build_dir.join("build");
            fs::create_dir_all(&cmake_build_dir).expect("Failed to create build dir");

            let cmake_status = Command::new("cmake")
                .arg("-DCMAKE_BUILD_TYPE=Release")
                .arg("..")
                .current_dir(&cmake_build_dir)
                .stdout(Stdio::null())
                .status();

            if cmake_status.is_err() || !cmake_status.as_ref().unwrap().success() {
                Command::new("cmake")
                    .arg("..")
                    .current_dir(&cmake_build_dir)
                    .stdout(Stdio::null())
                    .status()
                    .expect("CMake command failed")
            } else {
                cmake_status.unwrap()
            }
        }
        "cargo" => {
            let mut cargo_cmd = Command::new("cargo");
            cargo_cmd
                .arg("build")
                .arg("--release")
                .arg("--manifest-path")
                .arg(build_dir.join("Cargo.toml"))
                .arg("--target-dir")
                .arg(build_dir.join("target"))
                .current_dir(&build_dir)
                .stdout(Stdio::null());

            cargo_cmd.status().expect("Cargo command failed")
        }
        _ => unreachable!()
    };

    if !build_status.success() {
        eprintln!("{}", Red.paint("Build failed"));
        return;
    }

    println!("~> Installing...");
    let bin_name = format!("({}){}(radon)", source_str, repo);
    let bin_path = find_binary_path(&build_dir, repo, build_system);

    if bin_path.is_none() {
        eprintln!("{}: Failed to find built binary", Red.paint("Error"));
        return;
    }
    let bin_path = bin_path.unwrap();

    let dest = if local {
        let home = env::var("HOME").expect("HOME environment variable not set");
        let path = PathBuf::from(home).join(".local/bin");
        if !path.exists() {
            fs::create_dir_all(&path).expect("Failed to create local bin directory");
        }
        path
    } else {
        PathBuf::from("/usr/local/bin")
    };

    if !local && dest == Path::new("/usr/bin") {
        println!("{}", Yellow.paint("WARNING: Installing to /usr/bin may cause conflicts"));
    }

    if local {
        fs::copy(&bin_path, dest.join(&bin_name))
            .expect("Failed to copy binary to local directory");
    } else {
        Command::new(&utils::get_privilege_command())
            .arg("install")
            .arg("-m755")
            .arg(&bin_path)
            .arg(dest.join(&bin_name))
            .status()
            .expect("Installation failed");
    }

    if !local {
        Command::new(&utils::get_privilege_command())
            .arg("sh")
            .arg("-c")
            .arg(format!("echo {} >> /etc/radon/listinstalled", repo))
            .status()
            .expect("Failed to update package list");
    }

    println!("{} in {}s", Green.paint("~> INSTALL FINISHED"), start.elapsed().as_secs());

    if !local {
        println!(
            "{}",
            Yellow.paint(
                &format!("Warning: radon installs packages to /usr/local/bin by default.\n\
                If /usr/local/bin is not in your $PATH, you may need to add it.\n\
                Alternatively, you can move the installed binary manually:\n\
                  {} cp /usr/local/bin/{} /usr/bin\n\
                or\n\
                  doas cp /usr/local/bin/{} /usr/bin",
                utils::get_privilege_command(), bin_name, bin_name)
            )
        );
    } else {
        println!(
            "{}",
            Green.paint(
                "Installed to ~/.local/bin. Make sure this directory is in your PATH."
            )
        );
    }
}

fn find_binary_path(build_dir: &Path, repo: &str, build_system: &str) -> Option<PathBuf> {
    match build_system {
        "make" => {
            let path = build_dir.join(repo);
            if path.exists() { Some(path) } else { None }
        }
        "cargo" => {
            let target_dir = build_dir.join("target");
            if target_dir.exists() {
                let release_path = target_dir.join("release").join(repo);
                if release_path.exists() {
                    return Some(release_path);
                }

                if let Ok(entries) = fs::read_dir(&target_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_dir() {
                            let bin_path = path.join(repo);
                            if bin_path.exists() {
                                return Some(bin_path);
                            }

                            let release_bin = path.join("release").join(repo);
                            if release_bin.exists() {
                                return Some(release_bin);
                            }
                        }
                    }
                }
            }

            let debug_path = target_dir.join("debug").join(repo);
            if debug_path.exists() {
                return Some(debug_path);
            }

            None
        }
        "cmake" => {
            let path = build_dir.join("build").join(repo);
            if path.exists() { Some(path) } else { None }
        }
        _ => None
    }
}

fn parse_make_deps(dir: &Path, makefiles: &[&str]) -> Vec<String> {
    let found_file = makefiles.iter()
        .find(|f| dir.join(f).exists())
        .unwrap_or(&"Makefile");

    let makefile = fs::read_to_string(dir.join(found_file)).unwrap_or_default();
    makefile
        .lines()
        .find(|l| l.contains("# DEPENDENCIES:"))
        .map(|l| {
            l.split(':')
                .nth(1)
                .unwrap()
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_cargo_deps(dir: &Path) -> Vec<String> {
    let cargo_toml = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    let value = cargo_toml.parse::<Value>().unwrap_or(Value::Table(Map::new()));

    value.get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("radon"))
        .and_then(|r| r.get("dependencies"))
        .and_then(|d| d.as_array())
        .map(|deps| {
            deps.iter()
                .filter_map(|d| d.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn apply_patches(build_dir: &Path, patches_dir: &Path) {
    let patches: Vec<PathBuf> = fs::read_dir(patches_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "patch").unwrap_or(false))
        .collect();

    for patch in patches {
        println!("Applying patch: {}", patch.display());
        let status = Command::new("patch")
            .arg("-Np1")
            .arg("--directory")
            .arg(build_dir)
            .arg("--input")
            .arg(&patch)
            .status()
            .expect("Failed to apply patch");

        if !status.success() {
            eprintln!("{}: Failed to apply {}", Red.paint("Error"), patch.display());
        }
    }
}
