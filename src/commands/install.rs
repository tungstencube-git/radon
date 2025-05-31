use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::{self, Write};
use std::time::Instant;
use ansi_term::Colour::{Green, Red, Yellow};
use toml::{Value, map::Map};
use sha2::{Sha256, Digest};
use toml::Table;
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

    let (source_str, domain) = match source {
        Some("gitlab") => ("gitlab", "gitlab.com"),
        Some("codeberg") => ("codeberg", "codeberg.org"),
        _ => ("github", "github.com")
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
    } else if build_dir.join("meson.build").exists() {
        ("meson", vec!["meson".to_string(), "ninja".to_string()])
    } else if build_dir.join("build.ninja").exists() {
        ("ninja", vec!["ninja".to_string()])
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
        "meson" => Green.paint("Meson"),
        "ninja" => Green.paint("Ninja"),
        _ => unreachable!()
    });

    let build_file = match build_system {
        "make" => makefiles.iter().find(|f| build_dir.join(f).exists()).map(|f| f.to_string()),
        "cargo" => Some("Cargo.toml".to_string()),
        "cmake" => Some("CMakeLists.txt".to_string()),
        "meson" => Some("meson.build".to_string()),
        "ninja" => Some("build.ninja".to_string()),
        _ => None,
    };

    if let Some(file) = &build_file {
        let file_path = build_dir.join(file);
        if file_path.exists() {
            println!("~> Showing build file: {}", file);
            let status = Command::new("less")
                .arg(&file_path)
                .status();

            if status.is_err() || !status.unwrap().success() {
                let _ = Command::new("cat")
                    .arg(&file_path)
                    .status();
            }

            print!("~> Proceed with build? [Y/n] ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            if input.trim().eq_ignore_ascii_case("n") {
                println!("{}", Yellow.paint("Build cancelled by user"));
                return;
            }
        }
    }

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
        "meson" => {
            let meson_build_dir = build_dir.join("build");
            fs::create_dir_all(&meson_build_dir).expect("Failed to create build dir");

            let meson_status = Command::new("meson")
                .arg("setup")
                .arg(&meson_build_dir)
                .current_dir(&build_dir)
                .stdout(Stdio::null())
                .status();

            if meson_status.is_err() || !meson_status.as_ref().unwrap().success() {
                Command::new("meson")
                    .arg(&meson_build_dir)
                    .current_dir(&build_dir)
                    .stdout(Stdio::null())
                    .status()
                    .expect("Meson setup failed");
            }

            Command::new("ninja")
                .arg("-C")
                .arg(&meson_build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("Ninja build failed")
        }
        "ninja" => {
            Command::new("ninja")
                .current_dir(&build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("Ninja build failed")
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

    if !local {
        println!("{}", Yellow.paint("WARNING: Installing system-wide"));
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
            .arg(format!("echo '{}' >> /etc/radon/installed", repo))
            .status()
            .expect("Failed to update package list");
    }

    if !local && build_file.is_some() {
        let buildfile_dir = Path::new("/var/lib/radon/buildfiles").join(repo);
        let status = Command::new(&utils::get_privilege_command())
            .arg("mkdir")
            .arg("-p")
            .arg(&buildfile_dir)
            .status();
        
        if status.is_ok() && status.unwrap().success() {
            let status = Command::new(&utils::get_privilege_command())
                .arg("cp")
                .arg("-r")
                .arg(&build_dir)
                .arg(&buildfile_dir)
                .status();

            if status.is_ok() && status.unwrap().success() {
                let mut metadata = format!("repo_url = \"https://{}/{}\"\n", domain, package);
                metadata += &format!("build_file = \"{}\"\n", build_file.as_ref().unwrap());

                if let Some(bf) = &build_file {
                    let content = fs::read(build_dir.join(bf)).unwrap_or_default();
                    let mut hasher = Sha256::new();
                    hasher.update(&content);
                    let hash = format!("{:x}", hasher.finalize());
                    metadata += &format!("hash = \"{}\"\n", hash);
                }

                if build_system == "cargo" {
                    let cargo_toml = fs::read_to_string(build_dir.join("Cargo.toml")).unwrap_or_default();
                    if let Some(version) = cargo_toml.lines().find(|l| l.starts_with("version = ")) {
                        metadata += &format!("version = \"{}\"\n", version.split('"').nth(1).unwrap_or(""));
                    }
                }

                let temp_meta = Path::new("/tmp").join(format!("{}-metadata.toml", repo));
                fs::write(&temp_meta, metadata).unwrap_or_default();
                
                let _ = Command::new(&utils::get_privilege_command())
                    .arg("mv")
                    .arg(&temp_meta)
                    .arg(buildfile_dir.join("metadata.toml"))
                    .status();
            }
        }
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

fn get_cargo_binary_name(build_dir: &Path) -> Option<String> {
    let cargo_toml = build_dir.join("Cargo.toml");
    let content = match fs::read_to_string(&cargo_toml) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let value: Table = match content.parse() {
        Ok(v) => v,
        Err(_) => return None,
    };

    if let Some(bins) = value.get("bin") {
        if let Some(bin_array) = bins.as_array() {
            for bin_entry in bin_array {
                if let Some(bin_table) = bin_entry.as_table() {
                    if let Some(name) = bin_table.get("name") {
                        if let Some(name_str) = name.as_str() {
                            return Some(name_str.to_string());
                        }
                    }
                }
            }
        }
    }

    if let Some(package) = value.get("package") {
        if let Some(package_table) = package.as_table() {
            if let Some(name) = package_table.get("name") {
                if let Some(name_str) = name.as_str() {
                    return Some(name_str.to_string());
                }
            }
        }
    }

    None
}

fn find_binary_path(build_dir: &Path, repo: &str, build_system: &str) -> Option<PathBuf> {
    match build_system {
        "cargo" => {
            let binary_name = get_cargo_binary_name(build_dir).unwrap_or_else(|| repo.to_string());
            let release_path = build_dir.join("target/release").join(&binary_name);
            
            if release_path.exists() {
                return Some(release_path);
            }
            
            let debug_path = build_dir.join("target/debug").join(&binary_name);
            if debug_path.exists() {
                return Some(debug_path);
            }
            
            None
        },
        "make" => {
            let path = build_dir.join(repo);
            if path.exists() { Some(path) } else { None }
        },
        "cmake" => {
            let path = build_dir.join("build").join(repo);
            if path.exists() { Some(path) } else { None }
        },
        "meson" => {
            let build_output_dir = build_dir.join("build");
            find_executable_in_dir(&build_output_dir, repo)
        },
        "ninja" => {
            let path = build_dir.join(repo);
            if path.exists() { Some(path) } else { None }
        },
        _ => None
    }
}

fn find_executable_in_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if let Some(exec) = find_executable_in_dir(&path, name) {
                    return Some(exec);
                }
            } else if path.is_file() {
                if let Some(filename) = path.file_name() {
                    if filename == name {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
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
