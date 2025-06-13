use reqwest::blocking::Client;
use reqwest::header;
use serde_json::Value;
use comfy_table::{Table, ContentArrangement};
use comfy_table::presets::UTF8_FULL;

pub fn search(query: &str) {
    let url = format!("https://api.github.com/search/repositories?q={}",
                     urlencoding::encode(query));

    let client = Client::new();
    let response = client.get(&url)
        .header(header::USER_AGENT, "radon-pkg-manager")
        .send();

    let resp = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Failed to access GitHub API: {}", e);
            return;
        }
    };

    if !resp.status().is_success() {
        eprintln!("GitHub API error: {} - {}",
                 resp.status(),
                 resp.text().unwrap_or_default());
        return;
    }

    let json: Value = match resp.json() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse GitHub response: {}", e);
            return;
        }
    };

    if let Some(items) = json["items"].as_array() {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Package", "Stars", "Forks", "Source"]);

        for item in items.iter().take(10) {
            if let Some(name) = item["full_name"].as_str() {
                let stars = item["stargazers_count"].as_u64().unwrap_or(0);
                let forks = item["forks_count"].as_u64().unwrap_or(0);
                
                table.add_row(vec![
                    name,
                    &stars.to_string(),
                    &forks.to_string(),
                    "GitHub"
                ]);
            }
        }
        
        println!("{}", table);
    } else {
        eprintln!("Unexpected GitHub API response format");
        if let Some(message) = json["message"].as_str() {
            eprintln!("GitHub says: {}", message);
        }
    }
}
