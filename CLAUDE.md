# CLAUDE.md — cep

## Project overview

`cep` is a Rust CLI tool that fetches ComEd electricity prices from the ComEd Hourly Pricing API (`https://hourlypricing.comed.com/hp-api/`). Binary name is `cep`.

## Build and run

```bash
# cargo is at ~/.cargo/bin/cargo if not on PATH
~/.cargo/bin/cargo build --release
./target/release/cep --help

# Or use the build script
./build.sh
```

Tests: `~/.cargo/bin/cargo test`

## Architecture

```
src/
  main.rs      — parse → validate → fetch → transform → output
  config.rs    — clap Config struct, ValidatedConfig, validate(), parse_interval(), format_api_date()
  api.rs       — ApiEntry type, fetch_current(), fetch_range()
  bucket.rs    — PricePoint, Bucket, parse_entries(), average_all(), bucket_by_interval()
  output.rs    — OutputData enum, write_output() dispatching to text/json/yaml/csv formatters
  error.rs     — CepError enum (thiserror)
```

## Key invariants

**Timezone**: ComEd API expects `datestart`/`dateend` in Central Time (format `YYYYMMDDhhmm`), but returns `millisUTC` in UTC milliseconds. `ValidatedConfig` holds both:
- `start_naive` / `end_naive` — Central Time, passed to the API query string via `format_api_date()`
- `start_utc` / `end_utc` — same instants converted to UTC, used for bucket window comparisons

Never use the UTC fields for API calls or the naive fields for bucket comparisons.

**OutputData variants**:
- `OutputData::Single(f64)` — used when `average=y`; just one number
- `OutputData::Buckets { buckets: Vec<Bucket>, points: Vec<PricePoint> }` — used when `average=n`; always includes both the bucketed summary and the raw individual points

All four formatters (text, json, yaml, csv) must handle both variants.

## Dependencies

| Crate | Purpose |
|---|---|
| `clap` v4 (derive + env) | CLI flags + env var binding |
| `reqwest` v0.12 (blocking + json) | HTTP requests |
| `serde` + `serde_json` | JSON deserialization (API) and serialization (output) |
| `serde_yaml_ng` v0.10 | YAML output (maintained fork — use `serde_yaml_ng`, not `serde_yaml`) |
| `chrono` v0.4 | Date/time parsing and arithmetic |
| `chrono-tz` v0.10 | Central Time (`chrono_tz::US::Central`) conversions |
| `csv` v1 | CSV output |
| `thiserror` v2 | Typed error enum |
| `tokio` v1 (rt + rt-multi-thread) | Required by reqwest blocking under the hood |

**Note**: the crate is `serde_yaml_ng` (underscore), not `serde-yaml-ng` (hyphen) — crates.io requires the underscore form in `use` statements and Cargo.toml `[dependencies]`.

## CLI flags

All flags have both short (`-x`) and long (`--name`) forms, plus an env var fallback.

| Flag | Env | Default | Values |
|---|---|---|---|
| `-b` / `--base-url` | `BASE_URL` | `https://hourlypricing.comed.com` | URL |
| `-t` / `--type` | `TYPE` | `current` | `current`, `cur`, `range` |
| `-s` / `--start` | `START` | now−24h | `YYYYMMDDhhmm` (Central Time) |
| `-e` / `--end` | `END` | now | `YYYYMMDDhhmm` (Central Time) |
| `-a` / `--average` | `AVERAGE` | `y` | `y`, `yes`, `n`, `no` |
| `-i` / `--interval` | `INTERVAL` | `1d` | `5m`–`7d` (units: m/h/d; min 5m) |
| `-f` / `--format` | `FORMAT` | `json` | `text`, `json`, `yaml`, `csv` |

## Output formats

**JSON** (`average=n`):
```json
{ "summary": [{"start":"...","end":"...","average":1.42,"count":9}], "points": [...] }
```

**CSV** (`average=n`): single flat table with a `section` column (`"summary"` or `"point"`).

**Text** (`average=n`): "Summary" table printed first, then "Points" table.
