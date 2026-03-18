use clap::{Parser, ValueEnum};
use nattydate::{ParseConfig};
use chrono::{Datelike, Local, NaiveDate, Duration};

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
    let now = Local::now().date_naive();
    let year = now.year();
    let next_year = year + 1;

    let today_str = now.format("%Y-%m-%d").to_string();
    let tomorrow_str = (now + Duration::days(1)).format("%Y-%m-%d").to_string();
    let yesterday_str = (now - Duration::days(1)).format("%Y-%m-%d").to_string();
    
    // Helper to get next weekday
    let next_weekday = |wd: chrono::Weekday, weeks_ahead: i64| {
        let mut days_ahead = wd.num_days_from_monday() as i64 - now.weekday().num_days_from_monday() as i64;
        if days_ahead <= 0 {
            days_ahead += 7;
        }
        days_ahead += weeks_ahead * 7;
        (now + Duration::days(days_ahead)).format("%Y-%m-%d").to_string()
    };
    
    let next_monday = next_weekday(chrono::Weekday::Mon, 0);
    let monday_week = next_weekday(chrono::Weekday::Mon, 1);
    let next_friday = next_weekday(chrono::Weekday::Fri, 1);

    let tests = vec![
        // Absolutes
        ("2026-03-18T08:00:00Z", "2026-03-18 08:00:00 UTC".to_string()),
        ("7/6/26", "2026-07-06 00:00:00".to_string()),
        ("9-1", format!("{}-09-01 00:00:00", if NaiveDate::from_ymd_opt(year, 9, 1).unwrap() < now { next_year } else { year })),
        
        // Relatives
        ("tomorrow", format!("{} 00:00:00", tomorrow_str)),
        ("today", format!("{} 00:00:00", today_str)),
        ("yesterday", format!("{} 00:00:00", yesterday_str)),
        ("monday week", format!("{} 00:00:00", monday_week)),
        ("next friday", format!("{} 00:00:00", next_friday)),
        
        // Time parsing combinations
        ("tomorrow at 9-00", format!("{} 09:00:00", tomorrow_str)),
        ("tomorrow at 9:00", format!("{} 09:00:00", tomorrow_str)),
        ("tomorrow at 9;00", format!("{} 09:00:00", tomorrow_str)),
        ("tomorrow at 9|00", format!("{} 09:00:00", tomorrow_str)),
        ("tomorrow at 9:30", format!("{} 09:30:00", tomorrow_str)),
        ("monday week at 0700", format!("{} 07:00:00", monday_week)),
        ("monday week at 07h00", format!("{} 07:00:00", monday_week)),
        ("tomorrow 3 o'clock", format!("{} 15:00:00", tomorrow_str)),
        ("tomorrow at 15:00", format!("{} 15:00:00", tomorrow_str)),
        ("tomorrow at 3 pm", format!("{} 15:00:00", tomorrow_str)),
        ("tomorrow at 3 am", format!("{} 03:00:00", tomorrow_str)),
        ("tomorrow at one am", format!("{} 01:00:00", tomorrow_str)),
        ("tomorrow at nine thirty", format!("{} 09:30:00", tomorrow_str)),
        ("tomorrow at nine thirty-five", format!("{} 09:35:00", tomorrow_str)),
        ("tomorrow at half past ten", format!("{} 10:30:00", tomorrow_str)),
        ("tomorrow at quarter to nine", format!("{} 08:45:00", tomorrow_str)),
        ("tomorrow at quarter past nine", format!("{} 09:15:00", tomorrow_str)),
        
        // Day parts
        ("tomorrow morning", format!("{} 09:00:00", tomorrow_str)),
        ("tomorrow noon", format!("{} 12:00:00", tomorrow_str)),
        ("tomorrow afternoon", format!("{} 15:00:00", tomorrow_str)),
        ("tomorrow evening", format!("{} 18:00:00", tomorrow_str)),
        ("tomorrow night", format!("{} 21:00:00", tomorrow_str)),
        
        // Edge cases and replacements
        ("monday mrning at one am", format!("{} 01:00:00", next_monday)),
        ("monday mrning at nine thirty-five", format!("{} 09:35:00", next_monday)),
    ];

    let holiday_tests = vec![
        "christmas",
        "spring bank holiday",
        "fourth of july",
        "halloween",
        "thanksgiving",
        "memorial day",
        "labor day",
        "mlk day",
        "presidents day",
        "veterans day",
        "juneteenth",
        "valentines day",
        "boxing day",
        "guy fawkes night",
        "saint patricks day",
        "may day",
        "summer bank holiday",
    ];

    let config = nattydate::ParseConfig { day_first: false, resolve_dates: true };
    let mut passed = 0;
    let mut failed = 0;

    println!("Running {} specific tests...", tests.len());
    for (input, expected) in tests {
        let tokens = nattydate::tokenize_and_classify(input, &config);
        let output = nattydate::format_custom(&tokens, "YYYY-MM-DD HH:mm:ss Z").trim().to_string();
        if output == expected {
            passed += 1;
        } else {
            println!("❌ FAIL: '{}'\n   Expected: '{}'\n   Got:      '{}'", input, expected, output);
            failed += 1;
        }
    }

    println!("\nRunning {} holiday tests...", holiday_tests.len());
    for input in holiday_tests {
        let tokens = nattydate::tokenize_and_classify(input, &config);
        let output = nattydate::format_custom(&tokens, "YYYY-MM-DD");
        if output.len() == 10 && (output.starts_with(&year.to_string()) || output.starts_with(&next_year.to_string())) {
            passed += 1;
        } else {
            println!("❌ FAIL: Holiday '{}' parsed to '{}'", input, output);
            failed += 1;
        }
    }
    
    // Complex Edge Cases
    let edge_cases = vec![
        ("christmas day lunch at 1", "13:00:00"),
        ("tomorrow at 3pm EDT", "15:00:00 EDT"),
        ("nxt fri 14:00", "14:00:00"),
    ];

    println!("\nRunning {} edge cases...", edge_cases.len());
    for (input, expected_contains) in edge_cases {
        let tokens = nattydate::tokenize_and_classify(input, &config);
        let output = nattydate::format_custom(&tokens, "YYYY-MM-DD HH:mm:ss Z").trim().to_string();
        if output.contains(expected_contains) {
            passed += 1;
        } else {
            println!("❌ FAIL: Edge case '{}'\n   Expected to contain: '{}'\n   Got: '{}'", input, expected_contains, output);
            failed += 1;
        }
    }

    println!("\n=== Test Results ===");
    println!("Total: {}", passed + failed);
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    if failed > 0 {
        std::process::exit(1);
    }
}

fn main() {
    let args = Args::parse();
    
    if args.text.trim().to_lowercase() == "test" {
        run_test_suite();
        return;
    }

    let config = ParseConfig { day_first: args.day_first, resolve_dates: true };

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
