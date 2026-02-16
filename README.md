# perf_tool

A small Rust CLI for analyzing HAR files and reporting request timing and size hotspots.

## What it does

Given a HAR file, `perf_tool` reports:

- total entries
- total request time in milliseconds
- total response bytes
- top N slowest requests
- top N largest requests by bytes
- optional top N host groups with count/time/size metrics

It supports both human-readable text output and JSON output.

## Build

```bash
cargo build --release
```

## Usage

```bash
cargo run -- <PATH_TO_HAR>
```

Example with the included fixture:

```bash
cargo run -- tests/fixtures/sample.har
```

Limit the report to top 2 results:

```bash
cargo run -- --top 2 tests/fixtures/sample.har
```

Emit JSON:

```bash
cargo run -- --json tests/fixtures/sample.har
```

Group results by host:

```bash
cargo run -- --group-by host tests/fixtures/sample.har
```

Show help:

```bash
cargo run -- --help
```

## Example text output

```text
entries: 4
total_time_ms: 565.75
total_bytes: 3.29 KB

slowest 4:
  320.50 ms https://example.com/a
  180.25 ms https://example.com/c
   55.00 ms https://example.com/b
   10.00 ms https://example.com/d

largest 4 by bytes:
   2.03 KB  https://example.com/c
   1.20 KB  https://example.com/a
      70 B  https://example.com/b
       0 B  https://example.com/d

groups by host (top 1):
   4 req    565.75 ms total    141.44 ms avg    320.50 ms p95     3.29 KB  example.com
```

## Example JSON shape

```json
{
  "entries": 4,
  "total_time_ms": 565.75,
  "total_bytes": 3372,
  "top_requested": 10,
  "top_returned": 4,
  "group_by": "host",
  "top_slowest": [
    {
      "url": "https://example.com/a",
      "time_ms": 320.5,
      "bytes": 1224
    }
  ],
  "top_largest": [
    {
      "url": "https://example.com/c",
      "time_ms": 180.25,
      "bytes": 2078
    }
  ],
  "top_groups": [
    {
      "key": "example.com",
      "count": 4,
      "total_time_ms": 565.75,
      "avg_time_ms": 141.4375,
      "p95_time_ms": 320.5,
      "total_bytes": 3372
    }
  ]
}
```

## Errors

- Missing file path: CLI usage/help is shown by argument parsing.
- Missing or unreadable file: returns an error containing `failed to read file`.
- Invalid HAR/JSON: returns an error containing `failed to parse HAR JSON`.
