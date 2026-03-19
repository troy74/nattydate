use chrono::{Datelike, Duration, Local, NaiveDate};
use std::collections::HashMap;
use std::sync::OnceLock;
use strsim::levenshtein;

// ── Scoring constants ────────────────────────────────────────────────────────
const SCORE_EXACT: f32 = 1.0; // dictionary exact match (non-time)
const SCORE_DICT_TIME: f32 = 0.9; // dictionary exact match for named times
const SCORE_EXPLICIT: f32 = 0.95; // unambiguous: ISO, HH:MM, am/pm
const SCORE_NUMERIC: f32 = 0.9; // plain integer candidate
const SCORE_DATE_YMD: f32 = 0.95; // three-part date with year, unambiguous
const SCORE_DATE_YMD_AMBIG: f32 = 0.9; // three-part date with year, ambiguous m/d order
const SCORE_DATE_MD: f32 = 0.75; // two-part date (no year, at least one part > 12)
const SCORE_DATE_AMBIG: f32 = 0.7; // two-part date, both parts ≤ 12
const SCORE_TIME_COLON: f32 = 0.95; // HH:MM with colon separator
const SCORE_TIME_DEFAULT: f32 = 0.8; // time with other separators, no am/pm
const SCORE_TIME_SINGLE: f32 = 0.7; // bare hour with no suffix

// ── Fuzzy-match constants ────────────────────────────────────────────────────
const FUZZY_BONUS_PREFIX: f32 = 0.1; // candidate word starts with the query
const FUZZY_BONUS_FIRST_CHAR: f32 = 0.05; // first character matches
const FUZZY_MAX_SCORE: f32 = 0.95; // cap so fuzzy never equals exact
const FUZZY_MIN_ACCEPT: f32 = 0.65; // minimum to surface a fuzzy candidate
const FUZZY_MAX_LEN_DIFF: i32 = 2; // skip pairs whose lengths differ by more

// ── Context-boost constants ──────────────────────────────────────────────────
const BOOST_TEMPORAL_ADJ: f32 = 0.1; // Time adjacent to Date/TZ (or vice-versa)
const BOOST_MODIFIER_CTX: f32 = 0.2; // Weekday candidate following a Modifier
const BOOST_RELATIVE_TIME: f32 = 0.15; // RelativeDay candidate adjacent to a Time token
const BOOST_NUM_UNIT_PAIR: f32 = 0.2; // Number directly before a Unit token

// ── Classifier thresholds ────────────────────────────────────────────────────
const THRESHOLD_PROMOTE: f32 = 0.65; // Unknown token promoted to Known
const THRESHOLD_KNOWN: f32 = 0.65; // Single-candidate token accepted as Known
const THRESHOLD_MARGIN: f32 = 0.09; // Required score gap before top candidate wins (kept below 0.1 to survive f32 rounding)
const RECOMBINE_MIN: f32 = 0.8; // Min score for a recombined token to win
const RECOMBINE_GAIN: f32 = 0.15; // Recombined score must beat per-token avg by this

