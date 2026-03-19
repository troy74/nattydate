# 📅 NattyDate

A lightweight, ultra-fast, and deterministic natural language date preprocessor written in pure Rust.

Unlike massive NLP libraries or heavyweight resolvers, **NattyDate** operates as a strict pipeline designed to normalize, score, and evaluate messy human date and time inputs into predictable, canonical structures (like ISO-8601 or custom `strftime`-style templates).

## ✨ Features

- **Pure Rust:** No Python, no massive runtimes, no machine learning bloat.
- **Fuzzy & Contextual Scoring:** Algorithmically evaluates ambiguity using a rigorous confidence tier system (`0.0` - `1.0`), automatically discarding noise while sliding context windows across words to infer meaning (e.g. knowing `"3"` means `15:00:00` when placed after `"tomorrow"`).
- **Public Holidays:** Deep native support for 17+ floating and static US/UK public holidays (e.g., `"thanksgiving"`, `"spring bank holiday"`).
- **Time String Inference:** Natively parses complex literal numbers (`"07h00"`, `"9|00"`, `"nine thirty-five"`, `"quarter past ten"`) without destroying genuine dates (like `"9-1"`).
- **Custom Formats:** Output your processed tokens into strict standard letter forms (e.g. `YYYY-MM-DD HH:mm:ss TZ`).

## 🚀 Installation & Usage

To build NattyDate from source:

```bash
git clone https://github.com/troy74/nattydate.git
cd nattydate
cargo build --release
```

The compiled macOS executable will be available at `./target/release/nattydate`. 

*(A pre-packaged `./build` folder is also included containing the binary alongside its default testing suite)*.

### Basic Commands

Process a simple date:
```bash
./nattydate "tomorrow at 3pm EDT"
# Output: tomorrow at 15:00:00 EDT
```

Apply a strict output format template:
```bash
./nattydate "monday mrning at nine thirty-five" -f "YYYY-MM-DD HH:mm:ss"
# Output: 2026-03-23 09:35:00
```

Output the raw AST evaluation in JSON format for downstream pipelines:
```bash
./nattydate "nxt fri 14:00" -o json
```

## 🧪 Testing

NattyDate ships with an integrated, highly deterministic testing framework that explicitly bypasses the `Local::now()` clock to prevent fragile tests breaking every 24 hours.

Run the default 52-case testing suite (using the bundled `tests.json`):
```bash
./nattydate test
```

Run with verbose line-item output:
```bash
./nattydate test -v
```

Supply your own custom mock-time validation suite:
```bash
./nattydate test --test-file custom_evals.json
```
