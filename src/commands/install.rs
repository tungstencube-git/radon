use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::{self, Write};
use std::time::Instant;
use std::collections::HashMap;
use ansi_term::Colour::{Green, Red, Yellow};
use toml::{Value, map::Map};
use sha2::{Sha256, Digest};
use toml::Table;
use serde::{Serialize, Deserialize};
use crate::utils;

#[derive(Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub source: Option<String>,
    pub build_system: String,
    pub location: String,
    pub build_file: Option<String>,
    pub hash: Option<String>,
    pub version: Option<String>,
}

pub fn install(
    packages: &[String],
    gitlab: bool,
    codeberg: bool,
    local: bool,
    branch: Option<&str>,
    patches: Option<&Path>,
    flags: &[String],
    yes: bool,
) {
    for package in packages {
        install_single(package, gitlab, codeberg, local, branch, patches, flags, yes);
    }
}

pub fn install_single(
    package: &str,
    gitlab: bool,
    codeberg: bool,
    local: bool,
    branch: Option<&str>,
    patches: Option<&Path>,
    flags: &[String],
    yes: bool,
) {
    if !gitlab && !codeberg && !local && !package.contains('/') {
        install_aur(package, local, flags, yes);
        return;
    }
    let start = Instant::now();
    let tmp = Path::new("/tmp/radon");
    let builds = tmp.join("builds");
    let etc = Path::new("/etc/radon");

    for dir in [tmp, &builds] {
        if !dir.exists() {
            fs::create_dir_all(dir).expect("Failed to create temp directory");
        }
    }

    let source = if codeberg {
        Some("codeberg")
    } else if gitlab {
        Some("gitlab")
    } else {
        None
    };
    
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

    println!("\x1b[1m~> Cloning repository: {}\x1b[0m", package);
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

    build_and_install_from_source(&build_dir, repo, source, local, flags, yes, start);
}

fn install_aur(package: &str, local: bool, flags: &[String], yes: bool) {
    let start = Instant::now();
    let aur_builds = Path::new("/tmp/radon/aur");
    let build_dir = aur_builds.join(package);
    
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir).expect("Failed to clean previous build");
    }
    fs::create_dir_all(&build_dir).expect("Failed to create build directory");
    
    let pkgbuild_path = build_dir.join("PKGBUILD");
    let pkgbuild_url = format!("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}", package);
    
    println!("{}", Yellow.paint("~> Downloading PKGBUILD"));
    let status = Command::new("curl")
        .arg("-fL")
        .arg("--output")
        .arg(&pkgbuild_path)
        .arg(&pkgbuild_url)
        .status()
        .expect("Failed to download PKGBUILD");
    
    if !status.success() {
        eprintln!("{}", Red.paint("Failed to download PKGBUILD"));
        return;
    }
    
    let content = fs::read_to_string(&pkgbuild_path).unwrap_or_default();
    let mut variables = parse_pkgbuild_variables(&content);
    variables.entry("pkgname".to_string()).or_insert_with(|| package.to_string());
    
    let source_urls = extract_source_urls(&content, &variables);
    let src_url = source_urls.into_iter()
        .find(|url| !url.is_empty())
        .or_else(|| {
            content.lines()
                .find(|line| line.trim().starts_with("url="))
                .and_then(|line| {
                    let url_part = line.trim_start_matches("url=")
                        .trim_matches(|c| c == '"' || c == '\'');
                    let expanded = expand_variables(url_part, &variables);
                    let normalized = normalize_git_url(&expanded);
                    if !normalized.is_empty() {
                        Some(normalized)
                    } else {
                        None
                    }
                })
        });
    
    let src_url = match src_url {
        Some(url) => url,
        None => {
            eprintln!("{}", Red.paint("No valid git url found in PKGBUILD"));
            return;
        }
    };
    
    let repo_name = src_url.split('/')
        .last()
        .unwrap_or("repo")
        .trim_end_matches(".git");
    
    let src_dir = build_dir.join(repo_name);
    if src_dir.exists() {
        fs::remove_dir_all(&src_dir).expect("Failed to remove old source dir");
    }
    
    println!("{}", Yellow.paint(&format!("~> Cloning source repository: {}", src_url)));
    let status = Command::new("git")
        .arg("clone")
        .arg("--depth=1")
        .arg(&src_url)
        .arg(&src_dir)
        .status()
        .expect("Git clone failed");
    
    if !status.success() {
        eprintln!("{}", Red.paint("Failed to clone source repository"));
        return;
    }
    
    build_and_install_from_source(&src_dir, package, Some("aur"), local, flags, yes, start);
}

