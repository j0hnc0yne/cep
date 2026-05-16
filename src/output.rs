use std::io::Write;

use serde::Serialize;

use crate::bucket::{Bucket, PricePoint};
use crate::config::OutputFormat;
use crate::error::CepError;

pub enum OutputData {
    Single(f64),
    Buckets {
        buckets: Vec<Bucket>,
        points: Vec<PricePoint>,
    },
}

#[derive(Serialize)]
struct SingleRecord {
    price: f64,
}

#[derive(Serialize)]
struct BucketRecord {
    start: String,
    end: String,
    average: f64,
    count: usize,
}

#[derive(Serialize)]
struct PointRecord {
    timestamp: String,
    price: f64,
}

impl From<&Bucket> for BucketRecord {
    fn from(b: &Bucket) -> Self {
        BucketRecord {
            start: b.start.to_rfc3339(),
            end: b.end.to_rfc3339(),
            average: b.average,
            count: b.count,
        }
    }
}

impl From<&PricePoint> for PointRecord {
    fn from(p: &PricePoint) -> Self {
        PointRecord {
            timestamp: p.timestamp.to_rfc3339(),
            price: p.price,
        }
    }
}

pub fn write_output(
    data: &OutputData,
    format: OutputFormat,
    color: bool,
    writer: &mut impl Write,
) -> Result<(), CepError> {
    match format {
        OutputFormat::Text => write_text(data, writer),
        OutputFormat::Json => write_json(data, color, writer),
        OutputFormat::Yaml => write_yaml(data, color, writer),
        OutputFormat::Csv => write_csv(data, writer),
    }
}

fn write_text(data: &OutputData, w: &mut impl Write) -> Result<(), CepError> {
    match data {
        OutputData::Single(price) => {
            writeln!(w, "Price: {:.4} ¢/kWh", price).map_err(|e| CepError::Output(e.to_string()))
        }
        OutputData::Buckets { buckets, points } => {
            writeln!(w, "Summary").map_err(|e| CepError::Output(e.to_string()))?;
            writeln!(w, "{:<32} {:<32} {:>12} {:>6}", "Start", "End", "Avg (¢/kWh)", "Count")
                .map_err(|e| CepError::Output(e.to_string()))?;
            writeln!(w, "{}", "-".repeat(86)).map_err(|e| CepError::Output(e.to_string()))?;
            for b in buckets {
                writeln!(
                    w,
                    "{:<32} {:<32} {:>12.4} {:>6}",
                    b.start.to_rfc3339(),
                    b.end.to_rfc3339(),
                    b.average,
                    b.count
                )
                .map_err(|e| CepError::Output(e.to_string()))?;
            }
            writeln!(w).map_err(|e| CepError::Output(e.to_string()))?;
            writeln!(w, "Points").map_err(|e| CepError::Output(e.to_string()))?;
            writeln!(w, "{:<32} {:>12}", "Timestamp", "Price (¢/kWh)")
                .map_err(|e| CepError::Output(e.to_string()))?;
            writeln!(w, "{}", "-".repeat(46)).map_err(|e| CepError::Output(e.to_string()))?;
            for p in points {
                writeln!(w, "{:<32} {:>12.4}", p.timestamp.to_rfc3339(), p.price)
                    .map_err(|e| CepError::Output(e.to_string()))?;
            }
            Ok(())
        }
    }
}

// ── JSON ──────────────────────────────────────────────────────────────────────

fn build_json_string(data: &OutputData) -> Result<String, CepError> {
    match data {
        OutputData::Single(price) => serde_json::to_string_pretty(&SingleRecord { price: *price })
            .map_err(|e| CepError::Output(e.to_string())),
        OutputData::Buckets { buckets, points } => {
            #[derive(Serialize)]
            struct BucketsOutput<'a> {
                summary: Vec<BucketRecord>,
                points: Vec<PointRecord>,
                #[serde(skip)]
                _marker: std::marker::PhantomData<&'a ()>,
            }
            let out = BucketsOutput {
                summary: buckets.iter().map(BucketRecord::from).collect(),
                points: points.iter().map(PointRecord::from).collect(),
                _marker: std::marker::PhantomData,
            };
            serde_json::to_string_pretty(&out).map_err(|e| CepError::Output(e.to_string()))
        }
    }
}

