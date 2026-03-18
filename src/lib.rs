use strsim::levenshtein;
use chrono::{Datelike, Local, NaiveDate, Duration};

pub struct ParseConfig {
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
pub enum Weekday { Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Month { Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RelativeDay { Today, Tomorrow, Yesterday }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Modifier { Next, Last, This }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Unit { Day, Week, Month, Year }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum TimeZoneKind {
    Z,
    Utc,
    Offset { hours: i8, minutes: i8 },
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
    Time { hour: u8, min: u8, sec: Option<u8>, formatted: String },
    DateNumeric { y: Option<i32>, m: u8, d: u8 },
    TimeZone(TimeZoneKind),
    Holiday(Holiday),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum UnknownToken {
    Word(String),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Token {
    Known(KnownToken),
    Unknown(UnknownToken),
    Noise(String),
}

pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    let lower = input.to_lowercase();
    let replaced = lower.replace('@', " at ").replace('\'', "").replace("o clock", "oclock");
    
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
    
    let mut chars = replaced_phrases.chars().peekable();
    let mut in_space = false;
    
    while let Some(c) = chars.next() {
        if c == ',' || c == '!' || c == '?' || c == '|' || c == ';' {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else if c.is_whitespace() {
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
        "one" | "1" => Some(1), "two" | "2" => Some(2), "three" | "3" => Some(3),
        "four" | "4" => Some(4), "five" | "5" => Some(5), "six" | "6" => Some(6),
        "seven" | "7" => Some(7), "eight" | "8" => Some(8), "nine" | "9" => Some(9),
        "ten" | "10" => Some(10), "eleven" | "11" => Some(11), "twelve" | "12" => Some(12),
        "thirteen" | "13" => Some(13), "fourteen" | "14" => Some(14), "fifteen" | "15" => Some(15),
        "sixteen" | "16" => Some(16), "seventeen" | "17" => Some(17), "eighteen" | "18" => Some(18),
        "nineteen" | "19" => Some(19), "twenty" | "20" => Some(20), "thirty" | "30" => Some(30),
        "forty" | "40" => Some(40), "fifty" | "50" => Some(50),
        _ => None,
    }
}

fn convert_compound_numbers(words: Vec<String>) -> Vec<String> {
    let mut i = 0;
    let mut res = Vec::new();
    while i < words.len() {
        let w = &words[i];
        if let Some(n1) = text_to_number(w) {
            if i + 1 < words.len() {
                if let Some(n2) = text_to_number(&words[i+1]) {
                    if (n1 == 20 || n1 == 30 || n1 == 40 || n1 == 50) && n2 < 10 {
                        res.push((n1 + n2).to_string());
                        i += 2;
                        continue;
                    }
                }
            }
            res.push(n1.to_string());
        } else if w.contains('-') {
            let parts: Vec<&str> = w.split('-').collect();
            if parts.len() == 2 {
                if let (Some(n1), Some(n2)) = (text_to_number(parts[0]), text_to_number(parts[1])) {
                    if (n1 == 20 || n1 == 30 || n1 == 40 || n1 == 50) && n2 < 10 {
                        res.push((n1 + n2).to_string());
                        i += 1;
                        continue;
                    }
                }
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
        if words[i] == "half" && i + 2 < words.len() && words[i+1] == "past" {
            if let Ok(h) = words[i+2].parse::<u8>() {
                res.push(format!("{:02}:30", h));
                i += 3;
                continue;
            }
        } else if words[i] == "quarter" && i + 2 < words.len() {
            if words[i+1] == "past" {
                if let Ok(h) = words[i+2].parse::<u8>() {
                    res.push(format!("{:02}:15", h));
                    i += 3;
                    continue;
                }
            } else if words[i+1] == "to" {
                if let Ok(h) = words[i+2].parse::<u8>() {
                    let h_prev = if h == 1 || h == 0 { 12 } else { h - 1 };
                    res.push(format!("{:02}:45", h_prev));
                    i += 3;
                    continue;
                }
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

    words.into_iter().map(|s| {
        if s.chars().all(|c| c.is_ascii_digit()) {
            RawToken::Number(s)
        } else if s.contains(':') && s.contains('-') && (s.contains('t') || s.contains('z')) {
            RawToken::DateNumeric(s)
        } else if (s.contains('/') || s.contains('-') || s.contains('.')) && s.chars().any(|c| c.is_ascii_digit()) && !s.contains(':') {
            RawToken::DateNumeric(s)
        } else if s.contains(':') || s.contains('.') || s.contains(';') || s.contains('|') || s.contains('h') || s.ends_with("am") || s.ends_with("pm") {
            if s.chars().any(|c| c.is_ascii_digit()) {
                RawToken::Time(s)
            } else {
                RawToken::Word(s)
            }
        } else if s == "z" || s == "utc" || s == "gmt" || s.starts_with("utc+") || s.starts_with("utc-") || s.starts_with("gmt+") || s.starts_with("gmt-") || ((s.starts_with('+') || s.starts_with('-')) && s.len() >= 3 && s[1..].chars().all(|c| c.is_ascii_digit() || c == ':')) {
            RawToken::TimeZone(s)
        } else if s.contains('/') && !s.chars().any(|c| c.is_ascii_digit()) {
            RawToken::TimeZone(s)
        } else {
            RawToken::Word(s)
        }
    }).collect()
}

pub fn recombine(raw: Vec<RawToken>) -> Vec<RawToken> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if i + 1 < raw.len() {
            let s1 = raw[i].as_str();
            let s2 = raw[i+1].as_str();
            
            // Recombine time strings like "3 pm" -> "3pm"
            let combined_time = format!("{}{}", s1, s2);
            if let Some(_) = parse_time(&combined_time) {
                if s1.chars().all(|c| c.is_ascii_digit()) && (s2 == "am" || s2 == "pm") {
                    out.push(RawToken::Time(combined_time));
                    i += 2;
                    continue;
                }
            }
            
            // Handle adjacent numbers becoming time like "9 30" -> "9:30"
            if let (RawToken::Number(n1), RawToken::Number(n2)) = (&raw[i], &raw[i+1]) {
                if let (Ok(h), Ok(m)) = (n1.parse::<u8>(), n2.parse::<u8>()) {
                    if h < 24 && m < 60 {
                        let combined_time = format!("{:02}:{:02}", h, m);
                        out.push(RawToken::Time(combined_time));
                        i += 2;
                        continue;
                    }
                }
            }

            let combined_word = format!("{}{}", s1, s2);
            if lookup_dict(&combined_word).is_some() {
                out.push(RawToken::Word(combined_word));
                i += 2;
                continue;
            }

            let combined_word_space = format!("{} {}", s1, s2);
            if lookup_dict(&combined_word_space).is_some() {
                out.push(RawToken::Word(combined_word_space));
                i += 2;
                continue;
            }
        }
        out.push(raw[i].clone());
        i += 1;
    }
    out
}

pub fn lookup_dict(word: &str) -> Option<KnownToken> {
    match word {
        "monday" | "mon" => Some(KnownToken::Weekday(Weekday::Monday)),
        "tuesday" | "tue" | "tues" => Some(KnownToken::Weekday(Weekday::Tuesday)),
        "wednesday" | "wed" => Some(KnownToken::Weekday(Weekday::Wednesday)),
        "thursday" | "thu" | "thurs" => Some(KnownToken::Weekday(Weekday::Thursday)),
        "friday" | "fri" | "frday" | "fryday" => Some(KnownToken::Weekday(Weekday::Friday)),
        "saturday" | "sat" => Some(KnownToken::Weekday(Weekday::Saturday)),
        "sunday" | "sun" => Some(KnownToken::Weekday(Weekday::Sunday)),

        "january" | "jan" => Some(KnownToken::Month(Month::Jan)),
        "february" | "feb" => Some(KnownToken::Month(Month::Feb)),
        "march" | "mar" => Some(KnownToken::Month(Month::Mar)),
        "april" | "apr" => Some(KnownToken::Month(Month::Apr)),
        "may" => Some(KnownToken::Month(Month::May)),
        "june" | "jun" => Some(KnownToken::Month(Month::Jun)),
        "july" | "jul" => Some(KnownToken::Month(Month::Jul)),
        "august" | "aug" => Some(KnownToken::Month(Month::Aug)),
        "september" | "sep" | "sept" => Some(KnownToken::Month(Month::Sep)),
        "october" | "oct" => Some(KnownToken::Month(Month::Oct)),
        "november" | "nov" => Some(KnownToken::Month(Month::Nov)),
        "december" | "dec" => Some(KnownToken::Month(Month::Dec)),

        "today" => Some(KnownToken::RelativeDay(RelativeDay::Today)),
        "tomorrow" | "tmrw" | "tmr" | "tomorow" => Some(KnownToken::RelativeDay(RelativeDay::Tomorrow)),
        "yesterday" => Some(KnownToken::RelativeDay(RelativeDay::Yesterday)),

        "next" | "nxt" => Some(KnownToken::Modifier(Modifier::Next)),
        "last" | "lst" => Some(KnownToken::Modifier(Modifier::Last)),
        "this" => Some(KnownToken::Modifier(Modifier::This)),

        "day" | "days" => Some(KnownToken::Unit(Unit::Day)),
        "week" | "weeks" => Some(KnownToken::Unit(Unit::Week)),
        "month" | "months" => Some(KnownToken::Unit(Unit::Month)),
        "year" | "years" => Some(KnownToken::Unit(Unit::Year)),

        "at" | "on" | "in" => Some(KnownToken::At),
        
        "morning" | "mrning" => Some(KnownToken::Time { hour: 9, min: 0, sec: None, formatted: "09:00:00".to_string() }),
        "noon" => Some(KnownToken::Time { hour: 12, min: 0, sec: None, formatted: "12:00:00".to_string() }),
        "afternoon" => Some(KnownToken::Time { hour: 15, min: 0, sec: None, formatted: "15:00:00".to_string() }),
        "evening" | "evning" => Some(KnownToken::Time { hour: 18, min: 0, sec: None, formatted: "18:00:00".to_string() }),
        "night" | "nite" => Some(KnownToken::Time { hour: 21, min: 0, sec: None, formatted: "21:00:00".to_string() }),
        
        "newyearsday" => Some(KnownToken::Holiday(Holiday::NewYearsDay)),
        "christmas" => Some(KnownToken::Holiday(Holiday::Christmas)),
        "independenceday" => Some(KnownToken::Holiday(Holiday::IndependenceDay)),
        "halloween" => Some(KnownToken::Holiday(Holiday::Halloween)),
        "thanksgiving" => Some(KnownToken::Holiday(Holiday::Thanksgiving)),
        "memorialday" => Some(KnownToken::Holiday(Holiday::MemorialDay)),
        "laborday" => Some(KnownToken::Holiday(Holiday::LaborDay)),
        "mlkday" => Some(KnownToken::Holiday(Holiday::MlkDay)),
        "presidentsday" => Some(KnownToken::Holiday(Holiday::PresidentsDay)),
        "veteransday" => Some(KnownToken::Holiday(Holiday::VeteransDay)),
        "juneteenth" => Some(KnownToken::Holiday(Holiday::Juneteenth)),
        "valentinesday" => Some(KnownToken::Holiday(Holiday::ValentinesDay)),
        "boxingday" => Some(KnownToken::Holiday(Holiday::BoxingDay)),
        "guyfawkes" => Some(KnownToken::Holiday(Holiday::GuyFawkes)),
        "stpatricksday" => Some(KnownToken::Holiday(Holiday::StPatricksDay)),
        "mayday" => Some(KnownToken::Holiday(Holiday::MayDay)),
        "springbankholiday" => Some(KnownToken::Holiday(Holiday::SpringBankHoliday)),
        "summerbankholiday" => Some(KnownToken::Holiday(Holiday::SummerBankHoliday)),
        
        _ => None,
    }
}

pub fn fuzzy_match(token: &str) -> Option<(KnownToken, f32)> {
    let dict = vec![
        ("monday", KnownToken::Weekday(Weekday::Monday)),
        ("tuesday", KnownToken::Weekday(Weekday::Tuesday)),
        ("wednesday", KnownToken::Weekday(Weekday::Wednesday)),
        ("thursday", KnownToken::Weekday(Weekday::Thursday)),
        ("friday", KnownToken::Weekday(Weekday::Friday)),
        ("saturday", KnownToken::Weekday(Weekday::Saturday)),
        ("sunday", KnownToken::Weekday(Weekday::Sunday)),
        ("january", KnownToken::Month(Month::Jan)),
        ("february", KnownToken::Month(Month::Feb)),
        ("march", KnownToken::Month(Month::Mar)),
        ("april", KnownToken::Month(Month::Apr)),
        ("may", KnownToken::Month(Month::May)),
        ("june", KnownToken::Month(Month::Jun)),
        ("july", KnownToken::Month(Month::Jul)),
        ("august", KnownToken::Month(Month::Aug)),
        ("september", KnownToken::Month(Month::Sep)),
        ("october", KnownToken::Month(Month::Oct)),
        ("november", KnownToken::Month(Month::Nov)),
        ("december", KnownToken::Month(Month::Dec)),
        ("today", KnownToken::RelativeDay(RelativeDay::Today)),
        ("tomorrow", KnownToken::RelativeDay(RelativeDay::Tomorrow)),
        ("yesterday", KnownToken::RelativeDay(RelativeDay::Yesterday)),
        ("next", KnownToken::Modifier(Modifier::Next)),
        ("last", KnownToken::Modifier(Modifier::Last)),
        ("this", KnownToken::Modifier(Modifier::This)),
        ("morning", KnownToken::Time { hour: 9, min: 0, sec: None, formatted: "09:00:00".to_string() }),
        ("afternoon", KnownToken::Time { hour: 15, min: 0, sec: None, formatted: "15:00:00".to_string() }),
        ("evening", KnownToken::Time { hour: 18, min: 0, sec: None, formatted: "18:00:00".to_string() }),
        ("night", KnownToken::Time { hour: 21, min: 0, sec: None, formatted: "21:00:00".to_string() }),
        ("noon", KnownToken::Time { hour: 12, min: 0, sec: None, formatted: "12:00:00".to_string() }),
    ];

    let mut best_match = None;
    let mut best_score = 0.0;

    for (word, known) in dict {
        let dist = levenshtein(token, word) as f32;
        let max_len = token.len().max(word.len()) as f32;
        let mut score = 1.0 - (dist / max_len);
        
        if word.starts_with(token) {
            score += 0.1;
        }

        if score > best_score {
            best_score = score;
            best_match = Some(known.clone());
        }
    }

    if best_score >= 0.75 {
        best_match.map(|m| (m, best_score))
    } else {
        None
    }
}

pub fn parse_date_numeric(s: &str, config: &ParseConfig) -> Option<KnownToken> {
    let parts: Vec<&str> = s.split(|c| c == '/' || c == '-' || c == '.').collect();
    if parts.len() == 3 {
        let p1 = parts[0].parse::<i32>().ok()?;
        let p2 = parts[1].parse::<i32>().ok()?;
        let mut p3 = parts[2].parse::<i32>().ok()?;

        if p1 == 0 || p2 == 0 || p3 == 0 { return None; }

        if p1 < 1000 && p3 < 100 {
            p3 += 2000;
        }

        if p1 > 1000 {
            if p2 > 12 && p3 <= 12 {
                return Some(KnownToken::DateNumeric { y: Some(p1), m: p3 as u8, d: p2 as u8 });
            }
            if p2 <= 12 && p3 <= 31 {
                return Some(KnownToken::DateNumeric { y: Some(p1), m: p2 as u8, d: p3 as u8 });
            }
            return None;
        }

        if p3 > 1000 {
            if config.day_first || p1 > 12 {
                if p2 <= 12 && p1 <= 31 {
                    return Some(KnownToken::DateNumeric { y: Some(p3), m: p2 as u8, d: p1 as u8 });
                }
            } else {
                if p1 <= 12 && p2 <= 31 {
                    return Some(KnownToken::DateNumeric { y: Some(p3), m: p1 as u8, d: p2 as u8 });
                }
            }
            return None;
        }
    } else if parts.len() == 2 {
        let p1 = parts[0].parse::<i32>().ok()?;
        let p2 = parts[1].parse::<i32>().ok()?;

        if p1 == 0 || p2 == 0 || p1 > 31 || p2 > 31 { return None; }

        if config.day_first || p1 > 12 {
            if p2 <= 12 && p1 <= 31 {
                return Some(KnownToken::DateNumeric { y: None, m: p2 as u8, d: p1 as u8 });
            }
        } else {
            if p1 <= 12 && p2 <= 31 {
                return Some(KnownToken::DateNumeric { y: None, m: p1 as u8, d: p2 as u8 });
            }
        }
    }
    None
}
pub fn parse_time(s: &str) -> Option<KnownToken> {
    let lower = s.to_lowercase();
    let is_pm = lower.ends_with("pm") || lower.ends_with("p.m.");
    let is_am = lower.ends_with("am") || lower.ends_with("a.m.");
    let cleaned = lower.trim_end_matches("pm").trim_end_matches("am").trim_end_matches("p.m.").trim_end_matches("a.m.").trim();

    if cleaned.len() == 4 && cleaned.chars().all(|c| c.is_ascii_digit()) {
        let hour = cleaned[0..2].parse::<u8>().ok()?;
        let min = cleaned[2..4].parse::<u8>().ok()?;
        if hour < 24 && min < 60 {
            let mut h = hour;
            if is_pm && h < 12 { h += 12; }
            if is_am && h == 12 { h = 0; }
            return Some(KnownToken::Time { hour: h, min, sec: None, formatted: format!("{:02}:{:02}:00", h, min) });
        }
    }

    let parts: Vec<&str> = cleaned.split(|c| c == ':' || c == '.' || c == '-' || c == ';' || c == '|' || c == 'h').collect();
    if parts.is_empty() || parts[0].is_empty() { return None; }

    let mut hour = parts[0].parse::<u8>().ok()?;
    let min = if parts.len() > 1 && !parts[1].is_empty() { parts[1].parse::<u8>().ok()? } else { 0 };
    let sec = if parts.len() > 2 && !parts[2].is_empty() { Some(parts[2].parse::<u8>().ok()?) } else { None };

    if hour > 24 || min > 59 || sec.unwrap_or(0) > 59 { return None; }
    
    // Heuristic: If it has a separator (e.g., "-") and no explicit am/pm, 
    // it might be a date like "9-1" (Sept 1). 
    // Usually, times have '00' or '30' or '15' or are >= 13, but "9-1" is ambiguous.
    // Wait, the instructions said: "the 9-30 time doesnt resolve to a time. I think we need to be conditional about what we have, certainties, ie if we clearly have a date, a questionable entry might be more likely to be a time in an edge case."
    // Actually, if we just parse "9-00" as time, it works. For "9-1", min is 1. We could let it parse as time, but is "9-1" a time? Usually 9:01 is written "9:01". "9-1" is almost certainly a date.
    // Let's say if min < 10 and isn't padded with 0 (i.e. length is 1), and there's no am/pm, it's probably NOT a time unless the separator is `:`.
    if !is_am && !is_pm && parts.len() == 2 && !s.contains(':') && !s.contains('.') && parts[1].len() != 2 {
        return None;
    }

    if is_pm && hour < 12 { hour += 12; }
    if is_am && hour == 12 { hour = 0; }

    let formatted = if is_pm || is_am {
        format!("{:02}:{:02}:00", hour, min)
    } else if parts.len() == 1 {
        if hour <= 12 {
            hour += 12;
            if hour == 24 { hour = 0; }
            format!("{:02}:00:00", hour)
        } else {
            format!("{:02}:00:00", hour)
        }
    } else {
        if let Some(s) = sec {
            format!("{:02}:{:02}:{:02}", hour, min, s)
        } else {
            format!("{:02}:{:02}:00", hour, min)
        }
    };

    Some(KnownToken::Time { hour, min, sec, formatted })
}

pub fn parse_iso(s: &str) -> Option<Vec<KnownToken>> {
    let lower = s.to_lowercase();
    if let Some((date_part, rest)) = lower.split_once('t') {
        let mut tokens = Vec::new();
        if let Some(d) = parse_date_numeric(date_part, &ParseConfig { day_first: false, resolve_dates: false }) {
            tokens.push(d);
        } else {
            return None;
        }

        let time_part;
        let tz_part;
        if rest.ends_with('z') {
            time_part = &rest[..rest.len()-1];
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

        if let Some(mut t) = parse_time(time_part) {
            if let KnownToken::Time { formatted, .. } = &mut t {
                *formatted = time_part.to_string(); // Keep exactly as ISO
            }
            tokens.push(t);
        } else {
            return None;
        }

        if let Some(tz) = tz_part {
            if let Some(tz_token) = parse_timezone(tz) {
                tokens.push(tz_token);
            }
        }
        return Some(tokens);
    }
    None
}

pub fn parse_timezone(s: &str) -> Option<KnownToken> {
    let lower = s.to_lowercase();
    if lower == "z" { return Some(KnownToken::TimeZone(TimeZoneKind::Z)); }
    if lower == "utc" || lower == "gmt" { return Some(KnownToken::TimeZone(TimeZoneKind::Utc)); }

    if lower.starts_with("utc") || lower.starts_with("gmt") {
        let rest = &lower[3..];
        return parse_offset(rest);
    }
    if lower.starts_with('+') || lower.starts_with('-') {
        return parse_offset(&lower);
    }

    if lower.contains('/') {
        return Some(KnownToken::TimeZone(TimeZoneKind::Named(s.to_string())));
    }
    
    let tz_abbrs = ["est", "edt", "cst", "cdt", "mst", "mdt", "pst", "pdt", "cet", "cest", "eet", "eest", "bst", "jst", "ist", "aest", "aedt"];
    if tz_abbrs.contains(&lower.as_str()) {
        return Some(KnownToken::TimeZone(TimeZoneKind::Named(s.to_uppercase())));
    }

    None
}

fn parse_offset(s: &str) -> Option<KnownToken> {
    let s = s.trim();
    if s.is_empty() { return None; }
    let sign = if s.starts_with('+') { 1 } else if s.starts_with('-') { -1 } else { return None; };
    let rest = &s[1..];
    
    let (h, m) = if rest.contains(':') {
        let parts: Vec<&str> = rest.split(':').collect();
        let h = parts[0].parse::<i8>().ok()?;
        let m = parts[1].parse::<i8>().ok()?;
        (h, m)
    } else if rest.len() == 4 {
        let h = rest[0..2].parse::<i8>().ok()?;
        let m = rest[2..4].parse::<i8>().ok()?;
        (h, m)
    } else {
        let h = rest.parse::<i8>().ok()?;
        (h, 0)
    };

    Some(KnownToken::TimeZone(TimeZoneKind::Offset { hours: h * sign, minutes: m * sign }))
}

pub fn classify(raw: Vec<RawToken>, config: &ParseConfig) -> Vec<Token> {
    let mut tokens = Vec::new();
    for r in raw {
        let s = r.as_str();

        if let Some(k) = lookup_dict(s) {
            tokens.push(Token::Known(k));
            continue;
        }

        if s.contains('t') && (s.contains('-') || s.contains('z') || s.contains('+') || s.len() > 10) {
            if let Some(iso_tokens) = parse_iso(s) {
                for it in iso_tokens {
                    tokens.push(Token::Known(it));
                }
                continue;
            }
        }

        if let Some(t) = parse_time(s) {
            tokens.push(Token::Known(t));
            continue;
        }

        if let Some(d) = parse_date_numeric(s, config) {
            tokens.push(Token::Known(d));
            continue;
        }

        if let Some(tz) = parse_timezone(s) {
            tokens.push(Token::Known(tz));
            continue;
        }

        if let Ok(n) = s.parse::<i32>() {
            if s.len() == 4 {
                if let Some(t) = parse_time(s) {
                    tokens.push(Token::Known(t));
                    continue;
                }
            }
            tokens.push(Token::Known(KnownToken::Number(n)));
            continue;
        }

        if let Some((k, _score)) = fuzzy_match(s) {
            tokens.push(Token::Known(k));
            continue;
        }

        if s == "the" || s == "a" || s == "coming" || s == "after" || s == "of" || s == "oclock" || s == "clock" {
            tokens.push(Token::Noise(s.to_string()));
        } else {
            tokens.push(Token::Unknown(UnknownToken::Word(s.to_string())));
        }
    }
    tokens
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
    let mut d = NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap() - Duration::days(1);
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

pub fn resolve(tokens: Vec<Token>) -> Vec<Token> {
    let now = Local::now().date_naive();
    let mut resolved = Vec::new();
    let mut current_modifier = None;

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Known(KnownToken::Modifier(m)) => {
                current_modifier = Some(m.clone());
            }
            Token::Known(KnownToken::RelativeDay(r)) => {
                let d = match r {
                    RelativeDay::Today => now,
                    RelativeDay::Tomorrow => now + Duration::days(1),
                    RelativeDay::Yesterday => now - Duration::days(1),
                };
                resolved.push(Token::Known(KnownToken::DateNumeric { 
                    y: Some(d.year()), 
                    m: d.month() as u8, 
                    d: d.day() as u8 
                }));
                current_modifier = None;
            }
            Token::Known(KnownToken::Weekday(w)) => {
                let target_wd = match w {
                    Weekday::Monday => chrono::Weekday::Mon,
                    Weekday::Tuesday => chrono::Weekday::Tue,
                    Weekday::Wednesday => chrono::Weekday::Wed,
                    Weekday::Thursday => chrono::Weekday::Thu,
                    Weekday::Friday => chrono::Weekday::Fri,
                    Weekday::Saturday => chrono::Weekday::Sat,
                    Weekday::Sunday => chrono::Weekday::Sun,
                };
                
                let mut days_ahead = target_wd.num_days_from_monday() as i64 - now.weekday().num_days_from_monday() as i64;
                if days_ahead <= 0 {
                    days_ahead += 7;
                }
                
                if let Some(Modifier::Next) = current_modifier {
                    days_ahead += 7;
                } else if let Some(Modifier::Last) = current_modifier {
                    days_ahead -= 14; 
                    if days_ahead < -7 { days_ahead += 7; } 
                }
                
                if i + 1 < tokens.len() {
                    if let Token::Known(KnownToken::Unit(Unit::Week)) = &tokens[i+1] {
                        days_ahead += 7;
                        i += 1;
                    }
                }
                
                let d = now + Duration::days(days_ahead);
                resolved.push(Token::Known(KnownToken::DateNumeric { 
                    y: Some(d.year()), 
                    m: d.month() as u8, 
                    d: d.day() as u8 
                }));
                current_modifier = None;
            }
            Token::Known(KnownToken::Holiday(h)) => {
                let current_year = now.year();
                let mut h_date = resolve_holiday(h, current_year);
                if h_date < now {
                    h_date = resolve_holiday(h, current_year + 1);
                }
                resolved.push(Token::Known(KnownToken::DateNumeric { 
                    y: Some(h_date.year()), 
                    m: h_date.month() as u8, 
                    d: h_date.day() as u8 
                }));
                current_modifier = None;
            }
            Token::Known(KnownToken::DateNumeric { y, m, d }) => {
                let yr = *y;
                let mo = *m;
                let da = *d;
                if let Some(modi) = current_modifier.take() {
                    resolved.push(Token::Known(KnownToken::Modifier(modi)));
                }
                let mut final_year = yr.unwrap_or(now.year());
                if yr.is_none() {
                    let target_date = NaiveDate::from_ymd_opt(now.year(), mo as u32, da as u32).unwrap_or(now);
                    if target_date < now {
                        final_year = now.year() + 1;
                    }
                }
                
                resolved.push(Token::Known(KnownToken::DateNumeric { 
                    y: Some(final_year), 
                    m: mo, 
                    d: da 
                }));
            }
            other => {
                if let Some(m) = current_modifier.take() {
                    resolved.push(Token::Known(KnownToken::Modifier(m)));
                }
                resolved.push(other.clone());
            }
        }
        i += 1;
    }
    
    if let Some(m) = current_modifier {
        resolved.push(Token::Known(KnownToken::Modifier(m)));
    }
    
    resolved
}

pub fn to_canonical(mut tokens: Vec<Token>) -> String {
    tokens.retain(|t| !matches!(t, Token::Noise(_)));
    tokens.retain(|t| !matches!(t, Token::Unknown(_)));

    let mut modifier = None;
    let mut date_str = None;
    let mut time_str = None;
    let mut tz_str = None;

    for t in tokens {
        if let Token::Known(k) = t {
            match k {
                KnownToken::Modifier(m) => {
                    modifier = Some(match m {
                        Modifier::Next => "next",
                        Modifier::Last => "last",
                        Modifier::This => "this",
                    });
                }
                KnownToken::RelativeDay(r) => {
                    date_str = Some(match r {
                        RelativeDay::Today => "today".to_string(),
                        RelativeDay::Tomorrow => "tomorrow".to_string(),
                        RelativeDay::Yesterday => "yesterday".to_string(),
                    });
                }
                KnownToken::Weekday(w) => {
                    date_str = Some(match w {
                        Weekday::Monday => "monday".to_string(),
                        Weekday::Tuesday => "tuesday".to_string(),
                        Weekday::Wednesday => "wednesday".to_string(),
                        Weekday::Thursday => "thursday".to_string(),
                        Weekday::Friday => "friday".to_string(),
                        Weekday::Saturday => "saturday".to_string(),
                        Weekday::Sunday => "sunday".to_string(),
                    });
                }
                KnownToken::DateNumeric { y, m, d } => {
                    if let Some(year) = y {
                        date_str = Some(format!("{:04}-{:02}-{:02}", year, m, d));
                    } else {
                        date_str = Some(format!("{:02}-{:02}", m, d));
                    }
                }
                KnownToken::Time { formatted, .. } => {
                    time_str = Some(formatted);
                }
                KnownToken::TimeZone(tz) => {
                    tz_str = Some(match tz {
                        TimeZoneKind::Z => "UTC".to_string(),
                        TimeZoneKind::Utc => "UTC".to_string(),
                        TimeZoneKind::Offset { hours, minutes } => {
                            if minutes == 0 {
                                format!("UTC{:+}", hours)
                            } else {
                                format!("UTC{:+}:{:02}", hours, minutes)
                            }
                        }
                        TimeZoneKind::Named(n) => n.clone(),
                    });
                }
                _ => {}
            }
        }
    }

    let mut parts = Vec::new();
    if let Some(m) = modifier {
        if let Some(d) = date_str {
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

    let joined = parts.join(" ");
    
    if joined.contains("at") && tz_str.is_some() && joined.matches('-').count() >= 2 && joined.matches(':').count() >= 2 {
    }

    joined
}

pub fn tokenize_and_classify(input: &str, config: &ParseConfig) -> Vec<Token> {
    let norm = normalize(input);
    let raw = tokenize(&norm);
    let recombined = recombine(raw);
    let classified = classify(recombined, config);
    if config.resolve_dates {
        resolve(classified)
    } else {
        classified
    }
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

    for t in tokens {
        if let Token::Known(k) = t {
            match k {
                KnownToken::DateNumeric { y: year, m: month, d: day } => {
                    y = *year;
                    m = Some(*month);
                    d = Some(*day);
                }
                KnownToken::Time { hour, min: minute, sec, .. } => {
                    h = Some(*hour);
                    min = Some(*minute);
                    s = *sec;
                }
                KnownToken::TimeZone(tz_kind) => {
                    tz = Some(match tz_kind {
                        TimeZoneKind::Z => "UTC".to_string(),
                        TimeZoneKind::Utc => "UTC".to_string(),
                        TimeZoneKind::Offset { hours, minutes } => {
                            if *minutes == 0 {
                                format!("UTC{:+}", hours)
                            } else {
                                format!("UTC{:+}:{:02}", hours, minutes)
                            }
                        }
                        TimeZoneKind::Named(n) => n.clone(),
                    });
                }
                KnownToken::RelativeDay(r) => {
                    rel = Some(match r {
                        RelativeDay::Today => "today",
                        RelativeDay::Tomorrow => "tomorrow",
                        RelativeDay::Yesterday => "yesterday",
                    });
                }
                KnownToken::Weekday(w) => {
                    rel = Some(match w {
                        Weekday::Monday => "monday",
                        Weekday::Tuesday => "tuesday",
                        Weekday::Wednesday => "wednesday",
                        Weekday::Thursday => "thursday",
                        Weekday::Friday => "friday",
                        Weekday::Saturday => "saturday",
                        Weekday::Sunday => "sunday",
                    });
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
    to_canonical(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmrw_at_3() {
        let config = ParseConfig { day_first: false, resolve_dates: false };
        assert_eq!(process("tmrw @ 3", &config), "tomorrow at 15:00:00");
    }

    #[test]
    fn test_nxt_fri_14_00() {
        let config = ParseConfig { day_first: false, resolve_dates: false };
        assert_eq!(process("nxt fri 14:00", &config), "next friday at 14:00:00");
    }

    #[test]
    fn test_date_numeric() {
        let config = ParseConfig { day_first: true, resolve_dates: false }; 
        assert_eq!(process("03/04/2026", &config), "2026-04-03"); 
    }

    #[test]
    fn test_iso() {
        let config = ParseConfig { day_first: false, resolve_dates: false };
        assert_eq!(process("2026-03-18T08:00:00Z", &config), "2026-03-18 at 08:00:00 UTC"); 
    }

    #[test]
    fn test_tz() {
        let config = ParseConfig { day_first: false, resolve_dates: false };
        assert_eq!(process("08:00:00 UTC+2", &config), "08:00:00 UTC+2");
    }

    #[test]
    fn test_fri_day_3pm() {
        let config = ParseConfig { day_first: false, resolve_dates: false };
        assert_eq!(process("fri day 3 pm", &config), "friday at 15:00:00");
    }
    
    #[test]
    fn test_word_numbers() {
        let config = ParseConfig { day_first: false, resolve_dates: false };
        assert_eq!(process("nine thirty-five", &config), "09:35:00");
        assert_eq!(process("nine thirty five", &config), "09:35:00");
        assert_eq!(process("ten fifteen", &config), "10:15:00");
        assert_eq!(process("half past ten", &config), "10:30:00");
        assert_eq!(process("quarter to ten", &config), "09:45:00");
        assert_eq!(process("quarter past ten", &config), "10:15:00");
    }
    
    #[test]
    fn test_holidays() {
        let config = ParseConfig { day_first: false, resolve_dates: true };
        let year = chrono::Local::now().date_naive().year();
        let next_year = year + 1;
        assert!(process("christmas", &config).starts_with(&year.to_string()) || process("christmas", &config).starts_with(&next_year.to_string()));
    }
}
