use reqwest::blocking::get;
use serde_json::Value;
use ansi_term::Style;

pub fn search(query: &str) {
    let resp: Value = get(&format!(
        "https://api.github.com/search/repositories?q={}",
        query
    ))
    .unwrap_or_else(|_| panic!("Failed to search for {}", query))
    .json()
    .unwrap_or_default();

    let bold = Style::new().bold();

    for item in resp["items"].as_array().unwrap_or(&vec![]) {
        if let Some(name) = item["full_name"].as_str() {
            println!("{}", bold.paint(name));
        }
    }
}
