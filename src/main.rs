use std::{fs};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "haranalyze", version, about = "Analyze HAR files")]
struct Args {
    // Path to the HAR file
    path: PathBuf,
    // Show top N slowest requests
    #[arg(long, default_value_t = 10)]
    top: usize,
    // Output JSON (later)
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Debug, Deserialize)]
struct Har {
    log: HarLog,
}

#[derive(Debug, Deserialize)]
struct HarLog {
    entries: Vec<HarEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct HarEntry {
    time: f64,
    request: HarRequest,
    response: HarResponse,
}

#[derive(Debug, Deserialize, Clone)]
struct HarRequest {
    url: String,
}

#[derive(Debug, Deserialize, Clone)]
struct HarResponse {
    #[serde(default)]
    body_size: Option<i64>,
    #[serde(default)]
    headers_size: Option<i64>,
    #[serde(default)]
    content: Option<HarResponseContent>,
}

#[derive(Debug, Deserialize, Clone)]
struct HarResponseContent {
    #[serde(default)]
    size: Option<i64>,
}

fn pos_i64_to_u64(x: Option<i64>) -> u64 {
    match x {
        Some(v) if v > 0 => v as u64,
        _ => 0,
    }
}

fn format_bytes (n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;


    let nf = n as f64;

    if n < 1024 {
        format!("{} B", n)
    } else if nf < MB {
        format!("{:.2} KB", nf / KB)
    } else if nf < GB {
        format!("{:.2} MB", nf / MB)
    } else {
        format!("{:.2} GB", nf / GB)
    }
}

fn entry_bytes(e: &HarEntry) -> u64 {
    let r = &e.response;
    let body = pos_i64_to_u64(r.body_size)
        .max(pos_i64_to_u64(r.content.as_ref().and_then(|c| c.size)));
    let headers = pos_i64_to_u64(r.headers_size);
    body + headers
}

fn main() -> Result<()> {
    let args = Args::parse();

    let bytes = fs::read(&args.path)
        .with_context(|| format!("failed to read file: {}", args.path.display()))?;

    let har: Har = serde_json::from_slice(&bytes).with_context(|| "failed to parse HAR JSON")?;

    let total = har.log.entries.len();
    let total_time_ms: f64 = har.log.entries.iter().map(|e| e.time).sum();
    let total_bytes: u64 = har
        .log
        .entries
        .iter()
        .map(|e| {
            let r = &e.response;
            let body = pos_i64_to_u64(r.body_size)
                .max(pos_i64_to_u64(r.content.as_ref().and_then(|c| c.size)));

            let headers = pos_i64_to_u64(r.headers_size);

            body + headers
        })
        .sum();

    println!("entries: {}", total);
    println!("total_time_ms: {:.2}", total_time_ms);
    println!("total_bytes: {}", format_bytes(total_bytes));

    let mut by_time = har.log.entries.clone();
    by_time.sort_by(|a, b| {
        b.time
            .partial_cmp(&a.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_n = args.top.min(by_time.len());
    println!("\nslowest {}:", top_n);

    for e in by_time.into_iter().take(top_n) {
        println!("{:>8.2} ms {}", e.time, e.request.url);
    }

    let mut by_bytes = har.log.entries.clone();
     by_bytes.sort_by_key(|e| std::cmp::Reverse(entry_bytes(e)));

     let top_n = args.top.min(by_bytes.len());
     println!("\nlargest {} by bytes:", top_n);

     for e in by_bytes.iter().take(top_n) {
        println!(
            "{:>10}  {}",
            format_bytes(entry_bytes(e)),
            e.request.url
        );
    }

    Ok(())
}