fn write_json(data: &OutputData, color: bool, w: &mut impl Write) -> Result<(), CepError> {
    let s = build_json_string(data)?;
    let output = if color {
        use colored_json::ToColoredJson;
        s.to_colored_json_auto().unwrap_or_else(|_| s)
    } else {
        s
    };
    writeln!(w, "{}", output).map_err(|e| CepError::Output(e.to_string()))
}

// ── YAML ──────────────────────────────────────────────────────────────────────

const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const WHITE: &str = "\x1b[37m";

fn ansi(code: &str, s: &str) -> String {
    format!("{}{}{}", code, s, RESET)
}

fn colorize_yaml_scalar(s: &str) -> String {
    if s == "null" || s == "~" {
        return ansi(DIM, s);
    }
    if s == "true" || s == "false" {
        return ansi(YELLOW, s);
    }
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        return ansi(GREEN, s);
    }
    if s.parse::<f64>().is_ok() {
        return ansi(YELLOW, s);
    }
    ansi(GREEN, s)
}

// Find the colon that separates a YAML key from its value:
// must be followed by a space or be at end-of-string.
fn find_yaml_key_colon(s: &str) -> Option<usize> {
    for (i, c) in s.char_indices() {
        if c == ':' {
            let after = &s[i + 1..];
            if after.is_empty() || after.starts_with(' ') {
                return Some(i);
            }
        }
    }
    None
}

fn colorize_yaml_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];

    if trimmed == "---" || trimmed == "..." {
        return ansi(DIM, line);
    }

    let (list_prefix, content) = if let Some(rest) = trimmed.strip_prefix("- ") {
        (format!("{}{} ", indent, ansi(WHITE, "-")), rest)
    } else if trimmed == "-" {
        return format!("{}{}", indent, ansi(WHITE, "-"));
    } else {
        (indent.to_string(), trimmed)
    };

    if let Some(colon_pos) = find_yaml_key_colon(content) {
        let key = &content[..colon_pos];
        let after = &content[colon_pos + 1..];
        let colored_key = ansi(CYAN, key);
        if let Some(value) = after.strip_prefix(' ') {
            format!("{}{}: {}", list_prefix, colored_key, colorize_yaml_scalar(value))
        } else {
            format!("{}{}:", list_prefix, colored_key)
        }
    } else {
        format!("{}{}", list_prefix, colorize_yaml_scalar(content))
    }
}

fn colorize_yaml(s: &str) -> String {
    let result = s.lines().map(colorize_yaml_line).collect::<Vec<_>>().join("\n");
    if s.ends_with('\n') { result + "\n" } else { result }
}

fn build_yaml_string(data: &OutputData) -> Result<String, CepError> {
    match data {
        OutputData::Single(price) => serde_yaml_ng::to_string(&SingleRecord { price: *price })
            .map_err(|e| CepError::Output(e.to_string())),
        OutputData::Buckets { buckets, points } => {
            #[derive(Serialize)]
            struct BucketsOutput {
                summary: Vec<BucketRecord>,
                points: Vec<PointRecord>,
            }
            let out = BucketsOutput {
                summary: buckets.iter().map(BucketRecord::from).collect(),
                points: points.iter().map(PointRecord::from).collect(),
            };
            serde_yaml_ng::to_string(&out).map_err(|e| CepError::Output(e.to_string()))
        }
    }
}

fn write_yaml(data: &OutputData, color: bool, w: &mut impl Write) -> Result<(), CepError> {
    let s = build_yaml_string(data)?;
    let output = if color { colorize_yaml(&s) } else { s };
    write!(w, "{}", output).map_err(|e| CepError::Output(e.to_string()))
}

// ── CSV ───────────────────────────────────────────────────────────────────────

