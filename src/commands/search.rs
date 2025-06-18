use reqwest::blocking::Client;
use reqwest::header;
use serde_json::Value;
use ansi_term::Colour::{Green, Red};
use std::time::Duration;

pub fn search(query: &str) {
    println!("{}", Green.paint("Searching AUR and GitHub..."));

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    search_aur(query, &client);
    search_github(query, &client);
}

fn search_aur(query: &str, client: &Client) {
    let url = format!("https://aur.archlinux.org/rpc/?v=5&type=search&arg={}", 
                     urlencoding::encode(query));

    let response = client.get(&url)
        .header(header::USER_AGENT, "radon-pkg-manager")
        .send();

    let resp = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("{}: Failed to access AUR API: {}", Red.paint("Error"), e);
            return;
        }
    };

    if !resp.status().is_success() {
        eprintln!("{}: AUR API error: {} - {}", 
                 Red.paint("Error"),
                 resp.status(),
                 resp.text().unwrap_or_default());
        return;
    }

    let json: Value = match resp.json() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: Failed to parse AUR response: {}", Red.paint("Error"), e);
            return;
        }
    };

    if let Some(results) = json["results"].as_array() {
        println!("\n{}", Green.paint("AUR Packages:"));
        for res in results.iter().take(20) {
            if let Some(name) = res["Name"].as_str() {
                let votes = res["NumVotes"].as_u64().unwrap_or(0);
                let maintainer = res["Maintainer"].as_str().unwrap_or("Unknown");
                println!("  {} - {} - {} votes", name, maintainer, votes);
            }
        }
    }
}

fn search_github(query: &str, client: &Client) {
    let url = format!("https://api.github.com/search/repositories?q={}",
                     urlencoding::encode(query));

    let response = client.get(&url)
        .header(header::USER_AGENT, "radon-pkg-manager")
        .send();

    let resp = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("{}: Failed to access GitHub API: {}", Red.paint("Error"), e);
            return;
        }
    };

    if !resp.status().is_success() {
        eprintln!("{}: GitHub API error: {} - {}",
                 Red.paint("Error"),
                 resp.status(),
                 resp.text().unwrap_or_default());
        return;
    }

    let json: Value = match resp.json() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: Failed to parse GitHub response: {}", Red.paint("Error"), e);
            return;
        }
    };

    if let Some(items) = json["items"].as_array() {
        println!("\n{}", Green.paint("GitHub Repositories:"));
        for item in items.iter().take(20) {
            if let Some(name) = item["full_name"].as_str() {
                let stars = item["stargazers_count"].as_u64().unwrap_or(0);
                let forks = item["forks_count"].as_u64().unwrap_or(0);
                let owner = item["owner"]["login"].as_str().unwrap_or("Unknown");
                println!("  {} - {} - {}★ - {} forks", name, owner, stars, forks);
            }
        }
    }
}
