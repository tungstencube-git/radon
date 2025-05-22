use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use ansi_term::Colour::{Green, Red, Yellow};
use toml::{Value, map::Map};
use crate::utils::{check_deps, get_bin_path};

pub fn install(package: &str) {
    let start = Instant::now();
    let tmp = Path::new("/tmp/radon");
    let builds = tmp.join("builds");
    let etc = Path::new("/etc/radon");

    for dir in [tmp, &builds, etc] {
        if !dir.exists() {
            if dir == etc {
                Command::new("sudo")
                    .arg("mkdir")
                    .arg("-p")
                    .arg(dir)
                    .status()
                    .expect("Failed to create system directory");
            } else {
                fs::create_dir_all(dir).expect("Failed to create temp directory");
            }
        }
    }

    let repo = package.split('/').last().unwrap();
    let build_dir = builds.join(repo);

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir).expect("Failed to clean previous build");
    }

    println!("\x1b[1m~> Cloning repository\x1b[0m");
    let status = Command::new("git")
        .arg("clone")
        .arg(format!("https://github.com/{}", package))
        .arg(&build_dir)
        .stdout(Stdio::null())
        .status()
        .expect("Git command failed");

    if !status.success() {
        eprintln!("{}", Red.paint("Failed to clone"));
        return;
    }

    println!("\x1b[1m~> Searching for build file\x1b[0m");
    let (build_system, deps) = if build_dir.join("Makefile").exists() {
        ("make", parse_make_deps(&build_dir))
    } else if build_dir.join("Cargo.toml").exists() {
        ("cargo", parse_cargo_deps(&build_dir))
    } else if build_dir.join("CMakeLists.txt").exists() {
        ("cmake", vec!["cmake".to_string(), "make".to_string()])
    } else {
        eprintln!("{}", Red.paint("No build system found"));
        return;
    };

    println!("~> Build file is {}", match build_system {
        "make" => Green.paint("Make"),
        "cargo" => Green.paint("Cargo"),
        "cmake" => Green.paint("CMake"),
        _ => unreachable!()
    });

    check_deps(&deps);

    println!("~> Building...");
    let build_status = match build_system {
        "make" => Command::new("make")
            .current_dir(&build_dir)
            .stdout(Stdio::null())
            .status()
            .expect("Make command failed"),
        "cmake" => {
            let cmake_build_dir = build_dir.join("cmake-build-release");
            fs::create_dir_all(&cmake_build_dir).expect("Failed to create build dir");

            let cmake_status = Command::new("cmake")
                .arg("..")
                .current_dir(&cmake_build_dir)
                .stdout(Stdio::null())
                .status()
                .expect("CMake command failed");

            if !cmake_status.success() {
                cmake_status
            } else {
                Command::new("make")
                    .current_dir(&cmake_build_dir)
                    .stdout(Stdio::null())
                    .status()
                    .expect("Make command failed")
            }
        }
        "cargo" => Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&build_dir)
            .stdout(Stdio::null())
            .status()
            .expect("Cargo command failed"),
        _ => unreachable!()
    };

    if !build_status.success() {
        eprintln!("{}", Red.paint("Build failed"));
        return;
    }

    println!("~> Installing...");
    let bin_name = format!("{}(radon)", repo);
    let bin_path = match build_system {
        "make" => build_dir.join(repo),
        "cargo" => build_dir.join("target/release").join(repo),
        "cmake" => build_dir.join("cmake-build-release").join(repo),
        _ => unreachable!()
    };

    let dest = get_bin_path();
    if dest == Path::new("/usr/bin") {
        println!("{}", Yellow.paint("WARNING: Installing to /usr/bin may cause conflicts"));
    }

    Command::new("sudo")
        .arg("install")
        .arg("-m755")
        .arg(&bin_path)
        .arg(dest.join(&bin_name))
        .status()
        .expect("Installation failed");

    Command::new("sudo")
        .arg("sh")
        .arg("-c")
        .arg(format!("echo {} >> /etc/radon/listinstalled", repo))
        .status()
        .expect("Failed to update package list");

    println!("{} in {}s", Green.paint("~> INSTALL FINISHED"), start.elapsed().as_secs());
}

fn parse_make_deps(dir: &Path) -> Vec<String> {
    let makefile = fs::read_to_string(dir.join("Makefile")).unwrap_or_default();
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
    value["package"]["metadata"]["radon"]["dependencies"]
        .as_array()
        .map(|deps| {
            deps.iter()
                .filter_map(|d| d.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}