fn write_csv(data: &OutputData, w: &mut impl Write) -> Result<(), CepError> {
    let mut csv_writer = csv::Writer::from_writer(w);
    match data {
        OutputData::Single(price) => {
            csv_writer
                .write_record(["field", "value"])
                .map_err(|e| CepError::Output(e.to_string()))?;
            csv_writer
                .write_record(["price", &format!("{:.4}", price)])
                .map_err(|e| CepError::Output(e.to_string()))?;
        }
        OutputData::Buckets { buckets, points } => {
            csv_writer
                .write_record(["section", "start_or_timestamp", "end", "average_or_price", "count"])
                .map_err(|e| CepError::Output(e.to_string()))?;
            for b in buckets {
                csv_writer
                    .write_record([
                        "summary",
                        &b.start.to_rfc3339(),
                        &b.end.to_rfc3339(),
                        &format!("{:.4}", b.average),
                        &b.count.to_string(),
                    ])
                    .map_err(|e| CepError::Output(e.to_string()))?;
            }
            for p in points {
                csv_writer
                    .write_record([
                        "point",
                        &p.timestamp.to_rfc3339(),
                        "",
                        &format!("{:.4}", p.price),
                        "",
                    ])
                    .map_err(|e| CepError::Output(e.to_string()))?;
            }
        }
    }
    csv_writer
        .flush()
        .map_err(|e| CepError::Output(e.to_string()))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucket::{Bucket, PricePoint};
    use chrono::{TimeZone, Utc};

    fn make_bucket(start_secs: i64, end_secs: i64, avg: f64, count: usize) -> Bucket {
        Bucket {
            start: Utc.timestamp_opt(start_secs, 0).single().unwrap(),
            end: Utc.timestamp_opt(end_secs, 0).single().unwrap(),
            average: avg,
            count,
        }
    }

    fn make_point(secs: i64, price: f64) -> PricePoint {
        PricePoint {
            timestamp: Utc.timestamp_opt(secs, 0).single().unwrap(),
            price,
        }
    }

    fn buckets_data() -> OutputData {
        OutputData::Buckets {
            buckets: vec![make_bucket(0, 3600, 2.5, 2)],
            points: vec![make_point(0, 2.0), make_point(1800, 3.0)],
        }
    }

    #[test]
    fn test_json_single() {
        let mut buf = Vec::new();
        write_output(&OutputData::Single(3.14), OutputFormat::Json, false, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!((v["price"].as_f64().unwrap() - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_json_buckets() {
        let mut buf = Vec::new();
        write_output(&buckets_data(), OutputFormat::Json, false, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["summary"][0]["count"].as_u64().unwrap(), 2);
        assert_eq!(v["points"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_csv_single() {
        let mut buf = Vec::new();
        write_output(&OutputData::Single(1.0), OutputFormat::Csv, false, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("field,value"));
        assert!(s.contains("price,"));
    }

    #[test]
    fn test_csv_buckets() {
        let mut buf = Vec::new();
        write_output(&buckets_data(), OutputFormat::Csv, false, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("summary,"));
        assert!(s.contains("point,"));
    }

    #[test]
    fn test_text_single() {
        let mut buf = Vec::new();
        write_output(&OutputData::Single(2.5), OutputFormat::Text, false, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("2.5000"));
        assert!(s.contains("¢/kWh"));
    }

    #[test]
    fn test_text_buckets() {
        let mut buf = Vec::new();
        write_output(&buckets_data(), OutputFormat::Text, false, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Summary"));
        assert!(s.contains("Points"));
    }

    #[test]
    fn test_yaml_colorize_line() {
        let line = "price: 1.42";
        let colored = colorize_yaml_line(line);
        assert!(colored.contains("\x1b[36m")); // cyan key
        assert!(colored.contains("\x1b[33m")); // yellow number
    }

    #[test]
    fn test_yaml_colorize_string_value() {
        let line = "start: '2025-05-15T15:00:00+00:00'";
        let colored = colorize_yaml_line(line);
        assert!(colored.contains("\x1b[36m")); // cyan key
        assert!(colored.contains("\x1b[32m")); // green string
    }
}
