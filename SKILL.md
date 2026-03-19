---
name: nattydate
description: Parse, format, and evaluate natural language date/time strings using the NattyDate Rust CLI. Use when the user wants to resolve, normalize, or validate a human-written date or time expression — including past dates, ago patterns, and fuzzy misspellings.
---

# NattyDate Parser Skill

NattyDate is a deterministic, fuzzy-scoring natural language date preprocessor. It takes messy human-written date/time strings and outputs normalized canonical text, custom format strings, or a JSON token AST.

## Quick Reference

| Goal | Command |
|---|---|
| Normalize a date string | `nattydate "next monday at 9am"` |
| Output a strict format | `nattydate "tomorrow 3pm" -f "YYYY-MM-DD HH:mm:ss"` |
| Get JSON token AST | `nattydate "nxt fri 14:00" -o json` |
| Treat input as DD/MM/YY | `nattydate "9/1/26" --day-first` |
| Run test suite | `nattydate test` |
| Run tests verbosely | `nattydate test --verbose` |
| Run custom test file | `nattydate test --test-file my_suite.json` |
| Debug token evaluation | `nattydate "mrning" --debug` |

---

## Format Placeholders

Use these with `-f "..."`:

| Token | Meaning | Default if absent |
|---|---|---|
| `YYYY` | 4-digit year | literal `YYYY` |
| `YY` | 2-digit year | literal `YY` |
| `MM` | Zero-padded month (01–12) | literal `MM` |
| `DD` | Zero-padded day (01–31) | literal `DD` |
| `HH` | Zero-padded 24h hour | `00` |
| `mm` | Zero-padded minute | `00` |
| `ss` | Zero-padded second | `00` |
| `TZ` | Timezone name or offset (e.g. `EDT`, `+05:30`) | removed |
| `Z` | Alias for `TZ` | removed |
| `{RELATIVE}` | Relative label (`tomorrow`, `monday`, etc.) | removed |

Absent timezone: `TZ`/`Z` and their surrounding space are automatically stripped. Absent time components default to `00`. Absent date components remain as literal placeholders.

---

## When to Use Each Mode

**Canonical (default)** — Human-readable normalized output.

```bash
nattydate "tomorrow at 3pm EDT"
# tomorrow at 15:00:00 EDT
```

**Custom format (`-f`)** — Fixed timestamp format for downstream systems. Most common production use case.

```bash
nattydate "next friday 2pm" -f "YYYY-MM-DD HH:mm:ss"
# 2026-03-27 14:00:00
```

**JSON (`-o json`)** — Full resolved token AST. Use to inspect token types, scores, and structure, or to build pipelines that branch on token type.

```bash
nattydate "nxt fri 14:00" -o json
```

```json
[
  {
    "Known": {
      "token": { "DateNumeric": { "y": 2026, "m": 3, "d": 27 } },
      "score": 1.0
    }
  },
  {
    "Known": {
      "token": { "Time": { "hour": 14, "min": 0, "sec": null, "formatted": "14:00:00" } },
      "score": 0.95
    }
  }
]
```

> Note: `next` / `last` modifiers are consumed during resolution and do not appear in the JSON output — they are absorbed into the resolved `DateNumeric`.

---

## Past Date Expressions

NattyDate resolves past-pointing expressions naturally:

```bash
nattydate "last monday" -f "YYYY-MM-DD"       # 2026-03-16
nattydate "monday last" -f "YYYY-MM-DD"       # 2026-03-16  (postfix)
nattydate "previous friday" -f "YYYY-MM-DD"   # 2026-03-13
nattydate "past thursday" -f "YYYY-MM-DD"     # 2026-03-12
nattydate "3 days ago" -f "YYYY-MM-DD"        # 2026-03-15
nattydate "1 week ago" -f "YYYY-MM-DD"        # 2026-03-11
nattydate "2 weeks ago at 6pm" -f "YYYY-MM-DD HH:mm:ss"  # 2026-03-04 18:00:00
```

Supported past synonyms: `last`, `past`, `previous`, `prev`, `prior`.
`ago` syntax: `N day(s)`, `N week(s)`, `N month(s)`, `N year(s)` before `ago`.
Postfix: `"monday last"`, `"friday next"` — modifier can follow the weekday.

---

## Fuzzy Input — Do Not Pre-Clean

NattyDate's fuzzy engine handles noisy input. Pass strings as-is. Pre-normalizing defeats the purpose and may break context-sensitive scoring.

**These all work correctly:**