fn build_and_install_from_source(
    build_dir: &Path,
    repo: &str,
    source: Option<&str>,
    local: bool,
    flags: &[String],
    yes: bool,
    start: Instant,
) {
    println!("\x1b[1m~> Searching for build file\x1b[0m");
    let makefiles = ["Makefile", "makefile", "GNUMakefile"];
    let has_makefile = makefiles.iter().any(|f| build_dir.join(f).exists());

    let radon_json_path = build_dir.join("radon.json");
    let (build_system, mut deps, custom_flags) = if radon_json_path.exists() {
        parse_radon_json(&radon_json_path)
    } else if has_makefile {
        ("make".to_string(), parse_make_deps(&build_dir, &makefiles), vec![])
    } else if build_dir.join("configure").exists() {
        ("autotools".to_string(), parse_autotools_deps(&build_dir), vec![])
    } else if build_dir.join("Cargo.toml").exists() {
        ("cargo".to_string(), parse_cargo_deps(&build_dir), vec![])
    } else if build_dir.join("CMakeLists.txt").exists() {
        ("cmake".to_string(), vec!["cmake".to_string()], vec![])
    } else if build_dir.join("meson.build").exists() {
        ("meson".to_string(), vec!["meson".to_string(), "ninja".to_string()], vec![])
    } else if build_dir.join("build.ninja").exists() {
        ("ninja".to_string(), vec!["ninja".to_string()], vec![])
    } else if build_dir.join("*.nimble").exists() {
        ("nimble".to_string(), vec!["nim".to_string(), "nimble".to_string()], vec![])
    } else if build_dir.join("stack.yaml").exists() {
        ("stack".to_string(), vec!["stack".to_string()], vec![])
    } else {
        eprintln!("{}", Red.paint("No build system found"));
        return;
    };

    let mut final_flags = custom_flags;
    final_flags.extend(flags.iter().cloned());

    println!("~> Build system: {}", match build_system.as_str() {
        "make" => Green.paint("Make"),
        "autotools" => Green.paint("Autotools"),
        "cargo" => Green.paint("Cargo"),
        "cmake" => Green.paint("CMake"),
        "meson" => Green.paint("Meson"),
        "ninja" => Green.paint("Ninja"),
        "nimble" => Green.paint("Nimble"),
        "stack" => Green.paint("Stack"),
        _ => unreachable!()
    });

    let build_file = match build_system.as_str() {
        "make" => makefiles.iter().find(|f| build_dir.join(f).exists()).map(|f| f.to_string()),
        "autotools" => Some("configure".to_string()),
        "cargo" => Some("Cargo.toml".to_string()),
        "cmake" => Some("CMakeLists.txt".to_string()),
        "meson" => Some("meson.build".to_string()),
        "ninja" => Some("build.ninja".to_string()),
        "nimble" => build_dir.join("*.nimble").exists().then(|| "*.nimble".to_string()),
        "stack" => Some("stack.yaml".to_string()),
        _ => None,
    };

    if !yes {
        if let Some(file) = &build_file {
            let file_path = if file == "*.nimble" {
                let nimble_files: Vec<_> = fs::read_dir(&build_dir)
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().map(|e| e == "nimble").unwrap_or(false))
                    .collect();
                if nimble_files.is_empty() {
                    None
                } else {
                    nimble_files.get(0).cloned()
                }
            } else {
                Some(build_dir.join(file))
            };

            if let Some(file_path) = file_path {
                if file_path.exists() {
                    println!("~> Build file: {}", file);
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
        }
    }

    utils::check_deps(&deps);

    println!("~> Building with flags: {:?}", final_flags);
    let build_status = match build_system.as_str() {
        "make" => {
            let makefile = makefiles.iter()
                .find(|f| build_dir.join(f).exists())
                .unwrap_or(&"Makefile");

            let mut cmd = Command::new("make");
            cmd.arg("-f").arg(makefile)
                .args(&final_flags)
                .current_dir(&build_dir)
                .stdout(Stdio::null());

            cmd.status().expect("Make command failed")
        }
        "autotools" => {
            let configure_status = Command::new("./configure")
                .args(&final_flags)
                .current_dir(&build_dir)
                .status()
                .expect("Configure command failed");
            
            if !configure_status.success() {
                eprintln!("{}", Red.paint("Configure failed"));
                return;
            }
            
            Command::new("make")
                .current_dir(&build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("Make command failed")
        }
        "cmake" => {
            let cmake_build_dir = build_dir.join("build");
            fs::create_dir_all(&cmake_build_dir).expect("Failed to create build dir");

            let mut cmake_cmd = Command::new("cmake");
            cmake_cmd
                .arg("-DCMAKE_BUILD_TYPE=Release")
                .args(&final_flags)
                .arg("..")
                .current_dir(&cmake_build_dir)
                .stdout(Stdio::null());

            let cmake_status = cmake_cmd.status();

            if cmake_status.is_err() || !cmake_status.as_ref().unwrap().success() {
                Command::new("cmake")
                    .args(&final_flags)
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
                .args(&final_flags)
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
                .args(&final_flags)
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
                .args(&final_flags)
                .current_dir(&build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("Ninja build failed")
        }
        "nimble" => {
            Command::new("nimble")
                .arg("build")
                .args(&final_flags)
                .current_dir(&build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("Nimble command failed")
        }
        "stack" => {
            Command::new("stack")
                .arg("install")
                .args(&final_flags)
                .arg("--local-bin-path")
                .arg(build_dir.join("bin"))
                .current_dir(&build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("Stack command failed")
        }
        _ => unreachable!()
    };

    if !build_status.success() {
        eprintln!("{}", Red.paint("Build failed"));
        return;
    }

    println!("~> Installing...");
    let bin_path = find_binary_path(&build_dir, repo, &build_system);

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

    let bin_name = bin_path.file_name().unwrap().to_str().unwrap();
    let dest_path = dest.join(bin_name);

    if local {
        fs::copy(&bin_path, &dest_path)
            .expect("Failed to copy binary to local directory");
    } else {
        Command::new(&utils::get_privilege_command())
            .arg("install")
            .arg("-m755")
            .arg(&bin_path)
            .arg(&dest_path)
            .status()
            .expect("Installation failed");
    }

    if !local {
        let mut installed = utils::get_installed_packages();
        
        let mut hasher = Sha256::new();
        if let Some(bf) = &build_file {
            if let Ok(content) = fs::read(build_dir.join(bf)) {
                hasher.update(&content);
            }
        }
        let hash = format!("{:x}", hasher.finalize());
        
        let mut version = None;
        if build_system == "cargo" {
            if let Ok(cargo_toml) = fs::read_to_string(build_dir.join("Cargo.toml")) {
                if let Some(v) = cargo_toml.lines().find(|l| l.starts_with("version = ")) {
                    version = v.split('"').nth(1).map(|s| s.to_string());
                }
            }
        }
        
        let pkg = InstalledPackage {
            name: repo.to_string(),
            source: source.map(|s| s.to_string()),
            build_system: build_system.to_string(),
            location: dest_path.to_string_lossy().to_string(),
            build_file: build_file.clone(),
            hash: Some(hash),
            version,
        };
        
        installed.push(pkg);
        
        let temp_path = Path::new("/tmp").join("radon-installed.yaml");
        fs::write(&temp_path, serde_yaml::to_string(&installed).unwrap()).unwrap();
        
        Command::new(&utils::get_privilege_command())
            .arg("mv")
            .arg(&temp_path)
            .arg("/etc/radon/installed.yaml")
            .status()
            .expect("Failed to update package list");
    }

    println!("{} in {}s", Green.paint("~> INSTALL FINISHED"), start.elapsed().as_secs());

    if !local {
        println!(
            "{}",
            Yellow.paint(
                "Warning: radon installs packages to /usr/local/bin by default.\n\
                If /usr/local/bin is not in your $PATH, you may need to add it."
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
        "make" | "autotools" | "ninja" => {
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
        "nimble" => {
            let path = build_dir.join(repo);
            if path.exists() { Some(path) } else { None }
        },
        "stack" => {
            let bin_dir = build_dir.join("bin");
            if bin_dir.exists() {
                find_executable_in_dir(&bin_dir, repo)
            } else {
                None
            }
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

fn parse_autotools_deps(dir: &Path) -> Vec<String> {
    let configure = fs::read_to_string(dir.join("configure")).unwrap_or_default();
    let mut deps = Vec::new();
    if configure.contains("PKG_CHECK_MODULES") {
        deps.push("pkg-config".to_string());
    }
    if configure.contains("AC_PROG_CC") {
        deps.push("gcc".to_string());
    }
    if configure.contains("AC_PROG_CXX") {
        deps.push("g++".to_string());
    }
    deps
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

fn parse_radon_json(path: &Path) -> (String, Vec<String>, Vec<String>) {
    let file = std::fs::File::open(path).expect("Failed to open radon.json");
    let reader = std::io::BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).expect("Invalid radon.json");

    let build_system = json["build_system"]
        .as_str()
        .unwrap_or("make")
        .to_string();

    let deps = json["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let flags = json["flags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    (build_system, deps, flags)
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

fn parse_pkgbuild_variables(content: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos+1..].trim();
            let cleaned_value = value.trim_matches(|c| c == '"' || c == '\'' || c == '(' || c == ')').to_string();
            variables.insert(key.to_string(), cleaned_value);
        }
    }
    variables
}

fn expand_variables(text: &str, variables: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start+2..start+end];
            if let Some(value) = variables.get(var_name) {
                result.replace_range(start..start+end+1, value);
            } else {
                result.replace_range(start..start+end+1, "");
            }
        } else { break; }
    }
    let mut chars: Vec<char> = result.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i+1 < chars.len() && chars[i+1].is_alphabetic() {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
            let var_name: String = chars[start+1..i].iter().collect();
            if let Some(value) = variables.get(&var_name) {
                let replacement: Vec<char> = value.chars().collect();
                chars.splice(start..i, replacement);
                i = start + value.len();
            }
        } else { i += 1; }
    }
    chars.into_iter().collect()
}

fn normalize_git_url(url: &str) -> String {
    let url = url.trim();
    if url.contains(".tar.gz") || url.contains(".zip") || url.contains("archive/") {
        return String::new();
    }
    if url.starts_with("https://github.com/") || 
       url.starts_with("https://gitlab.com/") || 
       url.starts_with("https://codeberg.org/") {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 5 {
            let base_url = format!("{}/{}/{}/{}/{}", parts[0], parts[1], parts[2], parts[3], parts[4]);
            return if !base_url.ends_with(".git") {
                format!("{}.git", base_url)
            } else {
                base_url
            };
        }
    }
    if url.starts_with("git://") || url.ends_with(".git") {
        return url.to_string();
    }
    String::new()
}

fn extract_source_urls(content: &str, variables: &HashMap<String, String>) -> Vec<String> {
    let mut urls = Vec::new();
    let mut in_source_array = false;
    let mut paren_count = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("source=") {
            in_source_array = true;
            let after_equals = &line[7..];
            if after_equals.trim_start().starts_with('(') {
                paren_count = 1;
                let content = after_equals.trim_start().trim_start_matches('(');
                process_source_line(content, &mut urls, variables, &mut paren_count, &mut in_source_array);
            } else {
                let url = expand_variables(after_equals.trim_matches(|c| c == '"' || c == '\''), variables);
                let normalized = normalize_git_url(&url);
                if !normalized.is_empty() { urls.push(normalized); }
                in_source_array = false;
            }
        } else if in_source_array {
            process_source_line(line, &mut urls, variables, &mut paren_count, &mut in_source_array);
        }
    }
    urls
}

fn process_source_line(line: &str, urls: &mut Vec<String>, variables: &HashMap<String, String>, paren_count: &mut i32, in_source_array: &mut bool) {
    let mut current_line = line;
    while let Some(quote_start) = current_line.find(|c| c == '"' || c == '\'') {
        let quote_char = current_line.chars().nth(quote_start).unwrap();
        if let Some(quote_end) = current_line[quote_start+1..].find(quote_char) {
            let url_candidate = &current_line[quote_start+1..quote_start+1+quote_end];
            let expanded_url = expand_variables(url_candidate, variables);
            let normalized = normalize_git_url(&expanded_url);
            if !normalized.is_empty() { urls.push(normalized); }
            current_line = &current_line[quote_start+1+quote_end+1..];
        } else { break; }
    }
    *paren_count += line.chars().filter(|&c| c == '(').count() as i32;
    *paren_count -= line.chars().filter(|&c| c == ')').count() as i32;
    if *paren_count <= 0 { *in_source_array = false; }
}
