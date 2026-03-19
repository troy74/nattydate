use clap::{Parser, ValueEnum};
use nattydate::{ParseConfig};

#[derive(Parser, Debug)]
#[command(author, version, about = "A lightweight natural language date preprocessor", long_about = None)]
struct Args {
    /// The input text to process (e.g., "tomorrow at 3pm", "2026-03-18T08:00:00Z"). Use "test" to run the test suite.
    text: String,

    /// Assume day comes first in ambiguous numeric dates (DD/MM/YYYY vs MM/DD/YYYY)
    #[arg(short, long)]
    day_first: bool,

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

fn run_test_suite() {
    println!("Running integrated test suite via Cargo...");
    let status = std::process::Command::new("cargo")
        .arg("test")
        .status()
        .expect("Failed to execute cargo test");
    if !status.success() {
        std::process::exit(1);
    }
}

fn main() {
    let args = Args::parse();
    
    if args.text.trim().to_lowercase() == "test" {
        run_test_suite();
        return;
    }

    let config = ParseConfig { day_first: args.day_first, resolve_dates: true, mock_now: None };

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
