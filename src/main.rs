use clap::{Parser, ValueEnum};
use nattydate::{ParseConfig};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(author, version, about = "A lightweight natural language date preprocessor", long_about = None)]
struct Args {
    /// The input text to process (e.g., "tomorrow at 3pm", "2026-03-18T08:00:00Z"). Use "test" to run the test suite.
    text: String,

    /// Assume day comes first in ambiguous numeric dates (DD/MM/YYYY vs MM/DD/YYYY)
    #[arg(short, long)]
    pub day_first: bool,

    #[arg(long)]
    pub debug: bool,
    
    #[arg(short, long)]
    pub verbose: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Canonical)]
    output_format: OutputFormat,

    /// Custom output format string (e.g., "YYYY-MM-DD HH:mm:ss TZ")
    /// Supported placeholders: YYYY, YY, MM, DD, HH, mm, ss, TZ, Z, {RELATIVE}
    #[arg(short = 'f', long)]
    custom_format: Option<String>,
}

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    /// Canonical normalized string (default)
    Canonical,
    /// JSON array of classified tokens
    Json,
    /// Apply a custom template string
    Custom,
}

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

fn run_test_suite(verbose: bool) {
    let json_str = include_str!("../tests.json");
    let suite: TestSuite = serde_json::from_str(json_str).expect("Failed to parse tests.json");
    let mock_date = chrono::NaiveDate::parse_from_str(&suite.mock_now, "%Y-%m-%d").unwrap();
    
    let config = ParseConfig {
        day_first: false,
        resolve_dates: true,
        mock_now: Some(mock_date),
        debug: false,
    };

    println!("Running {} integrated tests (Mock Time: {})...", suite.cases.len(), suite.mock_now);
    if !verbose {
        println!("(Use --verbose to see individual test outputs)\n");
    }

    let mut passed = 0;
    let mut failed = 0;

    for (i, case) in suite.cases.iter().enumerate() {
        let tokens = nattydate::tokenize_and_classify(&case.input, &config);
        let output = nattydate::format_custom(&tokens, &case.format).trim().to_string();
        
        let is_pass = output == case.expected;
        if is_pass {
            passed += 1;
            if verbose {
                println!("✅ TEST {:02} PASS: '{}' -> '{}'", i + 1, case.input, output);
            }
        } else {
            failed += 1;
            println!("❌ TEST {:02} FAIL: '{}'", i + 1, case.input);
            println!("   Expected: '{}'", case.expected);
            println!("   Got:      '{}'", output);
        }
    }

    println!("\n=== Test Results ===");
    println!("Total:  {}", passed + failed);
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    if failed > 0 {
        std::process::exit(1);
    }
}

fn main() {
    let args = Args::parse();
    
    if args.text.trim().to_lowercase() == "test" {
        run_test_suite(args.verbose);
        return;
    }

    let config = ParseConfig { day_first: args.day_first, resolve_dates: true, mock_now: None, debug: args.debug };

    let tokens = nattydate::tokenize_and_classify(&args.text, &config);

    match args.output_format {
        OutputFormat::Canonical => {
            if let Some(fmt) = args.custom_format {
                println!("{}", nattydate::format_custom(&tokens, &fmt));
            } else {
                println!("{}", nattydate::to_canonical(tokens));
            }
        }
        OutputFormat::Json => {
            match serde_json::to_string_pretty(&tokens) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
        }
        OutputFormat::Custom => {
            if let Some(fmt) = args.custom_format {
                println!("{}", nattydate::format_custom(&tokens, &fmt));
            } else {
                eprintln!("Error: --custom-format (-f) must be provided when using --output-format custom");
                std::process::exit(1);
            }
        }
    }
}
