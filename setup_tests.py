import json

tests = {
  "mock_now": "2026-03-18",
  "cases": [
    { "input": "2026-03-18T08:00:00Z", "expected": "2026-03-18 08:00:00 UTC", "format": "YYYY-MM-DD HH:mm:ss Z" },
    { "input": "7/6/26", "expected": "2026-07-06 00:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "9-1", "expected": "2026-09-01 00:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow", "expected": "2026-03-19 00:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "today", "expected": "2026-03-18 00:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "yesterday", "expected": "2026-03-17 00:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "monday week", "expected": "2026-03-30 00:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "next friday", "expected": "2026-03-27 00:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 9-00", "expected": "2026-03-19 09:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 9:00", "expected": "2026-03-19 09:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 9;00", "expected": "2026-03-19 09:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 9|00", "expected": "2026-03-19 09:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 9:30", "expected": "2026-03-19 09:30:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "monday week at 0700", "expected": "2026-03-30 07:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "monday week at 07h00", "expected": "2026-03-30 07:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow 3 o'clock", "expected": "2026-03-19 15:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 15:00", "expected": "2026-03-19 15:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 3 pm", "expected": "2026-03-19 15:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 3 am", "expected": "2026-03-19 03:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at one am", "expected": "2026-03-19 01:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at nine thirty", "expected": "2026-03-19 09:30:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at nine thirty-five", "expected": "2026-03-19 09:35:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at half past ten", "expected": "2026-03-19 10:30:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at quarter to nine", "expected": "2026-03-19 08:45:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at quarter past nine", "expected": "2026-03-19 09:15:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow morning", "expected": "2026-03-19 09:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow noon", "expected": "2026-03-19 12:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow afternoon", "expected": "2026-03-19 15:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow evening", "expected": "2026-03-19 18:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow night", "expected": "2026-03-19 21:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "monday mrning at one am", "expected": "2026-03-23 01:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "monday mrning at nine thirty-five", "expected": "2026-03-23 09:35:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "christmas", "expected": "2026-12-25", "format": "YYYY-MM-DD" },
    { "input": "spring bank holiday", "expected": "2026-05-25", "format": "YYYY-MM-DD" },
    { "input": "fourth of july", "expected": "2026-07-04", "format": "YYYY-MM-DD" },
    { "input": "halloween", "expected": "2026-10-31", "format": "YYYY-MM-DD" },
    { "input": "thanksgiving", "expected": "2026-11-26", "format": "YYYY-MM-DD" },
    { "input": "memorial day", "expected": "2026-05-25", "format": "YYYY-MM-DD" },
    { "input": "labor day", "expected": "2026-09-07", "format": "YYYY-MM-DD" },
    { "input": "mlk day", "expected": "2027-01-18", "format": "YYYY-MM-DD" },
    { "input": "presidents day", "expected": "2027-02-15", "format": "YYYY-MM-DD" },
    { "input": "veterans day", "expected": "2026-11-11", "format": "YYYY-MM-DD" },
    { "input": "juneteenth", "expected": "2026-06-19", "format": "YYYY-MM-DD" },
    { "input": "valentines day", "expected": "2027-02-14", "format": "YYYY-MM-DD" },
    { "input": "boxing day", "expected": "2026-12-26", "format": "YYYY-MM-DD" },
    { "input": "guy fawkes night", "expected": "2026-11-05", "format": "YYYY-MM-DD" },
    { "input": "saint patricks day", "expected": "2027-03-17", "format": "YYYY-MM-DD" },
    { "input": "may day", "expected": "2026-05-04", "format": "YYYY-MM-DD" },
    { "input": "summer bank holiday", "expected": "2026-08-31", "format": "YYYY-MM-DD" },
    { "input": "christmas day lunch at 1", "expected": "2026-12-25 13:00:00", "format": "YYYY-MM-DD HH:mm:ss" },
    { "input": "tomorrow at 3pm EDT", "expected": "2026-03-19 15:00:00 EDT", "format": "YYYY-MM-DD HH:mm:ss TZ" },
    { "input": "nxt fri 14:00", "expected": "2026-03-27 14:00:00", "format": "YYYY-MM-DD HH:mm:ss" }
  ]
}

with open("tests.json", "w") as f:
    json.dump(tests, f, indent=2)

import re
with open("src/lib.rs", "r") as f:
    lib = f.read()

# Update ParseConfig
parse_config_old = r"""pub struct ParseConfig {
    pub day_first: bool,
    pub resolve_dates: bool,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            day_first: false,
            resolve_dates: true,
        }
    }
}"""
parse_config_new = """pub struct ParseConfig {
    pub day_first: bool,
    pub resolve_dates: bool,
    pub mock_now: Option<NaiveDate>,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            day_first: false,
            resolve_dates: true,
            mock_now: None,
        }
    }
}"""
lib = lib.replace(parse_config_old, parse_config_new)

# Update resolve
resolve_old = """pub fn resolve(tokens: Vec<Token>) -> Vec<Token> {
    let now = Local::now().date_naive();"""
resolve_new = """pub fn resolve(tokens: Vec<Token>, config: &ParseConfig) -> Vec<Token> {
    let now = config.mock_now.unwrap_or_else(|| Local::now().date_naive());"""
lib = lib.replace(resolve_old, resolve_new)

# Update tokenize_and_classify
tokenize_old = """    if config.resolve_dates {
        resolve(classified)
    } else {
        classified
    }"""
tokenize_new = """    if config.resolve_dates {
        resolve(classified, config)
    } else {
        classified
    }"""
lib = lib.replace(tokenize_old, tokenize_new)

# Replace Tests Module
tests_mod_old = r"mod tests \{.*"
tests_mod_new = """mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct TestSuite {
        mock_now: String,
        cases: Vec<TestCase>,
    }

    #[derive(Deserialize)]
    struct TestCase {
        input: String,
        expected: String,
        format: String,
    }

    #[test]
    fn run_json_test_suite() {
        let json_str = include_str!("../tests.json");
        let suite: TestSuite = serde_json::from_str(json_str).expect("Failed to parse JSON");
        
        let mock_date = chrono::NaiveDate::parse_from_str(&suite.mock_now, "%Y-%m-%d").unwrap();
        
        let config = ParseConfig {
            day_first: false,
            resolve_dates: true,
            mock_now: Some(mock_date),
        };

        for case in suite.cases {
            let tokens = tokenize_and_classify(&case.input, &config);
            let output = format_custom(&tokens, &case.format).trim().to_string();
            assert_eq!(output, case.expected, "Failed on input: {}", case.input);
        }
    }
}"""
lib = re.sub(tests_mod_old, tests_mod_new, lib, flags=re.DOTALL)

with open("src/lib.rs", "w") as f:
    f.write(lib)

with open("src/main.rs", "r") as f:
    main_rs = f.read()

# Replace run_test_suite logic in main.rs to run the library tests using process execution,
# or simply tell the user how to run it.
main_old = r"fn run_test_suite.*?fn main"
main_new = """fn run_test_suite() {
    println!("Running integrated test suite via Cargo...");
    let status = std::process::Command::new("cargo")
        .arg("test")
        .status()
        .expect("Failed to execute cargo test");
    if !status.success() {
        std::process::exit(1);
    }
}

fn main"""
main_rs = re.sub(main_old, main_new, main_rs, flags=re.DOTALL)

# Also fix config instantiation in main.rs
main_rs = main_rs.replace("ParseConfig { day_first: args.day_first, resolve_dates: true }", "ParseConfig { day_first: args.day_first, resolve_dates: true, mock_now: None }")

with open("src/main.rs", "w") as f:
    f.write(main_rs)

