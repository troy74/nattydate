# Contributing to NattyDate

Thanks for considering a contribution. NattyDate is intentionally small and deterministic — every change should preserve that character.

---

## Quick Start

```bash
git clone https://github.com/troy74/nattydate.git
cd nattydate
cargo build
cargo test          # runs all 104 test cases
cargo run -- test --verbose
```

Minimum Rust version: **1.85** (Rust 2024 edition). Use `rustup update stable` if `cargo build` fails.

---

## Project Layout

```
src/
  lib.rs      — full parser: normalize, tokenize, classify, score, resolve, format
  main.rs     — CLI wrapper + test runner
tests.json    — 104-case test suite with fixed mock_now
SKILL.md      — user-facing quick-reference
CLAUDE.md     — codebase guide for AI agents
AGENT.md      — same as CLAUDE.md
```

---

## How to Add a Dictionary Word

All recognized words live in `get_dict()` in `src/lib.rs`. Each entry is a `(&str, KnownToken)` pair.

**Example — adding "forthcoming" as a synonym for `Next`:**

```rust
("forthcoming", KnownToken::Modifier(Modifier::Next)),
```

Entries are matched case-insensitively (input is lowercased before lookup). Fuzzy matching handles minor misspellings automatically; explicit aliases like `"tmrw"` are for common abbreviations the fuzzy engine might not score high enough.

Add a corresponding test case in `tests.json` before opening the PR.

---

## How to Add a Holiday

1. Add a variant to the `Holiday` enum in `src/lib.rs`.
2. Add a lookup entry in `get_dict()`:
   ```rust
   ("anzacday", KnownToken::Holiday(Holiday::AnzacDay)),
   ```
3. Add phrase normalization in `normalize()` if needed:
   ```rust
   ("anzac day", "anzacday"),
   ```
4. Add a resolution arm in `resolve_holiday()`:
   ```rust
   Holiday::AnzacDay => NaiveDate::from_ymd_opt(year, 4, 25).unwrap(),
   ```
5. Add at least one test case in `tests.json`.

---

## How to Add a New Modifier Behaviour

Modifier resolution lives in `resolve()` in `src/lib.rs`. The `Weekday` arm is the primary place modifiers are consumed. `preprocess_ago_patterns()` handles the `N unit ago` pattern before the main loop.

To add a new structural pattern (e.g. `"in N days"`):
- Add the keyword to the dict if it's not already there
- Either handle it in `preprocess_ago_patterns()` or add a new pre-pass

---

## Scoring Constants

All magic numbers are named constants at the top of `src/lib.rs`. Before changing any threshold, run the full test suite and check that no cases regress. The `THRESHOLD_MARGIN` value (`0.09` rather than `0.10`) exists specifically to survive `f32` rounding — do not raise it above `0.099`.

---

## Test Cases

All changes that affect parsing behaviour **must** be accompanied by a test case in `tests.json`. Format:

```json
{ "input": "the string to parse", "expected": "formatted output", "format": "YYYY-MM-DD" }
```

The `mock_now` at the top of the file is `2026-03-18` (a Wednesday). Use this when computing expected values for relative expressions.

Run the suite:

```bash
cargo test                           # via Rust unit test
cargo run -- test --verbose          # via CLI runner
cargo run -- test --test-file path   # custom file
```

The CLI runner exits with code `1` on failure, so it is safe to use in CI.

---

## Code Style

- `cargo fmt` — enforced in CI
- `cargo clippy -- -D warnings` — enforced in CI
- Keep functions focused; the pipeline stages (`normalize`, `tokenize`, `classify`, `resolve`, `format`) should remain distinct
- Avoid adding dependencies without discussion — the small dep tree is intentional

---

## Pull Request Checklist

- [ ] `cargo fmt` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (all 104 cases)
- [ ] New behaviour is covered by at least one test case in `tests.json`
- [ ] CHANGELOG.md entry added under `[Unreleased]`
- [ ] Public API changes reflected in README.md and SKILL.md

---

## Licensing

By submitting a pull request you agree that your contribution is licensed under the same terms as NattyDate: **MIT OR Apache-2.0**.
