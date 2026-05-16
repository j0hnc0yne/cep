use chrono::{DateTime, Duration, TimeZone, Utc};

use crate::api::ApiEntry;
use crate::error::CepError;

#[derive(Debug, Clone)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub price: f64,
}

#[derive(Debug, Clone)]
pub struct Bucket {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub average: f64,
    pub count: usize,
}

pub fn parse_entries(raw: Vec<ApiEntry>) -> Result<Vec<PricePoint>, CepError> {
    let mut points = Vec::with_capacity(raw.len());
    for entry in raw {
        let millis: i64 = entry
            .millis_utc
            .parse()
            .map_err(|_| CepError::Api(format!("invalid millisUTC: {}", entry.millis_utc)))?;
        let price: f64 = entry
            .price
            .parse()
            .map_err(|_| CepError::Api(format!("invalid price: {}", entry.price)))?;
        let secs = millis / 1000;
        let nanos = ((millis % 1000) * 1_000_000) as u32;
        let timestamp = Utc
            .timestamp_opt(secs, nanos)
            .single()
            .ok_or_else(|| CepError::Api(format!("invalid timestamp: {}", millis)))?;
        points.push(PricePoint { timestamp, price });
    }
    points.sort_by_key(|p| p.timestamp);
    Ok(points)
}

pub fn average_all(points: &[PricePoint]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let sum: f64 = points.iter().map(|p| p.price).sum();
    sum / points.len() as f64
}

pub fn bucket_by_interval(
    points: &[PricePoint],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    interval: Duration,
) -> Vec<Bucket> {
    let start_utc = start;
    let end_utc = end;
    let range = end_utc - start_utc;

    if interval >= range {
        let avg = average_all(points);
        return vec![Bucket {
            start: start_utc,
            end: end_utc,
            average: avg,
            count: points.len(),
        }];
    }

    let mut buckets = Vec::new();
    let mut bucket_start = start_utc;

    while bucket_start < end_utc {
        let bucket_end = (bucket_start + interval).min(end_utc);
        let bucket_points: Vec<&PricePoint> = points
            .iter()
            .filter(|p| p.timestamp >= bucket_start && p.timestamp < bucket_end)
            .collect();
        let avg = if bucket_points.is_empty() {
            0.0
        } else {
            let sum: f64 = bucket_points.iter().map(|p| p.price).sum();
            sum / bucket_points.len() as f64
        };
        buckets.push(Bucket {
            start: bucket_start,
            end: bucket_end,
            average: avg,
            count: bucket_points.len(),
        });
        bucket_start = bucket_end;
    }

    buckets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ApiEntry;

    fn make_point(millis: i64, price: f64) -> PricePoint {
        let secs = millis / 1000;
        let nanos = ((millis % 1000) * 1_000_000) as u32;
        PricePoint {
            timestamp: Utc.timestamp_opt(secs, nanos).single().unwrap(),
            price,
        }
    }

    #[test]
    fn test_parse_entries() {
        let raw = vec![
            ApiEntry {
                millis_utc: "1000000000000".into(),
                price: "2.5".into(),
            },
            ApiEntry {
                millis_utc: "999999999000".into(),
                price: "3.0".into(),
            },
        ];
        let points = parse_entries(raw).unwrap();
        assert_eq!(points.len(), 2);
        // sorted ascending
        assert!(points[0].timestamp < points[1].timestamp);
        assert_eq!(points[0].price, 3.0);
        assert_eq!(points[1].price, 2.5);
    }

    #[test]
    fn test_average_all() {
        let points = vec![make_point(0, 2.0), make_point(300000, 4.0)];
        assert_eq!(average_all(&points), 3.0);
        assert_eq!(average_all(&[]), 0.0);
    }

    #[test]
    fn test_bucket_by_interval_two_buckets() {
        // 4 points spread over 2 hours, bucket by 1h → 2 buckets of 2 each
        let base_ms: i64 = 1_700_000_000_000;
        let hour_ms = 3_600_000i64;
        let points = vec![
            make_point(base_ms, 1.0),
            make_point(base_ms + 1800_000, 3.0),
            make_point(base_ms + hour_ms, 5.0),
            make_point(base_ms + hour_ms + 1800_000, 7.0),
        ];
        let start = Utc.timestamp_opt(base_ms / 1000, 0).single().unwrap();
        let end = Utc
            .timestamp_opt((base_ms + 2 * hour_ms) / 1000, 0)
            .single()
            .unwrap();
        let buckets = bucket_by_interval(&points, start, end, Duration::hours(1));
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].count, 2);
        assert_eq!(buckets[0].average, 2.0);
        assert_eq!(buckets[1].count, 2);
        assert_eq!(buckets[1].average, 6.0);
    }

    #[test]
    fn test_bucket_interval_larger_than_range() {
        let points = vec![make_point(0, 4.0), make_point(60_000, 6.0)];
        let start = Utc.timestamp_opt(0, 0).single().unwrap();
        let end = Utc.timestamp_opt(120, 0).single().unwrap();
        let buckets = bucket_by_interval(&points, start, end, Duration::hours(1));
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].average, 5.0);
        assert_eq!(buckets[0].count, 2);
    }
}
