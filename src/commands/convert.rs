use std::fs;
use std::path::Path;
use toml::Value;
use serde_json::{Map, Value as JsonValue};

pub fn convert(file: Option<&Path>) {
    let path = file.unwrap_or_else(|| Path::new("Cargo.toml"));
    if !path.exists() {
        eprintln!("{} not found", path.display());
        return;
    }
    let cargo_toml = fs::read_to_string(path).expect("Failed to read file");
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
    if let Some(build_dependencies) = value.get("build-dependencies") {
        let build_deps: Vec<JsonValue> = build_dependencies
            .as_table()
            .unwrap()
            .keys()
            .map(|k| JsonValue::String(k.clone()))
            .collect();
        radon_json.insert("build-dependencies".to_string(), JsonValue::Array(build_deps));
    }
    let output_path = Path::new("radon.json");
    let output_file = fs::File::create(output_path).expect("Failed to create radon.json");
    serde_json::to_writer_pretty(output_file, &radon_json).expect("Failed to write JSON");
    println!("Created radon.json");
}

