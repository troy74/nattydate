# Changelog

All notable changes to NattyDate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.1.0] — 2026-03-18

Initial public release.

### Added

**Parser core**
- Deterministic pipeline: normalize → tokenize → classify → score → resolve → format
- Fuzzy token matching via Levenshtein distance with prefix and first-character bonuses (threshold ≥ 0.65)
- Context-boost system: temporal adjacency, modifier→weekday, number+unit, at+number→time
- O(1) exact dictionary lookup via `OnceLock<HashMap>` with lazy fuzzy fallback

**Date resolution**
- Relative days: `today`, `tomorrow`, `yesterday` (+ aliases `tmrw`, `tmr`, `tomorow`)
- Weekday resolution with `next`, `this`, `last` prefix modifiers
- Postfix modifiers: `"monday last"`, `"friday next"`
- Past synonyms: `past`, `previous`, `prev`, `prior` (all map to `Last`)
- Future synonym: `following` (maps to `Next`)
- `"N unit(s) ago"` patterns: `1 day ago`, `3 days ago`, `1 week ago`, `2 weeks ago`, `1 month ago`
- `"monday week"` — weekday + 7 days
- Numeric dates: `MM/DD/YY`, `DD/MM/YY` (`--day-first`), `YYYY-MM-DD`, `DD.MM.YYYY`, two-part `MM-DD`
- ISO 8601 passthrough: `2026-03-18T08:00:00Z`
- Auto year-fill: yearless dates in the past roll forward to next year

**Time resolution**
- 24h colon format: `14:30:00`, `09:00`
- 12h am/pm: `3pm`, `9am`, `3:30pm`, `12am`, `12pm`
- Named times: `morning` (09:00), `noon` (12:00), `afternoon` (15:00), `evening` (18:00), `night` (21:00)
- Alternate separators: `07h00`, `9|00`, `9;00`, `9-00`
- Spoken: `"nine thirty"`, `"nine thirty-five"`, `"quarter past nine"`, `"quarter to nine"`, `"half past ten"`
- o'clock: `"3 o'clock"`
- Context: bare number after `at` resolves to PM time (`at 3` → `15:00:00`)

**Holidays (18)**
- US static: Christmas, New Year's Day, Independence Day, Halloween, Veterans Day, Juneteenth, Valentine's Day
- UK static: Boxing Day, Guy Fawkes Night (also: Bonfire Night), St. Patrick's Day
- US floating: Thanksgiving, Memorial Day, Labor Day, MLK Day, Presidents Day, May Day
- UK floating: Spring Bank Holiday, Summer Bank Holiday
- Auto-advance to next year if holiday has already passed

**Timezone**
- Named abbreviations: `EST`, `EDT`, `CST`, `CDT`, `MST`, `MDT`, `PST`, `PDT`, `CET`, `CEST`, `BST`, `JST`, `IST`, `AEST`, `AEDT`
- IANA paths: `America/New_York` style
- Offsets: `+05:30`, `UTC+05:30`, `GMT-8`, `-0800`
- Z and UTC passthrough

**CLI**
- Default canonical output
- Custom format via `-f "YYYY-MM-DD HH:mm:ss TZ"`
- JSON token AST via `-o json`
- `--day-first` for DD/MM/YY numeric date disambiguation
- `--debug` for per-token score tracing
- Integrated test runner: `nattydate test [--verbose] [--test-file path]`

**Library**
- Public API: `tokenize_and_classify`, `format_custom`, `to_canonical`, `process`
- `ParseConfig` with `mock_now` for deterministic testing
- All token types serializable via serde

**Testing**
- 104-case JSON test suite with fixed `mock_now`
- `cargo test` runs the full suite as a unit test
- Named scoring constants throughout (no magic numbers)

---

[Unreleased]: https://github.com/troy74/nattydate/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/troy74/nattydate/releases/tag/v0.1.0