```bash
nattydate "tmrw mrning"          # tomorrow morning → 09:00:00
nattydate "nxt fri 14:00"        # next friday 14:00
nattydate "thurday"              # thursday (fuzzy)
nattydate "satrday"              # saturday (fuzzy)
nattydate "ysterday"             # yesterday (fuzzy)
nattydate "9|00"                 # 09:00:00
nattydate "07h00"                # 07:00:00
nattydate "quarter past nine"    # 09:15:00
nattydate "half past two"        # 02:30:00
nattydate "monday week"          # monday + 7 days
nattydate "evning"               # evening (18:00)
```

Fuzzy matching uses Levenshtein distance with prefix and first-character bonuses. A candidate must score ≥ 0.65 to be accepted.

---

## Ambiguous Numeric Dates

Default ordering is MM/DD/YY. Use `--day-first` for DD/MM/YY:

```bash
nattydate "9/1/26"               # September 1, 2026
nattydate "9/1/26" --day-first   # January 9, 2026
nattydate "18.3.26"              # March 18, 2026  (18 > 12 → unambiguously day-first)
nattydate "2026/03/18"           # March 18, 2026  (YYYY first → unambiguous)
```

---

## Holidays

NattyDate resolves 18 US/UK holidays by name. If the holiday has already passed this year, it resolves to next year's occurrence.

```bash
nattydate "thanksgiving" -f "YYYY-MM-DD"          # 2026-11-26
nattydate "christmas" -f "YYYY-MM-DD"             # 2026-12-25
nattydate "spring bank holiday" -f "YYYY-MM-DD"   # UK floating date
nattydate "mlk day" -f "YYYY-MM-DD"               # 3rd Monday of January
nattydate "bonfire night" -f "YYYY-MM-DD"         # Guy Fawkes, Nov 5
nattydate "new years day" -f "YYYY-MM-DD"         # Jan 1, next year if past
```

Full list: Christmas, New Year's Day, Independence Day, Halloween, Veterans Day, Juneteenth, Valentine's Day, Boxing Day, Guy Fawkes Night / Bonfire Night, St. Patrick's Day, Thanksgiving, Memorial Day, Labor Day, MLK Day, Presidents Day, May Day, Spring Bank Holiday, Summer Bank Holiday.

---

## Writing Test Suites

Use the test runner to validate date resolution against a fixed mock clock — critical for any relative date logic.

```json
{
  "mock_now": "2026-03-18",
  "cases": [
    { "input": "last monday",    "expected": "2026-03-16",           "format": "YYYY-MM-DD" },
    { "input": "3 days ago",     "expected": "2026-03-15",           "format": "YYYY-MM-DD" },
    { "input": "tomorrow at 3pm","expected": "2026-03-19 15:00:00",  "format": "YYYY-MM-DD HH:mm:ss" }
  ]
}
```

- `mock_now` pins "today" for all relative resolution in the suite.
- `expected` must exactly match the formatted output (after trimming).
- Test runner exits with code `1` on any failure — safe for CI.

**Never** validate relative date output without a `mock_now`.

---

## Debugging Unexpected Parses

1. Run with `--debug` to see per-token evaluation and context boosts:
   ```bash
   nattydate "9-00" --debug
   ```

2. Run with `-o json` to inspect the full resolved token AST:
   ```bash
   nattydate "9-00" -o json
   ```

3. Check if the issue is date vs. time ambiguity. `"9-1"` parses as September 1 (date, score 0.75), not 09:01 (time, score 0.8, but gap < `THRESHOLD_MARGIN`). Context boosts from adjacent tokens can tip the balance — inspect neighbours.

4. If a token appears as `Unknown`, no candidate scored ≥ 0.65. Try less abbreviated input, or check for unsupported character encodings.

5. Past expressions not working? Confirm the modifier synonym is in the dictionary (`last`, `past`, `previous`, `prev`, `prior` all map to `Last`). Postfix modifiers (`"monday last"`) require the modifier to immediately follow the weekday with no intervening tokens.

---

## Common Patterns

**Resolve and format for a database insert:**
```bash
nattydate "next tuesday at noon" -f "YYYY-MM-DD HH:mm:ss"
```

**Past date for an audit log entry:**
```bash
nattydate "3 days ago at 9am" -f "YYYY-MM-DD HH:mm:ss"
```

**Extract date and time separately:**
```bash
nattydate "last friday 3pm EDT" -f "YYYY-MM-DD"    # date only
nattydate "last friday 3pm EDT" -f "HH:mm:ss TZ"   # time only
```

**Validate an ISO 8601 timestamp passes through cleanly:**
```bash
nattydate "2026-03-18T08:00:00Z" -f "YYYY-MM-DD HH:mm:ss Z"
# 2026-03-18 08:00:00 UTC
```

**Batch processing via shell loop:**
```bash
while IFS= read -r line; do
  nattydate "$line" -f "YYYY-MM-DD HH:mm:ss"
done < dates.txt
```
