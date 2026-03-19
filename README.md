# NattyDate

A lightweight, deterministic natural language date preprocessor written in pure Rust.

NattyDate operates as a strict pipeline: **normalize → tokenize → classify → score → resolve → format**. It handles messy human date/time strings and outputs predictable canonical structures, ISO-8601 timestamps, or any custom format template — with no ML, no runtime dependencies, and no clock fragility.

[![CI](https://github.com/troy74/nattydate/actions/workflows/ci.yml/badge.svg)](https://github.com/troy74/nattydate/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/nattydate.svg)](https://crates.io/crates/nattydate)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

---

## Features

- **Pure Rust.** No Python, no ML runtimes, no external services.
- **Fuzzy scoring.** Resolves ambiguity using Levenshtein distance and a confidence scoring system (`0.0`–`1.0`). Noise is discarded; context windows infer intent (e.g. `"3"` after `"at"` becomes `15:00:00`).
- **Past & future dates.** `"last monday"`, `"monday last"`, `"3 days ago"`, `"previous friday"` all resolve correctly alongside forward-looking expressions.
- **18 public holidays.** Floating and static US/UK holidays (`thanksgiving`, `spring bank holiday`, `mlk day`, etc.). Automatically rolls to the next occurrence if the date has passed.
- **Flexible time parsing.** `"07h00"`, `"9|00"`, `"nine thirty-five"`, `"quarter past ten"`, `"half past two"` all resolve correctly without destroying genuine dates like `"9-1"`.
- **Custom output templates.** Map resolved tokens to any format string using named placeholders.
- **Deterministic testing.** Built-in test runner with injectable mock clock — 104 test cases, zero flakiness.

---

## Installation

### From source

```bash
git clone https://github.com/troy74/nattydate.git
cd nattydate
cargo build --release
```

Binary: `./target/release/nattydate`
A pre-built macOS binary is included in `./build/` for convenience.

### Via cargo

```bash
cargo install nattydate
```

---

## Usage

### Basic

```bash
nattydate "tomorrow at 3pm"
# tomorrow at 15:00:00

nattydate "thanksgiving"
# 2026-11-26
```

### Custom Format Templates (`-f`)

Use `-f` to apply a format template. Supported placeholders:

| Placeholder | Output |
|---|---|
| `YYYY` | 4-digit year |
| `YY` | 2-digit year |
| `MM` | Zero-padded month |
| `DD` | Zero-padded day |
| `HH` | Zero-padded 24h hour (defaults to `00` if absent) |
| `mm` | Zero-padded minute (defaults to `00` if absent) |
| `ss` | Zero-padded second (defaults to `00` if absent) |
| `TZ` | Timezone string (e.g. `EDT`, `UTC+05:30`) |
| `Z` | Alias for `TZ` |
| `{RELATIVE}` | Relative day name or weekday |

```bash
nattydate "monday morning at nine thirty-five" -f "YYYY-MM-DD HH:mm:ss"
# 2026-03-23 09:35:00

nattydate "next friday 14:00 GMT" -f "YYYY-MM-DD HH:mm:ss TZ"
# 2026-03-27 14:00:00 GMT

nattydate "tomorrow at 3pm" -f "{RELATIVE} HH:mm"
# tomorrow 15:00
```

If the timezone is absent, `TZ`/`Z` and their surrounding space are cleanly removed.

### JSON AST Output (`-o json`)

Output the full resolved token array as JSON for downstream processing:

```bash
nattydate "nxt fri 14:00" -o json
```

```json
[
  {
    "Known": {
      "token": {
        "DateNumeric": { "y": 2026, "m": 3, "d": 27 }
      },
      "score": 1.0
    }
  },
  {
    "Known": {
      "token": {
        "Time": { "hour": 14, "min": 0, "sec": null, "formatted": "14:00:00" }
      },
      "score": 0.95
    }
  }
]
```

> Note: modifiers (`next`, `last`, etc.) are consumed during resolution and do not appear in the final token list.

### Day-First Parsing (`--day-first`)

For ambiguous numeric dates, control whether the first number is treated as day or month:

```bash
nattydate "9/1/26"               # MM/DD/YY → September 1
nattydate "9/1/26" --day-first   # DD/MM/YY → January 9
```

### Debug Mode (`--debug`)

Prints internal token evaluation details and context boosts — useful when investigating unexpected parses:

```bash
nattydate "mrning" --debug
```

---

## Fuzzy Parsing

NattyDate handles noisy, abbreviated, or typo-ridden input. Pass strings as-is — do not pre-clean them.

| Input | Resolved |
|---|---|
| `"tmrw"` | tomorrow |
| `"mrning"` | morning (09:00) |
| `"evning"` | evening (18:00) |
| `"nxt fri"` | next friday |
| `"thurday"` | thursday (fuzzy) |
| `"satrday"` | saturday (fuzzy) |
| `"9\|00"` | 09:00:00 |
| `"07h00"` | 07:00:00 |
| `"quarter past nine"` | 09:15:00 |
| `"half past ten"` | 10:30:00 |

Fuzzy matching uses Levenshtein distance with prefix and first-character bonuses. A candidate must score ≥ 0.65 to be accepted.

---

## Past Date Expressions

NattyDate resolves past-pointing expressions as naturally as future ones.

| Input | Resolved (from Wed 2026-03-18) |
|---|---|
| `"last monday"` | 2026-03-16 |
| `"monday last"` | 2026-03-16 (postfix modifier) |
| `"previous friday"` | 2026-03-13 |
| `"past thursday"` | 2026-03-12 |
| `"prior tuesday"` | 2026-03-17 |
| `"1 day ago"` | 2026-03-17 |
| `"3 days ago"` | 2026-03-15 |
| `"1 week ago"` | 2026-03-11 |
| `"2 weeks ago"` | 2026-03-04 |
| `"3 days ago at noon"` | 2026-03-15 12:00:00 |

Supported past modifiers: `last`, `past`, `previous`, `prev`, `prior` (all resolve to `Last`).
`ago` is resolved as a postfix unit quantifier: `N day(s)/week(s)/month(s)/year(s) ago`.
`following` resolves as a synonym for `next`.

---

## Supported Holidays

| Holiday | Type | Date |
|---|---|---|
| Christmas | Static | Dec 25 |
| New Year's Day | Static | Jan 1 |
| Independence Day | Static | Jul 4 |
| Halloween | Static | Oct 31 |
| Veterans Day | Static | Nov 11 |
| Juneteenth | Static | Jun 19 |
| Valentine's Day | Static | Feb 14 |
| Boxing Day | Static | Dec 26 |
| Guy Fawkes Night / Bonfire Night | Static | Nov 5 |
| St. Patrick's Day | Static | Mar 17 |
| Thanksgiving (US) | Floating | 4th Thursday of November |
| Memorial Day | Floating | Last Monday of May |
| Labor Day | Floating | 1st Monday of September |
| MLK Day | Floating | 3rd Monday of January |
| Presidents Day | Floating | 3rd Monday of February |
| May Day | Floating | 1st Monday of May |
| Spring Bank Holiday (UK) | Floating | Last Monday of May |
| Summer Bank Holiday (UK) | Floating | Last Monday of August |

If a holiday has already passed in the current year, NattyDate resolves to the next occurrence.

---

## Testing

NattyDate ships with an integrated, clock-isolated test runner. All relative date tests use a fixed `mock_now` date declared in the JSON suite — no flaky system-clock dependencies.

```bash
# Run the bundled 104-case suite
nattydate test

# Verbose output (individual pass/fail per case)
nattydate test --verbose

# Use a custom test file
nattydate test --test-file my_suite.json
```

The test runner exits with code `1` if any case fails, making it suitable for CI pipelines.
`cargo test` runs the same suite as a Rust unit test.

### Test File Format

```json
{
  "mock_now": "2026-03-18",
  "cases": [
    {
      "input": "last monday",
      "expected": "2026-03-16",
      "format": "YYYY-MM-DD"
    },
    {
      "input": "tomorrow at 3pm",
      "expected": "2026-03-19 15:00:00",
      "format": "YYYY-MM-DD HH:mm:ss"
    }
  ]
}
```

- `mock_now`: The date treated as "now" for all relative resolution.
- `input`: Raw natural language string to parse.
- `expected`: Exact string the formatted output must equal (after trimming).
- `format`: Format template applied before comparison.

---

## Library Usage

NattyDate can be used as a Rust library:

```toml
[dependencies]
nattydate = "0.1"
```

```rust
use nattydate::{ParseConfig, tokenize_and_classify, format_custom};

let config = ParseConfig {
    day_first: false,
    resolve_dates: true,
    mock_now: None,   // uses Local::now()
    debug: false,
};

let tokens = tokenize_and_classify("last friday at noon", &config);
let output = format_custom(&tokens, "YYYY-MM-DD HH:mm:ss");
println!("{}", output);   // e.g. "2026-03-13 12:00:00"
```

---

## Dependencies

| Crate | Purpose |
|---|---|
| `chrono` | Date/time primitives and weekday arithmetic |
| `clap` | CLI argument parsing |
| `serde` / `serde_json` | JSON serialization for AST output and test suite |
| `strsim` | Levenshtein distance for fuzzy token matching |

---

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