#[derive(Debug, Clone)]
pub struct ParseConfig {
    pub day_first: bool,
    pub resolve_dates: bool,
    pub mock_now: Option<NaiveDate>,
    pub debug: bool,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            day_first: false,
            resolve_dates: true,
            mock_now: None,
            debug: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RawToken {
    Word(String),
    Number(String),
    DateNumeric(String),
    Time(String),
    TimeZone(String),
    Separator(String),
}

impl RawToken {
    pub fn as_str(&self) -> &str {
        match self {
            RawToken::Word(s) => s,
            RawToken::Number(s) => s,
            RawToken::DateNumeric(s) => s,
            RawToken::Time(s) => s,
            RawToken::TimeZone(s) => s,
            RawToken::Separator(s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Month {
    Jan,
    Feb,
    Mar,
    Apr,
    May,
    Jun,
    Jul,
    Aug,
    Sep,
    Oct,
    Nov,
    Dec,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RelativeDay {
    Today,
    Tomorrow,
    Yesterday,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Modifier {
    Next,
    Last,
    This,
    Ago,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Unit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum TimeZoneKind {
    Z,
    Utc,
    Offset { minutes_total: i16 },
    Named(String),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Holiday {
    Christmas,
    NewYearsDay,
    IndependenceDay,
    Halloween,
    Thanksgiving,
    MemorialDay,
    LaborDay,
    MlkDay,
    PresidentsDay,
    VeteransDay,
    Juneteenth,
    ValentinesDay,
    BoxingDay,
    GuyFawkes,
    StPatricksDay,
    MayDay,
    SpringBankHoliday,
    SummerBankHoliday,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum KnownToken {
    Weekday(Weekday),
    Month(Month),
    RelativeDay(RelativeDay),
    Modifier(Modifier),
    Unit(Unit),
    At,
    Number(i32),
    Time {
        hour: u8,
        min: u8,
        sec: Option<u8>,
        formatted: String,
    },
    DateNumeric {
        y: Option<i32>,
        m: u8,
        d: u8,
    },
    TimeZone(TimeZoneKind),
    Holiday(Holiday),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ScoredToken {
    pub token: KnownToken,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Token {
    Known(ScoredToken),
    Unknown {
        word: String,
        candidates: Vec<ScoredToken>,
    },
    Noise(String),
}

pub fn get_dict() -> &'static [(&'static str, KnownToken)] {
    static DICT: OnceLock<Vec<(&'static str, KnownToken)>> = OnceLock::new();
    DICT.get_or_init(|| {
        vec![
            ("monday", KnownToken::Weekday(Weekday::Monday)),
            ("mon", KnownToken::Weekday(Weekday::Monday)),
            ("tuesday", KnownToken::Weekday(Weekday::Tuesday)),
            ("tue", KnownToken::Weekday(Weekday::Tuesday)),
            ("tues", KnownToken::Weekday(Weekday::Tuesday)),
            ("wednesday", KnownToken::Weekday(Weekday::Wednesday)),
            ("wed", KnownToken::Weekday(Weekday::Wednesday)),
            ("thursday", KnownToken::Weekday(Weekday::Thursday)),
            ("thu", KnownToken::Weekday(Weekday::Thursday)),
            ("thurs", KnownToken::Weekday(Weekday::Thursday)),
            ("friday", KnownToken::Weekday(Weekday::Friday)),
            ("fri", KnownToken::Weekday(Weekday::Friday)),
            ("frday", KnownToken::Weekday(Weekday::Friday)),
            ("fryday", KnownToken::Weekday(Weekday::Friday)),
            ("saturday", KnownToken::Weekday(Weekday::Saturday)),
            ("sat", KnownToken::Weekday(Weekday::Saturday)),
            ("sunday", KnownToken::Weekday(Weekday::Sunday)),
            ("sun", KnownToken::Weekday(Weekday::Sunday)),
            ("january", KnownToken::Month(Month::Jan)),
            ("jan", KnownToken::Month(Month::Jan)),
            ("february", KnownToken::Month(Month::Feb)),
            ("feb", KnownToken::Month(Month::Feb)),
            ("march", KnownToken::Month(Month::Mar)),
            ("mar", KnownToken::Month(Month::Mar)),
            ("april", KnownToken::Month(Month::Apr)),
            ("apr", KnownToken::Month(Month::Apr)),
            ("may", KnownToken::Month(Month::May)),
            ("june", KnownToken::Month(Month::Jun)),
            ("jun", KnownToken::Month(Month::Jun)),
            ("july", KnownToken::Month(Month::Jul)),
            ("jul", KnownToken::Month(Month::Jul)),
            ("august", KnownToken::Month(Month::Aug)),
            ("aug", KnownToken::Month(Month::Aug)),
            ("september", KnownToken::Month(Month::Sep)),
            ("sep", KnownToken::Month(Month::Sep)),
            ("sept", KnownToken::Month(Month::Sep)),
            ("october", KnownToken::Month(Month::Oct)),
            ("oct", KnownToken::Month(Month::Oct)),
            ("november", KnownToken::Month(Month::Nov)),
            ("nov", KnownToken::Month(Month::Nov)),
            ("december", KnownToken::Month(Month::Dec)),
            ("dec", KnownToken::Month(Month::Dec)),
            ("today", KnownToken::RelativeDay(RelativeDay::Today)),
            ("tomorrow", KnownToken::RelativeDay(RelativeDay::Tomorrow)),
            ("tmrw", KnownToken::RelativeDay(RelativeDay::Tomorrow)),
            ("tmr", KnownToken::RelativeDay(RelativeDay::Tomorrow)),
            ("tomorow", KnownToken::RelativeDay(RelativeDay::Tomorrow)),
            ("yesterday", KnownToken::RelativeDay(RelativeDay::Yesterday)),
            ("next", KnownToken::Modifier(Modifier::Next)),
            ("nxt", KnownToken::Modifier(Modifier::Next)),
            ("last", KnownToken::Modifier(Modifier::Last)),
            ("lst", KnownToken::Modifier(Modifier::Last)),
            ("this", KnownToken::Modifier(Modifier::This)),
            ("past", KnownToken::Modifier(Modifier::Last)),
            ("previous", KnownToken::Modifier(Modifier::Last)),
            ("prev", KnownToken::Modifier(Modifier::Last)),
            ("prior", KnownToken::Modifier(Modifier::Last)),
            ("following", KnownToken::Modifier(Modifier::Next)),
            ("ago", KnownToken::Modifier(Modifier::Ago)),
            ("day", KnownToken::Unit(Unit::Day)),
            ("days", KnownToken::Unit(Unit::Day)),
            ("week", KnownToken::Unit(Unit::Week)),
            ("weeks", KnownToken::Unit(Unit::Week)),
            ("month", KnownToken::Unit(Unit::Month)),
            ("months", KnownToken::Unit(Unit::Month)),
            ("year", KnownToken::Unit(Unit::Year)),
            ("years", KnownToken::Unit(Unit::Year)),
            ("at", KnownToken::At),
            ("on", KnownToken::At),
            ("in", KnownToken::At),
            (
                "morning",
                KnownToken::Time {
                    hour: 9,
                    min: 0,
                    sec: None,
                    formatted: "09:00:00".to_string(),
                },
            ),
            (
                "mrning",
                KnownToken::Time {
                    hour: 9,
                    min: 0,
                    sec: None,
                    formatted: "09:00:00".to_string(),
                },
            ),
            (
                "noon",
                KnownToken::Time {
                    hour: 12,
                    min: 0,
                    sec: None,
                    formatted: "12:00:00".to_string(),
                },
            ),
            (
                "afternoon",
                KnownToken::Time {
                    hour: 15,
                    min: 0,
                    sec: None,
                    formatted: "15:00:00".to_string(),
                },
            ),
            (
                "evening",
                KnownToken::Time {
                    hour: 18,
                    min: 0,
                    sec: None,
                    formatted: "18:00:00".to_string(),
                },
            ),
            (
                "evning",
                KnownToken::Time {
                    hour: 18,
                    min: 0,
                    sec: None,
                    formatted: "18:00:00".to_string(),
                },
            ),
            (
                "night",
                KnownToken::Time {
                    hour: 21,
                    min: 0,
                    sec: None,
                    formatted: "21:00:00".to_string(),
                },
            ),
            (
                "nite",
                KnownToken::Time {
                    hour: 21,
                    min: 0,
                    sec: None,
                    formatted: "21:00:00".to_string(),
                },
            ),
            ("newyearsday", KnownToken::Holiday(Holiday::NewYearsDay)),
            ("christmas", KnownToken::Holiday(Holiday::Christmas)),
            (
                "independenceday",
                KnownToken::Holiday(Holiday::IndependenceDay),
            ),
            ("halloween", KnownToken::Holiday(Holiday::Halloween)),
            ("thanksgiving", KnownToken::Holiday(Holiday::Thanksgiving)),
            ("memorialday", KnownToken::Holiday(Holiday::MemorialDay)),
            ("laborday", KnownToken::Holiday(Holiday::LaborDay)),
            ("mlkday", KnownToken::Holiday(Holiday::MlkDay)),
            ("presidentsday", KnownToken::Holiday(Holiday::PresidentsDay)),
            ("veteransday", KnownToken::Holiday(Holiday::VeteransDay)),
            ("juneteenth", KnownToken::Holiday(Holiday::Juneteenth)),
            ("valentinesday", KnownToken::Holiday(Holiday::ValentinesDay)),
            ("boxingday", KnownToken::Holiday(Holiday::BoxingDay)),
            ("guyfawkes", KnownToken::Holiday(Holiday::GuyFawkes)),
            ("stpatricksday", KnownToken::Holiday(Holiday::StPatricksDay)),
            ("mayday", KnownToken::Holiday(Holiday::MayDay)),
            (
                "springbankholiday",
                KnownToken::Holiday(Holiday::SpringBankHoliday),
            ),
            (
                "summerbankholiday",
                KnownToken::Holiday(Holiday::SummerBankHoliday),
            ),
        ]
    })
}

fn get_dict_map() -> &'static HashMap<&'static str, KnownToken> {
    static MAP: OnceLock<HashMap<&'static str, KnownToken>> = OnceLock::new();
    MAP.get_or_init(|| get_dict().iter().map(|(k, v)| (*k, v.clone())).collect())
}

pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    let lower = input.to_lowercase();
    let replaced = lower
        .replace('@', " at ")
        .replace('\'', "")
        .replace("o clock", "oclock")
        .replace(['|', ';'], ":");

    let phrases = [
        ("new years day", "newyearsday"),
        ("new years", "newyearsday"),
        ("christmas day", "christmas"),
        ("independence day", "independenceday"),
        ("fourth of july", "independenceday"),
        ("4th of july", "independenceday"),
        ("memorial day", "memorialday"),
        ("labor day", "laborday"),
        ("martin luther king day", "mlkday"),
        ("mlk day", "mlkday"),
        ("presidents day", "presidentsday"),
        ("veterans day", "veteransday"),
        ("valentines day", "valentinesday"),
        ("boxing day", "boxingday"),
        ("guy fawkes night", "guyfawkes"),
        ("guy fawkes", "guyfawkes"),
        ("bonfire night", "guyfawkes"),
        ("saint patricks day", "stpatricksday"),
        ("st patricks day", "stpatricksday"),
        ("may day", "mayday"),
        ("spring bank holiday", "springbankholiday"),
        ("summer bank holiday", "summerbankholiday"),
    ];
    let mut replaced_phrases = replaced;
    for (phrase, replacement) in phrases {
        replaced_phrases = replaced_phrases.replace(phrase, replacement);
    }

    let chars = replaced_phrases.chars().peekable();
    let mut in_space = false;

    for c in chars {
        if c == ',' || c == '!' || c == '?' || c.is_whitespace() {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            out.push(c);
            in_space = false;
        }
    }

    out.trim().to_string()
}

fn text_to_number(s: &str) -> Option<u32> {
    match s {
        "one" | "1" => Some(1),
        "two" | "2" | "too" => Some(2),
        "three" | "3" => Some(3),
        "four" | "4" => Some(4),
        "five" | "5" => Some(5),
        "six" | "6" => Some(6),
        "seven" | "7" => Some(7),
        "eight" | "8" => Some(8),
        "nine" | "9" => Some(9),
        "ten" | "10" => Some(10),
        "eleven" | "11" => Some(11),
        "twelve" | "12" => Some(12),
        "thirteen" | "13" => Some(13),
        "fourteen" | "14" => Some(14),
        "fifteen" | "15" => Some(15),
        "sixteen" | "16" => Some(16),
        "seventeen" | "17" => Some(17),
        "eighteen" | "18" => Some(18),
        "nineteen" | "19" => Some(19),
        "twenty" | "20" => Some(20),
        "thirty" | "30" => Some(30),
        "forty" | "40" => Some(40),
        "fifty" | "50" => Some(50),
        _ => None,
    }
}

fn convert_compound_numbers(words: Vec<String>) -> Vec<String> {
    let mut i = 0;
    let mut res = Vec::new();
    while i < words.len() {
        let w = &words[i];
        if let Some(n1) = text_to_number(w) {
            if i + 1 < words.len()
                && let Some(n2) = text_to_number(&words[i + 1])
                && (n1 == 20 || n1 == 30 || n1 == 40 || n1 == 50)
                && n2 < 10
            {
                res.push((n1 + n2).to_string());
                i += 2;
                continue;
            }
            res.push(n1.to_string());
        } else if w.contains('-') {
            let parts: Vec<&str> = w.split('-').collect();
            if parts.len() == 2
                && let (Some(n1), Some(n2)) = (text_to_number(parts[0]), text_to_number(parts[1]))
                && (n1 == 20 || n1 == 30 || n1 == 40 || n1 == 50)
                && n2 < 10
            {
                res.push((n1 + n2).to_string());
                i += 1;
                continue;
            }
            res.push(w.to_string());
        } else {
            res.push(w.to_string());
        }
        i += 1;
    }
    res
}

fn resolve_time_phrases(words: Vec<String>) -> Vec<String> {
    let mut i = 0;
    let mut res = Vec::new();
    while i < words.len() {
        if words[i] == "half" && i + 2 < words.len() && words[i + 1] == "past" {
            if let Ok(h) = words[i + 2].parse::<u8>() {
                res.push(format!("{:02}:30", h));
                i += 3;
                continue;
            }
        } else if words[i] == "quarter" && i + 2 < words.len() {
            if words[i + 1] == "past" {
                if let Ok(h) = words[i + 2].parse::<u8>() {
                    res.push(format!("{:02}:15", h));
                    i += 3;
                    continue;
                }
            } else if words[i + 1] == "to"
                && let Ok(h) = words[i + 2].parse::<u8>()
            {
                let h_prev = if h == 1 || h == 0 { 12 } else { h - 1 };
                res.push(format!("{:02}:45", h_prev));
                i += 3;
                continue;
            }
        }
        res.push(words[i].clone());
        i += 1;
    }
    res
}

pub fn tokenize(input: &str) -> Vec<RawToken> {
    let words: Vec<String> = input.split_whitespace().map(|s| s.to_string()).collect();
    let words = convert_compound_numbers(words);
    let words = resolve_time_phrases(words);

    words
        .into_iter()
        .map(|s| {
            if s.chars().all(|c| c.is_ascii_digit()) {
                RawToken::Number(s)
            } else if (s.contains(':') && s.contains('-') && (s.contains('t') || s.contains('z')))
                || ((s.contains('/') || s.contains('-') || s.contains('.'))
                    && s.chars().any(|c| c.is_ascii_digit())
                    && !s.contains(':'))
            {
                RawToken::DateNumeric(s)
            } else if s.contains(':')
                || s.contains('.')
                || s.contains(';')
                || s.contains('|')
                || s.contains('h')
                || s.ends_with("am")
                || s.ends_with("pm")
            {
                if s.chars().any(|c| c.is_ascii_digit()) {
                    RawToken::Time(s)
                } else {
                    RawToken::Word(s)
                }
            } else if s == "z"
                || s == "utc"
                || s == "gmt"
                || s.starts_with("utc+")
                || s.starts_with("utc-")
                || s.starts_with("gmt+")
                || s.starts_with("gmt-")
                || ((s.starts_with('+') || s.starts_with('-'))
                    && s.len() >= 3
                    && s[1..].chars().all(|c| c.is_ascii_digit() || c == ':'))
                || (s.contains('/') && !s.chars().any(|c| c.is_ascii_digit()))
            {
                RawToken::TimeZone(s)
            } else {
                RawToken::Word(s)
            }
        })
        .collect()
}

pub fn parse_date_numeric_scored(s: &str, config: &ParseConfig) -> Option<(KnownToken, f32)> {
    let parts: Vec<&str> = s.split(['/', '-', '.']).collect();
    if parts.len() == 3 {
        let p1 = parts[0].parse::<i32>().ok()?;
        let p2 = parts[1].parse::<i32>().ok()?;
        let mut p3 = parts[2].parse::<i32>().ok()?;

        if p1 == 0 || p2 == 0 || p3 == 0 {
            return None;
        }

        if p1 < 1000 && p3 < 100 {
            p3 += 2000;
        }

        if p1 > 1000 {
            if p2 > 12 && p3 <= 12 {
                return Some((
                    KnownToken::DateNumeric {
                        y: Some(p1),
                        m: p3 as u8,
                        d: p2 as u8,
                    },
                    SCORE_DATE_YMD,
                ));
            }
            if p2 <= 12 && p3 <= 31 {
                return Some((
                    KnownToken::DateNumeric {
                        y: Some(p1),
                        m: p2 as u8,
                        d: p3 as u8,
                    },
                    SCORE_DATE_YMD,
                ));
            }
            return None;
        }

        if p3 > 1000 {
            if p1 <= 12 && p2 <= 12 {
                if config.day_first {
                    return Some((
                        KnownToken::DateNumeric {
                            y: Some(p3),
                            m: p2 as u8,
                            d: p1 as u8,
                        },
                        SCORE_DATE_YMD_AMBIG,
                    ));
                } else {
                    return Some((
                        KnownToken::DateNumeric {
                            y: Some(p3),
                            m: p1 as u8,
                            d: p2 as u8,
                        },
                        SCORE_DATE_YMD_AMBIG,
                    ));
                }
            } else if config.day_first || p1 > 12 {
                if p2 <= 12 && p1 <= 31 {
                    return Some((
                        KnownToken::DateNumeric {
                            y: Some(p3),
                            m: p2 as u8,
                            d: p1 as u8,
                        },
                        SCORE_DATE_YMD_AMBIG,
                    ));
                }
            } else if p1 <= 12 && p2 <= 31 {
                return Some((
                    KnownToken::DateNumeric {
                        y: Some(p3),
                        m: p1 as u8,
                        d: p2 as u8,
                    },
                    SCORE_DATE_YMD_AMBIG,
                ));
            }
            return None;
        }
    } else if parts.len() == 2 {
        let p1 = parts[0].parse::<i32>().ok()?;
        let p2 = parts[1].parse::<i32>().ok()?;

        if p1 == 0 || p2 == 0 || p1 > 31 || p2 > 31 {
            return None;
        }

        let mut score = SCORE_DATE_MD;
        if p1 <= 12 && p2 <= 12 {
            score = SCORE_DATE_AMBIG;
        }

        if config.day_first || p1 > 12 {
            if p2 <= 12 && p1 <= 31 {
                return Some((
                    KnownToken::DateNumeric {
                        y: None,
                        m: p2 as u8,
                        d: p1 as u8,
                    },
                    score,
                ));
            }
        } else if p1 <= 12 && p2 <= 31 {
            return Some((
                KnownToken::DateNumeric {
                    y: None,
                    m: p1 as u8,
                    d: p2 as u8,
                },
                score,
            ));
        }
    }
    None
}

pub fn parse_time_scored(s: &str) -> Option<(KnownToken, f32)> {
    let lower = s.to_lowercase();
    let is_pm = lower.ends_with("pm") || lower.ends_with("p.m.");
    let is_am = lower.ends_with("am") || lower.ends_with("a.m.");
    let is_oclock =
        lower.ends_with("oclock") || lower.ends_with("o'clock") || lower.ends_with("clock");
    let cleaned = lower
        .trim_end_matches("pm")
        .trim_end_matches("am")
        .trim_end_matches("p.m.")
        .trim_end_matches("a.m.")
        .trim_end_matches("oclock")
        .trim_end_matches("o'clock")
        .trim_end_matches("clock")
        .trim();

    if cleaned.len() == 4 && cleaned.chars().all(|c| c.is_ascii_digit()) {
        let hour = cleaned[0..2].parse::<u8>().ok()?;
        let min = cleaned[2..4].parse::<u8>().ok()?;
        if hour < 24 && min < 60 {
            let mut h = hour;
            if is_pm && h < 12 {
                h += 12;
            }
            if is_am && h == 12 {
                h = 0;
            }
            return Some((
                KnownToken::Time {
                    hour: h,
                    min,
                    sec: None,
                    formatted: format!("{:02}:{:02}:00", h, min),
                },
                SCORE_NUMERIC,
            ));
        }
    }

    let parts: Vec<&str> = cleaned.split([':', '.', '-', ';', '|', 'h']).collect();
    if parts.is_empty() || parts[0].is_empty() {
        return None;
    }

    let mut hour = parts[0].parse::<u8>().ok()?;
    let min = if parts.len() > 1 && !parts[1].is_empty() {
        parts[1].parse::<u8>().ok()?
    } else {
        0
    };
    let sec = if parts.len() > 2 && !parts[2].is_empty() {
        Some(parts[2].parse::<u8>().ok()?)
    } else {
        None
    };

    if hour > 24 || min > 59 || sec.unwrap_or(0) > 59 {
        return None;
    }

    let mut score = SCORE_TIME_DEFAULT;
    if !is_am
        && !is_pm
        && !is_oclock
        && parts.len() == 2
        && !s.contains(':')
        && !s.contains('.')
        && parts[1].len() != 2
    {
        return None;
    } else if parts.len() == 1 && !is_pm && !is_am && !is_oclock {
        score = SCORE_TIME_SINGLE;
    } else if parts.len() == 2 && s.contains(':') {
        score = SCORE_TIME_COLON;
    } else if is_pm || is_am || is_oclock {
        score = SCORE_EXPLICIT;
    }

    if is_pm && hour < 12 {
        hour += 12;
    }
    if is_am && hour == 12 {
        hour = 0;
    }

    let formatted = if is_pm || is_am {
        format!("{:02}:{:02}:00", hour, min)
    } else if parts.len() == 1 {
        if hour <= 12 {
            hour += 12;
            if hour == 24 {
                hour = 0;
            }
            format!("{:02}:00:00", hour)
        } else {
            format!("{:02}:00:00", hour)
        }
    } else if let Some(s) = sec {
        format!("{:02}:{:02}:{:02}", hour, min, s)
    } else {
        format!("{:02}:{:02}:00", hour, min)
    };

    Some((
        KnownToken::Time {
            hour,
            min,
            sec,
            formatted,
        },
        score,
    ))
}

pub fn parse_iso(s: &str, config: &ParseConfig) -> Option<Vec<KnownToken>> {
    let lower = s.to_lowercase();
    if let Some((date_part, rest)) = lower.split_once('t') {
        let mut tokens = Vec::new();
        if let Some((d, _)) = parse_date_numeric_scored(date_part, config) {
            tokens.push(d);
        } else {
            return None;
        }

        let time_part;
        let tz_part;
        if let Some(stripped) = rest.strip_suffix('z') {
            time_part = stripped;
            tz_part = Some("z");
        } else if let Some(idx) = rest.find('+') {
            time_part = &rest[..idx];
            tz_part = Some(&rest[idx..]);
        } else if let Some(idx) = rest.find('-') {
            time_part = &rest[..idx];
            tz_part = Some(&rest[idx..]);
        } else {
            time_part = rest;
            tz_part = None;
        }

        if let Some((mut t, _)) = parse_time_scored(time_part) {
            if let KnownToken::Time { formatted, .. } = &mut t {
                *formatted = time_part.to_string(); // Keep exactly as ISO
            }
            tokens.push(t);
        } else {
            return None;
        }

        if let Some(tz) = tz_part
            && let Some((tz_token, _)) = parse_timezone_scored(tz)
        {
            tokens.push(tz_token);
        }
        return Some(tokens);
    }
    None
}

pub fn parse_timezone_scored(s: &str) -> Option<(KnownToken, f32)> {
    let lower = s.to_lowercase();
    if lower == "z" {
        return Some((KnownToken::TimeZone(TimeZoneKind::Z), 0.95));
    }
    if lower == "utc" || lower == "gmt" {
        return Some((KnownToken::TimeZone(TimeZoneKind::Utc), 0.95));
    }

    if lower.starts_with("utc") || lower.starts_with("gmt") {
        let rest = &lower[3..];
        if let Some((tz, s)) = parse_offset(rest) {
            return Some((tz, s));
        }
    }
    if (lower.starts_with('+') || lower.starts_with('-'))
        && let Some((tz, s)) = parse_offset(&lower)
    {
        return Some((tz, s));
    }

    if lower.contains('/') && !lower.chars().all(|c| c.is_ascii_digit() || c == '/') {
        return Some((
            KnownToken::TimeZone(TimeZoneKind::Named(s.to_string())),
            0.95,
        ));
    }

    let tz_abbrs = [
        "est", "edt", "cst", "cdt", "mst", "mdt", "pst", "pdt", "cet", "cest", "eet", "eest",
        "bst", "jst", "ist", "aest", "aedt",
    ];
    if tz_abbrs.contains(&lower.as_str()) {
        return Some((
            KnownToken::TimeZone(TimeZoneKind::Named(s.to_uppercase())),
            0.95,
        ));
    }

    None
}

fn parse_offset(s: &str) -> Option<(KnownToken, f32)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let sign = if s.starts_with('+') {
        1
    } else if s.starts_with('-') {
        -1
    } else {
        return None;
    };
    let rest = &s[1..];

    let (h, m) = if rest.contains(':') {
        let parts: Vec<&str> = rest.split(':').collect();
        let h = parts[0].parse::<i16>().ok()?;
        let m = parts[1].parse::<i16>().ok()?;
        (h, m)
    } else if rest.len() == 4 {
        let h = rest[0..2].parse::<i16>().ok()?;
        let m = rest[2..4].parse::<i16>().ok()?;
        (h, m)
    } else {
        let h = rest.parse::<i16>().ok()?;
        (h, 0)
    };

    let minutes_total = (h * 60 + m) * sign;
    Some((
        KnownToken::TimeZone(TimeZoneKind::Offset { minutes_total }),
        0.95,
    ))
}

fn evaluate_token(s: &str, config: &ParseConfig) -> Vec<ScoredToken> {
    let mut candidates = Vec::new();

    if let Some((t, score)) = parse_time_scored(s) {
        candidates.push(ScoredToken { token: t, score });
    }
    if let Some((d, score)) = parse_date_numeric_scored(s, config) {
        candidates.push(ScoredToken { token: d, score });
    }
    if let Some((tz, score)) = parse_timezone_scored(s) {
        candidates.push(ScoredToken { token: tz, score });
    }
    if let Ok(n) = s.parse::<i32>() {
        candidates.push(ScoredToken {
            token: KnownToken::Number(n),
            score: SCORE_NUMERIC,
        });
    }

    let lower = s.to_lowercase();

    // O(1) exact-match via HashMap — only falls through to fuzzy on a miss
    if let Some(known) = get_dict_map().get(lower.as_str()) {
        let score = if matches!(known, KnownToken::Time { .. }) {
            SCORE_DICT_TIME
        } else {
            SCORE_EXACT
        };
        if !candidates.iter().any(|c| &c.token == known) {
            candidates.push(ScoredToken {
                token: known.clone(),
                score,
            });
        }
    } else if lower.len() > 2 && !lower.chars().all(|c| c.is_ascii_digit()) {
        // Fuzzy scan — only runs when there is no exact dictionary match
        let mut best_fuzzy: Option<KnownToken> = None;
        let mut best_fuzzy_score = 0.0_f32;

        for &(word, ref known) in get_dict().iter() {
            if word.len() <= 2 {
                continue;
            }
            let len_diff = (lower.len() as i32 - word.len() as i32).abs();
            if len_diff > FUZZY_MAX_LEN_DIFF {
                continue;
            }

            let dist = levenshtein(&lower, word) as f32;
            let max_len = lower.len().max(word.len()) as f32;
            let mut score = 1.0 - (dist / max_len);
            if word.starts_with(lower.as_str()) {
                score += FUZZY_BONUS_PREFIX;
            }
            if word.chars().next() == lower.chars().next() {
                score += FUZZY_BONUS_FIRST_CHAR;
            }
            score = score.clamp(0.0, FUZZY_MAX_SCORE);

            if score > best_fuzzy_score {
                best_fuzzy_score = score;
                best_fuzzy = Some(known.clone());
            }
        }

        if best_fuzzy_score >= FUZZY_MIN_ACCEPT
            && let Some(tok) = best_fuzzy
            && !candidates.iter().any(|c| c.token == tok)
        {
            candidates.push(ScoredToken {
                token: tok,
                score: best_fuzzy_score,
            });
        }
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates
}

pub fn apply_context_boosts(tokens: &mut [Token], debug: bool) {
    let len = tokens.len();
    if len < 2 {
        return;
    }

    // Phase 1: accumulate all boosts in one read-only pass before mutating
    let mut known_boosts = vec![0.0_f32; len];
    let mut unknown_boosts: Vec<Vec<(usize, f32)>> = vec![vec![]; len];

    for i in 0..len {
        match &tokens[i] {
            Token::Known(st) => match st.token {
                KnownToken::Time { .. } => {
                    if i > 0
                        && let Token::Known(prev) = &tokens[i - 1]
                        && matches!(
                            prev.token,
                            KnownToken::DateNumeric { .. } | KnownToken::TimeZone(_)
                        )
                    {
                        known_boosts[i] += BOOST_TEMPORAL_ADJ;
                    }
                    if i + 1 < len
                        && let Token::Known(next) = &tokens[i + 1]
                        && matches!(
                            next.token,
                            KnownToken::DateNumeric { .. } | KnownToken::TimeZone(_)
                        )
                    {
                        known_boosts[i] += BOOST_TEMPORAL_ADJ;
                    }
                }
                KnownToken::TimeZone(_) | KnownToken::DateNumeric { .. } => {
                    if i > 0
                        && let Token::Known(prev) = &tokens[i - 1]
                        && matches!(prev.token, KnownToken::Time { .. })
                    {
                        known_boosts[i] += BOOST_TEMPORAL_ADJ;
                    }
                    if i + 1 < len
                        && let Token::Known(next) = &tokens[i + 1]
                        && matches!(next.token, KnownToken::Time { .. })
                    {
                        known_boosts[i] += BOOST_TEMPORAL_ADJ;
                    }
                }
                KnownToken::Number(_) => {
                    if i + 1 < len
                        && let Token::Known(next) = &tokens[i + 1]
                        && matches!(next.token, KnownToken::Unit(_))
                    {
                        known_boosts[i] += BOOST_NUM_UNIT_PAIR;
                        known_boosts[i + 1] += BOOST_NUM_UNIT_PAIR;
                    }
                }
                _ => {}
            },
            Token::Unknown { candidates, .. } => {
                let prev_is_modifier = i > 0
                    && matches!(tokens[i - 1], Token::Known(ref st) if matches!(st.token, KnownToken::Modifier(_)));
                if prev_is_modifier {
                    for (ci, c) in candidates.iter().enumerate() {
                        if matches!(c.token, KnownToken::Weekday(_)) {
                            unknown_boosts[i].push((ci, BOOST_MODIFIER_CTX));
                        }
                    }
                }
                let has_time_neighbor = (i > 0
                    && matches!(tokens[i - 1], Token::Known(ref st) if matches!(st.token, KnownToken::Time { .. })))
                    || (i + 1 < len
                        && matches!(tokens[i + 1], Token::Known(ref st) if matches!(st.token, KnownToken::Time { .. })));
                if has_time_neighbor {
                    for (ci, c) in candidates.iter().enumerate() {
                        if matches!(c.token, KnownToken::RelativeDay(_)) {
                            unknown_boosts[i].push((ci, BOOST_RELATIVE_TIME));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Phase 2: apply all accumulated boosts; sort each Unknown's candidates once
    for i in 0..len {
        if known_boosts[i] > 0.0
            && let Token::Known(ref mut st) = tokens[i]
        {
            st.score = (st.score + known_boosts[i]).min(1.0);
            if debug {
                println!(
                    "DEBUG: Context boosted {:?} by {}",
                    st.token, known_boosts[i]
                );
            }
        }
        if !unknown_boosts[i].is_empty()
            && let Token::Unknown {
                ref mut candidates, ..
            } = tokens[i]
        {
            for &(ci, boost) in &unknown_boosts[i] {
                if let Some(c) = candidates.get_mut(ci) {
                    c.score = (c.score + boost).min(1.0);
                    if debug {
                        println!("DEBUG: Boosted candidate {:?} by {}", c.token, boost);
                    }
                }
            }
            candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        }
    }

    // Phase 3: At + Number → Time (structural conversion, after scores are settled).
    // Reads are separated from the mutable write to satisfy the borrow checker.
    // If a named morning token (hour == 9) is present within ±5 tokens, the number
    // is treated as AM rather than defaulting to PM ("morning at 2" → 02:00, not 14:00).
    for i in 0..len - 1 {
        let has_at =
            matches!(tokens[i], Token::Known(ref st) if matches!(st.token, KnownToken::At));
        if !has_at {
            continue;
        }

        // Read: extract the number value before any mutable borrow.
        let maybe_n = if let Token::Known(ref st) = tokens[i + 1] {
            if let KnownToken::Number(n) = st.token {
                Some(n)
            } else {
                None
            }
        } else {
            None
        };
        let n = match maybe_n {
            Some(n) if n > 0 && n <= 24 => n,
            _ => continue,
        };

        // Read: scan a ±5-token window for a named morning Time (hour == 9).
        let window_lo = i.saturating_sub(4);
        let window_hi = (i + 6).min(len);
        let morning_context = (window_lo..window_hi)
            .filter(|&j| j != i && j != i + 1)
            .any(|j| {
                matches!(tokens[j],
                Token::Known(ref st) if matches!(st.token, KnownToken::Time { hour: 9, .. }))
            });

        let mut h = n as u8;
        if !morning_context && h <= 12 {
            h += 12;
            if h == 24 {
                h = 0;
            }
        }

        // Write: take the mutable borrow only after all reads are done.
        if let Token::Known(ref mut st_next) = tokens[i + 1] {
            st_next.token = KnownToken::Time {
                hour: h,
                min: 0,
                sec: None,
                formatted: format!("{:02}:00:00", h),
            };
            st_next.score = SCORE_EXPLICIT;
            if debug {
                println!(
                    "DEBUG: Converted At + Number to Time ({}:00{})",
                    h,
                    if morning_context {
                        " [morning ctx]"
                    } else {
                        ""
                    }
                );
            }
        }
    }

    // Phase 4: promote Unknown ≥ THRESHOLD_PROMOTE to Known
    for token in tokens.iter_mut().take(len) {
        let best_clone = if let Token::Unknown { candidates, .. } = &*token
            && let Some(best) = candidates.first()
            && best.score >= THRESHOLD_PROMOTE
        {
            Some(best.clone())
        } else {
            None
        };
        if let Some(bc) = best_clone {
            *token = Token::Known(bc);
        }
    }
}

pub fn tokenize_and_classify(input: &str, config: &ParseConfig) -> Vec<Token> {
    let norm = normalize(input);
    let raw = tokenize(&norm);

    let mut i = 0;
    let mut tokens = Vec::new();

    while i < raw.len() {
        let s1 = raw[i].as_str();

        if s1 == "the" || s1 == "a" || s1 == "coming" || s1 == "after" || s1 == "of" {
            tokens.push(Token::Noise(s1.to_string()));
            i += 1;
            continue;
        }

        if s1.contains('t')
            && (s1.contains('-') || s1.contains('z') || s1.contains('+') || s1.len() > 10)
            && let Some(iso_tokens) = parse_iso(s1, config)
        {
            for t in iso_tokens {
                tokens.push(Token::Known(ScoredToken {
                    token: t,
                    score: 1.0,
                }));
            }
            i += 1;
            continue;
        }

        let cands1 = evaluate_token(s1, config);
        let score1 = cands1.first().map(|c| c.score).unwrap_or(0.0);

        if i + 1 < raw.len() {
            let s2 = raw[i + 1].as_str();

            let cands2 = evaluate_token(s2, config);
            let score2 = cands2.first().map(|c| c.score).unwrap_or(0.0);

            let score_original = (score1 + score2) / 2.0;

            let combined_direct = format!("{}{}", s1, s2);
            let cands_comb_dir = evaluate_token(&combined_direct, config);
            let score_dir = cands_comb_dir.first().map(|c| c.score).unwrap_or(0.0);

            let combined_space = format!("{} {}", s1, s2);
            let cands_comb_spc = evaluate_token(&combined_space, config);
            let score_spc = cands_comb_spc.first().map(|c| c.score).unwrap_or(0.0);

            let best_comb_score = score_dir.max(score_spc);
            let best_cands = if score_dir > score_spc {
                cands_comb_dir
            } else {
                cands_comb_spc
            };

            if best_comb_score > score_original + RECOMBINE_GAIN && best_comb_score >= RECOMBINE_MIN
            {
                if config.debug {
                    println!(
                        "DEBUG: Recombined '{}' and '{}' -> score {}",
                        s1, s2, best_comb_score
                    );
                }
                tokens.push(Token::Known(best_cands[0].clone()));
                i += 2;
                continue;
            }

            if let (Ok(h), Ok(m)) = (s1.parse::<u8>(), s2.parse::<u8>())
                && h < 24
                && m < 60
                && s2.len() == 2
            {
                let combined_time = format!("{:02}:{:02}", h, m);
                if let Some((t, score)) = parse_time_scored(&combined_time) {
                    if config.debug {
                        println!(
                            "DEBUG: Recombined adjacent numbers '{}' and '{}' into time",
                            s1, s2
                        );
                    }
                    tokens.push(Token::Known(ScoredToken { token: t, score }));
                    i += 2;
                    continue;
                }
            }
        }

        if cands1.is_empty() {
            tokens.push(Token::Unknown {
                word: s1.to_string(),
                candidates: vec![],
            });
        } else if cands1.len() == 1 {
            if cands1[0].score >= THRESHOLD_KNOWN {
                tokens.push(Token::Known(cands1[0].clone()));
            } else {
                tokens.push(Token::Unknown {
                    word: s1.to_string(),
                    candidates: cands1,
                });
            }
        } else {
            let best = &cands1[0];
            let second = &cands1[1];
            if best.score >= THRESHOLD_KNOWN && (best.score - second.score >= THRESHOLD_MARGIN) {
                tokens.push(Token::Known(best.clone()));
            } else {
                tokens.push(Token::Unknown {
                    word: s1.to_string(),
                    candidates: cands1,
                });
            }
        }
        i += 1;
    }

    apply_context_boosts(&mut tokens, config.debug);

    if config.resolve_dates {
        resolve(tokens, config)
    } else {
        tokens
    }
}

fn nth_weekday_of_month(year: i32, month: u32, weekday: chrono::Weekday, n: u8) -> NaiveDate {
    let mut d = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let mut count = 0;
    while d.month() == month {
        if d.weekday() == weekday {
            count += 1;
            if count == n {
                return d;
            }
        }
        d += Duration::days(1);
    }
    d - Duration::days(1)
}

fn last_weekday_of_month(year: i32, month: u32, weekday: chrono::Weekday) -> NaiveDate {
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_month_year = if month == 12 { year + 1 } else { year };
    let mut d =
        NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap() - Duration::days(1);
    while d.month() == month {
        if d.weekday() == weekday {
            return d;
        }
        d -= Duration::days(1);
    }
    d
}

fn resolve_holiday(h: &Holiday, year: i32) -> NaiveDate {
    match h {
        Holiday::Christmas => NaiveDate::from_ymd_opt(year, 12, 25).unwrap(),
        Holiday::NewYearsDay => NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
        Holiday::IndependenceDay => NaiveDate::from_ymd_opt(year, 7, 4).unwrap(),
        Holiday::Halloween => NaiveDate::from_ymd_opt(year, 10, 31).unwrap(),
        Holiday::VeteransDay => NaiveDate::from_ymd_opt(year, 11, 11).unwrap(),
        Holiday::Juneteenth => NaiveDate::from_ymd_opt(year, 6, 19).unwrap(),
        Holiday::ValentinesDay => NaiveDate::from_ymd_opt(year, 2, 14).unwrap(),
        Holiday::BoxingDay => NaiveDate::from_ymd_opt(year, 12, 26).unwrap(),
        Holiday::GuyFawkes => NaiveDate::from_ymd_opt(year, 11, 5).unwrap(),
        Holiday::StPatricksDay => NaiveDate::from_ymd_opt(year, 3, 17).unwrap(),
        Holiday::Thanksgiving => nth_weekday_of_month(year, 11, chrono::Weekday::Thu, 4),
        Holiday::MemorialDay => last_weekday_of_month(year, 5, chrono::Weekday::Mon),
        Holiday::LaborDay => nth_weekday_of_month(year, 9, chrono::Weekday::Mon, 1),
        Holiday::MlkDay => nth_weekday_of_month(year, 1, chrono::Weekday::Mon, 3),
        Holiday::PresidentsDay => nth_weekday_of_month(year, 2, chrono::Weekday::Mon, 3),
        Holiday::MayDay => nth_weekday_of_month(year, 5, chrono::Weekday::Mon, 1),
        Holiday::SpringBankHoliday => last_weekday_of_month(year, 5, chrono::Weekday::Mon),
        Holiday::SummerBankHoliday => last_weekday_of_month(year, 8, chrono::Weekday::Mon),
    }
}

/// Converts "N unit(s) ago" token triples into a resolved DateNumeric in place.
fn preprocess_ago_patterns(mut tokens: Vec<Token>, now: NaiveDate) -> Vec<Token> {
    let mut i = 0;
    while i + 2 < tokens.len() {
        let maybe_n = if let Token::Known(ref st) = tokens[i] {
            if let KnownToken::Number(n) = st.token {
                Some((n, st.score))
            } else {
                None
            }
        } else {
            None
        };

        let maybe_unit = if let Token::Known(ref st) = tokens[i + 1] {
            if let KnownToken::Unit(ref u) = st.token {
                Some((u.clone(), st.score))
            } else {
                None
            }
        } else {
            None
        };

        let is_ago = matches!(tokens[i + 2],
            Token::Known(ref st) if matches!(st.token, KnownToken::Modifier(Modifier::Ago)));

        if let (Some((n, ns)), Some((unit, us)), true) = (maybe_n, maybe_unit, is_ago) {
            let days = match unit {
                Unit::Day => n as i64,
                Unit::Week => n as i64 * 7,
                Unit::Month => n as i64 * 30,
                Unit::Year => n as i64 * 365,
            };
            let d = now - Duration::days(days);
            let new_token = Token::Known(ScoredToken {
                token: KnownToken::DateNumeric {
                    y: Some(d.year()),
                    m: d.month() as u8,
                    d: d.day() as u8,
                },
                score: (ns + us) / 2.0,
            });
            tokens.splice(i..i + 3, std::iter::once(new_token));
            // don't advance i — re-check from same position in case of chained patterns
        } else {
            i += 1;
        }
    }
    tokens
}

pub fn resolve(tokens: Vec<Token>, config: &ParseConfig) -> Vec<Token> {
    let now = config.mock_now.unwrap_or_else(|| Local::now().date_naive());
    let tokens = preprocess_ago_patterns(tokens, now);
    let mut resolved = Vec::new();
    let mut current_modifier = None;
    let mut current_modifier_score = 0.0;

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Known(st) => {
                match st.token {
                    KnownToken::Modifier(ref m) => {
                        current_modifier = Some(m.clone());
                        current_modifier_score = st.score;
                    }
                    KnownToken::RelativeDay(ref r) => {
                        let d = match r {
                            RelativeDay::Today => now,
                            RelativeDay::Tomorrow => now + Duration::days(1),
                            RelativeDay::Yesterday => now - Duration::days(1),
                        };
                        resolved.push(Token::Known(ScoredToken {
                            token: KnownToken::DateNumeric {
                                y: Some(d.year()),
                                m: d.month() as u8,
                                d: d.day() as u8,
                            },
                            score: st.score,
                        }));
                        current_modifier = None;
                    }
                    KnownToken::Weekday(ref w) => {
                        let target_wd = match w {
                            Weekday::Monday => chrono::Weekday::Mon,
                            Weekday::Tuesday => chrono::Weekday::Tue,
                            Weekday::Wednesday => chrono::Weekday::Wed,
                            Weekday::Thursday => chrono::Weekday::Thu,
                            Weekday::Friday => chrono::Weekday::Fri,
                            Weekday::Saturday => chrono::Weekday::Sat,
                            Weekday::Sunday => chrono::Weekday::Sun,
                        };

                        let mut days_ahead = target_wd.num_days_from_monday() as i64
                            - now.weekday().num_days_from_monday() as i64;
                        if days_ahead <= 0 {
                            days_ahead += 7;
                        }

                        // Support postfix modifiers: "monday last", "friday next"
                        let postfix_mod = if current_modifier.is_none() && i + 1 < tokens.len() {
                            if let Token::Known(ref st_next) = tokens[i + 1] {
                                if let KnownToken::Modifier(ref m) = st_next.token {
                                    Some(m.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let effective_mod = current_modifier.as_ref().or(postfix_mod.as_ref());

                        if let Some(Modifier::Next) = effective_mod {
                            days_ahead += 7;
                        } else if let Some(Modifier::Last) = effective_mod {
                            days_ahead -= 14;
                            if days_ahead < -7 {
                                days_ahead += 7;
                            }
                        }

                        // Consume the postfix modifier token
                        if postfix_mod.is_some() {
                            i += 1;
                        }

                        if i + 1 < tokens.len()
                            && let Token::Known(st_next) = &tokens[i + 1]
                            && let KnownToken::Unit(Unit::Week) = st_next.token
                        {
                            days_ahead += 7;
                            i += 1;
                        }

                        let d = now + Duration::days(days_ahead);
                        resolved.push(Token::Known(ScoredToken {
                            token: KnownToken::DateNumeric {
                                y: Some(d.year()),
                                m: d.month() as u8,
                                d: d.day() as u8,
                            },
                            score: st.score,
                        }));
                        current_modifier = None;
                    }
                    KnownToken::Holiday(ref h) => {
                        let current_year = now.year();
                        let mut h_date = resolve_holiday(h, current_year);
                        if h_date < now {
                            h_date = resolve_holiday(h, current_year + 1);
                        }
                        resolved.push(Token::Known(ScoredToken {
                            token: KnownToken::DateNumeric {
                                y: Some(h_date.year()),
                                m: h_date.month() as u8,
                                d: h_date.day() as u8,
                            },
                            score: st.score,
                        }));
                        current_modifier = None;
                    }
                    KnownToken::DateNumeric { y, m, d } => {
                        let yr = y;
                        let mo = m;
                        let da = d;
                        if let Some(modi) = current_modifier.take() {
                            resolved.push(Token::Known(ScoredToken {
                                token: KnownToken::Modifier(modi),
                                score: current_modifier_score,
                            }));
                        }
                        let mut final_year = yr.unwrap_or(now.year());
                        if yr.is_none() {
                            let target_date =
                                NaiveDate::from_ymd_opt(now.year(), mo as u32, da as u32)
                                    .unwrap_or(now);
                            if target_date < now {
                                final_year = now.year() + 1;
                            }
                        }

                        resolved.push(Token::Known(ScoredToken {
                            token: KnownToken::DateNumeric {
                                y: Some(final_year),
                                m: mo,
                                d: da,
                            },
                            score: st.score,
                        }));
                    }
                    _ => {
                        if let Some(modi) = current_modifier.take() {
                            resolved.push(Token::Known(ScoredToken {
                                token: KnownToken::Modifier(modi),
                                score: current_modifier_score,
                            }));
                        }
                        resolved.push(Token::Known(st.clone()));
                    }
                }
            }
            other => {
                if let Some(modi) = current_modifier.take() {
                    resolved.push(Token::Known(ScoredToken {
                        token: KnownToken::Modifier(modi),
                        score: current_modifier_score,
                    }));
                }
                resolved.push(other.clone());
            }
        }
        i += 1;
    }

    if let Some(modi) = current_modifier {
        resolved.push(Token::Known(ScoredToken {
            token: KnownToken::Modifier(modi),
            score: current_modifier_score,
        }));
    }

    resolved
}

pub fn to_canonical(tokens: &[Token]) -> String {
    let tokens: Vec<Token> = tokens
        .iter()
        .filter(|t| !matches!(t, Token::Noise(_) | Token::Unknown { .. }))
        .cloned()
        .collect();

    let mut modifier = None;
    let mut date_str = None;
    let mut time_str = None;
    let mut tz_str = None;

    let mut modifier_score = 0.0;
    let mut date_score = 0.0;
    let mut time_score = 0.0;
    let mut tz_score = 0.0;

    for t in tokens {
        if let Token::Known(st) = t {
            let score = st.score;
            match st.token {
                KnownToken::Modifier(ref m) => {
                    if score >= modifier_score {
                        modifier = Some(match m {
                            Modifier::Next => "next",
                            Modifier::Last => "last",
                            Modifier::This => "this",
                            Modifier::Ago => "ago",
                        });
                        modifier_score = score;
                    }
                }
                KnownToken::RelativeDay(ref r) => {
                    if score >= date_score {
                        date_str = Some(match r {
                            RelativeDay::Today => "today".to_string(),
                            RelativeDay::Tomorrow => "tomorrow".to_string(),
                            RelativeDay::Yesterday => "yesterday".to_string(),
                        });
                        date_score = score;
                    }
                }
                KnownToken::Weekday(ref w) => {
                    if score >= date_score {
                        date_str = Some(match w {
                            Weekday::Monday => "monday".to_string(),
                            Weekday::Tuesday => "tuesday".to_string(),
                            Weekday::Wednesday => "wednesday".to_string(),
                            Weekday::Thursday => "thursday".to_string(),
                            Weekday::Friday => "friday".to_string(),
                            Weekday::Saturday => "saturday".to_string(),
                            Weekday::Sunday => "sunday".to_string(),
                        });
                        date_score = score;
                    }
                }
                KnownToken::DateNumeric { y, m, d } => {
                    if score >= date_score {
                        if let Some(year) = y {
                            date_str = Some(format!("{:04}-{:02}-{:02}", year, m, d));
                        } else {
                            date_str = Some(format!("{:02}-{:02}", m, d));
                        }
                        date_score = score;
                    }
                }
                KnownToken::Time { ref formatted, .. } => {
                    if score >= time_score {
                        time_str = Some(formatted.clone());
                        time_score = score;
                    }
                }
                KnownToken::TimeZone(ref tz) => {
                    if score >= tz_score {
                        tz_str = Some(match tz {
                            TimeZoneKind::Z => "UTC".to_string(),
                            TimeZoneKind::Utc => "UTC".to_string(),
                            TimeZoneKind::Offset { minutes_total } => {
                                let sign = if *minutes_total < 0 { "-" } else { "+" };
                                let abs_min = minutes_total.abs();
                                let h = abs_min / 60;
                                let m = abs_min % 60;
                                if m == 0 {
                                    format!("UTC{}{:02}", sign, h)
                                } else {
                                    format!("UTC{}{:02}:{:02}", sign, h, m)
                                }
                            }
                            TimeZoneKind::Named(n) => n.clone(),
                        });
                        tz_score = score;
                    }
                }
                _ => {}
            }
        }
    }

    let mut parts = Vec::new();
    if let Some(m) = modifier {
        if let Some(d) = &date_str {
            parts.push(format!("{} {}", m, d));
        } else {
            parts.push(m.to_string());
        }
    } else if let Some(d) = date_str {
        parts.push(d);
    }

    if let Some(t) = time_str {
        if !parts.is_empty() {
            parts.push("at".to_string());
        }
        parts.push(t);
    }

    if let Some(ref tz) = tz_str {
        parts.push(tz.clone());
    }

    parts.join(" ")
}

pub fn format_custom(tokens: &[Token], template: &str) -> String {
    let mut y = None;
    let mut m = None;
    let mut d = None;
    let mut h = None;
    let mut min = None;
    let mut s = None;
    let mut tz = None;
    let mut rel = None;

    let mut date_score = 0.0;
    let mut time_score = 0.0;
    let mut tz_score = 0.0;
    let mut rel_score = 0.0;

    for t in tokens {
        if let Token::Known(st) = t {
            let score = st.score;
            match st.token {
                KnownToken::DateNumeric {
                    y: year,
                    m: month,
                    d: day,
                } => {
                    if score >= date_score {
                        y = year;
                        m = Some(month);
                        d = Some(day);
                        date_score = score;
                    }
                }
                KnownToken::Time {
                    hour,
                    min: minute,
                    sec,
                    ..
                } => {
                    if score >= time_score {
                        h = Some(hour);
                        min = Some(minute);
                        s = sec;
                        time_score = score;
                    }
                }
                KnownToken::TimeZone(ref tz_kind) => {
                    if score >= tz_score {
                        tz = Some(match tz_kind {
                            TimeZoneKind::Z => "UTC".to_string(),
                            TimeZoneKind::Utc => "UTC".to_string(),
                            TimeZoneKind::Offset { minutes_total } => {
                                let sign = if *minutes_total < 0 { "-" } else { "+" };
                                let abs_min = minutes_total.abs();
                                let hr = abs_min / 60;
                                let mr = abs_min % 60;
                                if mr == 0 {
                                    format!("UTC{}{:02}", sign, hr)
                                } else {
                                    format!("UTC{}{:02}:{:02}", sign, hr, mr)
                                }
                            }
                            TimeZoneKind::Named(n) => n.clone(),
                        });
                        tz_score = score;
                    }
                }
                KnownToken::RelativeDay(ref r) => {
                    if score >= rel_score {
                        rel = Some(match r {
                            RelativeDay::Today => "today",
                            RelativeDay::Tomorrow => "tomorrow",
                            RelativeDay::Yesterday => "yesterday",
                        });
                        rel_score = score;
                    }
                }
                KnownToken::Weekday(ref w) => {
                    if score >= rel_score {
                        rel = Some(match w {
                            Weekday::Monday => "monday",
                            Weekday::Tuesday => "tuesday",
                            Weekday::Wednesday => "wednesday",
                            Weekday::Thursday => "thursday",
                            Weekday::Friday => "friday",
                            Weekday::Saturday => "saturday",
                            Weekday::Sunday => "sunday",
                        });
                        rel_score = score;
                    }
                }
                _ => {}
            }
        }
    }

    let mut out = template.to_string();
    if let Some(year) = y {
        out = out.replace("YYYY", &format!("{:04}", year));
        out = out.replace("YY", &format!("{:02}", year % 100));
    }
    if let Some(month) = m {
        out = out.replace("MM", &format!("{:02}", month));
    }
    if let Some(day) = d {
        out = out.replace("DD", &format!("{:02}", day));
    }
    if let Some(hour) = h {
        out = out.replace("HH", &format!("{:02}", hour));
    } else {
        out = out.replace("HH", "00");
    }
    if let Some(minute) = min {
        out = out.replace("mm", &format!("{:02}", minute));
    } else {
        out = out.replace("mm", "00");
    }
    if let Some(sec) = s {
        out = out.replace("ss", &format!("{:02}", sec));
    } else {
        out = out.replace("ss", "00");
    }
    if let Some(tz_str) = tz {
        out = out.replace("TZ", &tz_str);
        out = out.replace("Z", &tz_str);
    } else {
        out = out.replace(" TZ", "");
        out = out.replace(" Z", "");
        out = out.replace("TZ", "");
        out = out.replace("Z", "");
    }

    if let Some(r) = rel {
        out = out.replace("{RELATIVE}", r);
    } else {
        out = out.replace("{RELATIVE}", "");
    }

    out
}

pub fn process(input: &str, config: &ParseConfig) -> String {
    let tokens = tokenize_and_classify(input, config);
    to_canonical(&tokens)
}

#[cfg(test)]
mod tests {
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
            debug: false,
        };

        for case in suite.cases {
            let tokens = tokenize_and_classify(&case.input, &config);
            let output = format_custom(&tokens, &case.format).trim().to_string();
            assert_eq!(output, case.expected, "Failed on input: {}", case.input);
        }
    }
}
