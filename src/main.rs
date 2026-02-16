use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

mod har;
mod report;

#[derive(Parser, Debug)]
#[command(name = "perf_tool", version, about = "Analyze HAR files")]
struct Args {
    // Path to the HAR file
    path: PathBuf,
    // Show top N slowest requests
    #[arg(long, default_value_t = 10)]
    top: usize,
    // Output JSON
    #[arg(long, default_value_t = false)]
    json: bool,
    // Group request metrics by dimension
    #[arg(long, value_enum)]
    group_by: Option<report::GroupBy>,
}

fn render_text(report: &report::Report) {
    println!("entries: {}", report.entries);
    println!("total_time_ms: {:.2}", report.total_time_ms);
    println!("total_bytes: {}", report::format_bytes(report.total_bytes));

    println!("\nslowest {}:", report.top_returned);
    for row in &report.top_slowest {
        println!("{:>8.2} ms {}", row.time_ms, row.url);
    }

    println!("\nlargest {} by bytes:", report.top_returned);
    for row in &report.top_largest {
        println!("{:>10}  {}", report::format_bytes(row.bytes), row.url);
    }

    if report.group_by.is_some() {
        println!("\ngroups by host (top {}):", report.top_groups.len());
        for group in &report.top_groups {
            println!(
                "{:>4} req  {:>8.2} ms total  {:>8.2} ms avg  {:>8.2} ms p95  {:>10}  {}",
                group.count,
                group.total_time_ms,
                group.avg_time_ms,
                group.p95_time_ms,
                report::format_bytes(group.total_bytes),
                group.key
            );
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let bytes = fs::read(&args.path)
        .with_context(|| format!("failed to read file: {}", args.path.display()))?;

    let har = har::parse_har(&bytes)?;
    let report = report::build_report(&har.log.entries, args.top, args.group_by);

    if args.json {
        let out = serde_json::to_string_pretty(&report)
            .with_context(|| "failed to serialize JSON output")?;
        println!("{}", out);
    } else {
        render_text(&report);
    }

    Ok(())
}
