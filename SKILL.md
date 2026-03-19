---
name: nattydate
description: Specialized guidance for interfacing with the NattyDate Rust CLI tool to parse, format, and evaluate natural language dates.
---

# <skill> NattyDate Parser

This skill provides instructions for correctly utilizing the NattyDate CLI tool. NattyDate is a deterministic preprocessor that transforms messy natural language timestamps into normalized canonical templates, JSON ASTs, or strictly formatted strings using a fuzzy context-scoring engine.

## <instructions>

### 1. Basic Interpretation
When requested to pre-process a date using NattyDate, simply invoke the compiled binary against the target string.

```bash
nattydate "tomorrow at 3pm"
```

The system will naturally fall back to the most recent `Local::now()` timestamp for any relative queries.

### 2. Custom Formatting (`-f`, `--custom-format`)
NattyDate can output structured timestamp strings bypassing its default `canonical` format by utilizing standard replacement tokens (e.g. `YYYY`, `MM`, `DD`, `HH`, `mm`, `ss`, `TZ`).

Always prioritize explicitly utilizing the `-f` flag when downstream systems expect a strictly formatted timestamp rather than the verbose canonical AST map.

```bash
nattydate "monday mrning at 9:30" -f "YYYY-MM-DD HH:mm:ss"
# Yields: 2026-03-23 09:30:00
```

### 3. Testing Context (`test`)
If you need to evaluate new capabilities or verify that relative dates are mathematically mapping to expectations properly, invoke the built-in testing framework instead of passing a literal string.

```bash
nattydate test -v
```

**Never test NattyDate against the generic `--custom-format` engine without first injecting a deterministic `tests.json` file if relative outputs are required!**
Passing custom JSON mock-files directly evaluates deterministic outputs free from system-clock rot:

```bash
nattydate test --test-file ./my_eval_suite.json
```

### 4. Disambiguation
NattyDate is highly capable at fuzzy disambiguation using prefix-scoring and Levenshtein distances:
- `"mrning"`, `"evning"`
- `"nxt fri 14:00"`
- `"9|00"`, `"07h00"`
- `"quarter past nine"`

Do not aggressively normalize strings *before* supplying them to NattyDate unless they violate fundamental spacing conventions.
