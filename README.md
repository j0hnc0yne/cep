# cep — ComEd Electricity Price CLI

Fetch current and historical ComEd hourly electricity prices from the command line.

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)

### Build

```bash
./build.sh
```

This compiles a release binary to `./target/release/cep`. Optionally install it system-wide:

```bash
cp target/release/cep /usr/local/bin/
```

## Usage

```
Usage: cep [OPTIONS]

Options:
  -b, --base-url <BASE_URL>  Base URL for the API [default: https://hourlypricing.comed.com]
  -t, --type <QUERY_TYPE>    Price type: current (cur) or range [default: current]
  -s, --start <START>        Start date/time YYYYMMDDhhmm, Central Time (range only, default: now-24h)
  -e, --end <END>            End date/time YYYYMMDDhhmm, Central Time (range only, default: now)
  -a, --average <AVERAGE>    Return average (y) or bucketed list (n) [default: y]
  -i, --interval <INTERVAL>  Bucket interval: e.g. 5m, 2h, 1d (min 5m, max 7d) [default: 1d]
  -f, --format <FORMAT>      Output format: text, json, yaml, csv [default: json]
  -h, --help                 Print help
```

All options can also be set via environment variables (uppercase with underscores, e.g. `FORMAT=text`).

## Examples

### Current price

```bash
# JSON (default)
cep

# Plain text
cep -f text
```

```
Price: 1.6000 ¢/kWh
```

### Historical range — averaged

```bash
# Average price from 6 AM to noon today (Central Time)
cep -t range -s 202505150600 -e 202505151200 -f yaml
```

```yaml
price: 3.0
```

### Historical range — bucketed list

```bash
# Last 6 hours bucketed by 1 hour, CSV output
cep -t range -s 202505150600 -e 202505151200 -a n -i 1h -f csv
```

```
start,end,average,count
2025-05-15T11:00:00+00:00,2025-05-15T12:00:00+00:00,2.5750,12
2025-05-15T12:00:00+00:00,2025-05-15T13:00:00+00:00,2.8167,12
...
```

```bash
# Same range as a text table
cep -t range -s 202505150600 -e 202505151200 -a n -i 1h -f text
```

```
Start                            End                               Avg (¢/kWh)  Count
--------------------------------------------------------------------------------------
2025-05-15T11:00:00+00:00        2025-05-15T12:00:00+00:00              2.5750     12
...
```

### Using environment variables

```bash
export FORMAT=text
export TYPE=range
export START=202505150600
export END=202505151200
cep
```

## Notes

- Date/time inputs (`--start`, `--end`) are in **Central Time** (America/Chicago), matching ComEd's API convention.
- Response timestamps in JSON/YAML/CSV output are in **UTC** (RFC 3339).
- Prices are in **cents per kWh**.
- When `--average=n`, the API's 5-minute data points are averaged into buckets of the requested `--interval`.
- If `--interval` is larger than the requested time range, a single bucket covering the full range is returned.
- The underlying API is documented at <https://hourlypricing.comed.com/hp-api/>.
