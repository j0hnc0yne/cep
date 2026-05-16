use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::US::Central;
use clap::Parser;

use crate::error::CepError;

#[derive(Debug, Clone, Parser)]
#[command(name = "cep", about = "ComEd Electricity Price CLI")]
pub struct Config {
    #[arg(
        short = 'b',
        long = "base-url",
        env = "BASE_URL",
        default_value = "https://hourlypricing.comed.com",
        help = "Base URL for the API"
    )]
    pub base_url: String,

    #[arg(
        short = 't',
        long = "type",
        env = "TYPE",
        default_value = "current",
        help = "Price type: 'current' (cur) or 'range'"
    )]
    pub query_type: String,

    #[arg(
        short = 's',
        long = "start",
        env = "START",
        help = "Start date/time YYYYMMDDhhmm (range only, default: now-24h)"
    )]
    pub start: Option<String>,

    #[arg(
        short = 'e',
        long = "end",
        env = "END",
        help = "End date/time YYYYMMDDhhmm (range only, default: now)"
    )]
    pub end: Option<String>,

    #[arg(
        short = 'a',
        long = "average",
        env = "AVERAGE",
        default_value = "y",
        help = "Return average (y) or bucketed list (n)"
    )]
    pub average: String,

    #[arg(
        short = 'i',
        long = "interval",
        env = "INTERVAL",
        default_value = "1d",
        help = "Bucket interval when average=n: e.g. 5m, 2h, 1d (min 5m, max 7d)"
    )]
    pub interval: String,

    #[arg(
        short = 'f',
        long = "format",
        env = "FORMAT",
        default_value = "json",
        help = "Output format: text, json, yaml, csv"
    )]
    pub format: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    Current,
    Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Yaml,
    Csv,
}

#[derive(Debug, Clone)]
pub struct ValidatedConfig {
    pub base_url: String,
    pub query_type: QueryType,
    /// Start in Central Time, as supplied to the API
    pub start_naive: NaiveDateTime,
    /// End in Central Time, as supplied to the API
    pub end_naive: NaiveDateTime,
    /// start_naive interpreted as Central Time, converted to UTC for bucketing
    pub start_utc: DateTime<Utc>,
    /// end_naive interpreted as Central Time, converted to UTC for bucketing
    pub end_utc: DateTime<Utc>,
    pub average: bool,
    pub interval: Duration,
    pub format: OutputFormat,
}

const DATE_FMT: &str = "%Y%m%d%H%M";

impl Config {
    pub fn validate(&self) -> Result<ValidatedConfig, CepError> {
        let query_type = match self.query_type.to_lowercase().as_str() {
            "cur" | "current" => QueryType::Current,
            "range" => QueryType::Range,
            v => {
                return Err(CepError::InvalidOptionValue {
                    option: "type".into(),
                    value: v.into(),
                    possible: "current, cur, range".into(),
                })
            }
        };

        let average = match self.average.to_lowercase().as_str() {
            "y" | "yes" => true,
            "n" | "no" => false,
            v => {
                return Err(CepError::InvalidOptionValue {
                    option: "average".into(),
                    value: v.into(),
                    possible: "y, n".into(),
                })
            }
        };

        let format = match self.format.to_lowercase().as_str() {
            "text" => OutputFormat::Text,
            "json" => OutputFormat::Json,
            "yaml" => OutputFormat::Yaml,
            "csv" => OutputFormat::Csv,
            v => {
                return Err(CepError::InvalidOptionValue {
                    option: "format".into(),
                    value: v.into(),
                    possible: "text, json, yaml, csv".into(),
                })
            }
        };

        let now_central = Utc::now().with_timezone(&Central).naive_local();
        let start_naive = match &self.start {
            None => now_central - Duration::hours(24),
            Some(s) => NaiveDateTime::parse_from_str(s, DATE_FMT)
                .map_err(|_| CepError::InvalidDateFormat { input: s.clone() })?,
        };
        let end_naive = match &self.end {
            None => now_central,
            Some(s) => NaiveDateTime::parse_from_str(s, DATE_FMT)
                .map_err(|_| CepError::InvalidDateFormat { input: s.clone() })?,
        };

        if start_naive >= end_naive {
            return Err(CepError::StartNotBeforeEnd);
        }

        let start_utc = Central
            .from_local_datetime(&start_naive)
            .earliest()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        let end_utc = Central
            .from_local_datetime(&end_naive)
            .earliest()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let interval = parse_interval(&self.interval)?;

        Ok(ValidatedConfig {
            base_url: self.base_url.trim_end_matches('/').to_string(),
            query_type,
            start_naive,
            end_naive,
            start_utc,
            end_utc,
            average,
            interval,
            format,
        })
    }
}

pub fn parse_interval(s: &str) -> Result<Duration, CepError> {
    let err = || CepError::InvalidInterval {
        input: s.to_string(),
    };

    if s.is_empty() {
        return Err(err());
    }

    let unit = s.chars().last().ok_or_else(err)?;
    let digits = &s[..s.len() - 1];
    let n: u64 = digits.parse().map_err(|_| err())?;

    if n == 0 {
        return Err(err());
    }

    let duration = match unit {
        'm' => {
            if n < 5 {
                return Err(err());
            }
            Duration::minutes(n as i64)
        }
        'h' => Duration::hours(n as i64),
        'd' => Duration::days(n as i64),
        _ => return Err(err()),
    };

    let max = Duration::days(7);
    if duration > max {
        return Err(err());
    }

    Ok(duration)
}

pub fn format_api_date(dt: NaiveDateTime) -> String {
    dt.format(DATE_FMT).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_interval_valid() {
        assert_eq!(parse_interval("5m").unwrap(), Duration::minutes(5));
        assert_eq!(parse_interval("30m").unwrap(), Duration::minutes(30));
        assert_eq!(parse_interval("2h").unwrap(), Duration::hours(2));
        assert_eq!(parse_interval("1d").unwrap(), Duration::days(1));
        assert_eq!(parse_interval("7d").unwrap(), Duration::days(7));
    }

    #[test]
    fn test_parse_interval_invalid() {
        assert!(parse_interval("4m").is_err()); // below min
        assert!(parse_interval("8d").is_err()); // above max
        assert!(parse_interval("3x").is_err()); // bad unit
        assert!(parse_interval("0m").is_err()); // zero
        assert!(parse_interval("").is_err()); // empty
        assert!(parse_interval("h").is_err()); // no digits
    }

    #[test]
    fn test_format_api_date() {
        let dt = NaiveDateTime::parse_from_str("202505151430", "%Y%m%d%H%M").unwrap();
        assert_eq!(format_api_date(dt), "202505151430");
    }
}
