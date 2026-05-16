mod api;
mod bucket;
mod config;
mod error;
mod output;

use clap::Parser;

use config::{format_api_date, Config, QueryType};
use output::{write_output, OutputData};

fn main() {
    let raw = Config::parse();
    let cfg = match raw.validate() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("Run with --help for usage.");
            std::process::exit(1);
        }
    };

    let raw_entries = match cfg.query_type {
        QueryType::Current => api::fetch_current(&cfg.base_url),
        QueryType::Range => {
            let start_str = format_api_date(cfg.start_naive);
            let end_str = format_api_date(cfg.end_naive);
            api::fetch_range(&cfg.base_url, &start_str, &end_str)
        }
    };

    let raw_entries = match raw_entries {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let points = match bucket::parse_entries(raw_entries) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let output_data = if cfg.average {
        OutputData::Single(bucket::average_all(&points))
    } else {
        let buckets = bucket::bucket_by_interval(&points, cfg.start_utc, cfg.end_utc, cfg.interval);
        OutputData::Buckets { buckets, points }
    };

    use std::io::IsTerminal;
    let stdout = std::io::stdout();
    let use_color = stdout.is_terminal();
    if let Err(e) = write_output(&output_data, cfg.format, use_color, &mut stdout.lock()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
