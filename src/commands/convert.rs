use std::fs;
use std::path::Path;
use std::io::{self, Write};
use toml::Value;
use serde_json::{Map, Value as JsonValue};

pub fn convert(file: Option<&Path>) {
    let path = match file {
        Some(p) => p.to_path_buf(),
        None => {
            let default = Path::new("Cargo.toml");
            if !default.exists() {
                println!("Cargo.toml not found in current directory");
                print!("Enter path to Cargo.toml: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                Path::new(input.trim()).to_path_buf()
            } else {
                default.to_path_buf()
            }
        }
    };

    if !path.exists() {
        eprintln!("Error: File not found - {}", path.display());
        return;
    }

    if path.file_name().and_then(|f| f.to_str()) != Some("Cargo.toml") {
        eprintln!("Error: Only Cargo.toml files are supported");
        return;
    }

    let cargo_toml = fs::read_to_string(&path).expect("Failed to read file");
    let value: Value = toml::from_str(&cargo_toml).expect("Invalid TOML format");

    let mut radon_json = Map::new();
    radon_json.insert("build_system".to_string(), JsonValue::String("cargo".to_string()));

    if let Some(package) = value.get("package") {
        if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
            radon_json.insert("name".to_string(), JsonValue::String(name.to_string()));
        }
    }

    if let Some(dependencies) = value.get("dependencies") {
        let deps: Vec<JsonValue> = dependencies
            .as_table()
            .unwrap()
            .keys()
            .map(|k| JsonValue::String(k.clone()))
            .collect();
        radon_json.insert("dependencies".to_string(), JsonValue::Array(deps));
    }

    let output_path = path.with_file_name("radon.json");
    let output_file = fs::File::create(&output_path).expect("Failed to create radon.json");
    serde_json::to_writer_pretty(output_file, &radon_json).expect("Failed to write JSON");
    println!("Created radon.json at {}", output_path.display());
}

