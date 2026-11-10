use chrono::{DateTime, Datelike, Duration, Local, Months, TimeZone, Timelike};

#[derive(Default)]
struct Pending {
    months: i64,
    secs: i64,
}

pub fn expand_macros(input: &str) -> String {
    expand_at(input, Local::now())
}

pub fn expand_at(input: &str, now: DateTime<Local>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut work = now;
    let mut pending = Pending::default();
    let mut i = 0usize;
    while i < input.len() {
        if input[i..].starts_with("%(") {
            if let Some(rel) = input[i + 2..].find(')') {
                let end = i + 2 + rel;
                let token = &input[i + 2..end];
                if let Some((kind, n)) = parse_adjust(token) {
                    match kind {
                        Adjust::Months => pending.months += n * 12,
                        Adjust::MonthsUnit => pending.months += n,
                        Adjust::Secs => pending.secs += n,
                    }
                    i = end + 1;
                    continue;
                }
                if let Some(text) = format_token(token, &mut work, &mut pending) {
                    out.push_str(&text);
                    i = end + 1;
                    continue;
                }
                out.push_str(&input[i..=end]);
                i = end + 1;
                continue;
            }
        }
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

enum Adjust {
    Months,
    MonthsUnit,
    Secs,
}

fn parse_adjust(token: &str) -> Option<(Adjust, i64)> {
    let lower = token.to_lowercase();
    let table: [(&str, Adjust, i64); 21] = [
        ("addyears", Adjust::Months, 12),
        ("addmonths", Adjust::MonthsUnit, 1),
        ("adddays", Adjust::Secs, 86_400),
        ("addhours", Adjust::Secs, 3_600),
        ("addminutes", Adjust::Secs, 60),
        ("addseconds", Adjust::Secs, 1),
        ("subyears", Adjust::Months, -12),
        ("submonths", Adjust::MonthsUnit, -1),
        ("subdays", Adjust::Secs, -86_400),
        ("subhours", Adjust::Secs, -3_600),
        ("subminutes", Adjust::Secs, -60),
        ("subseconds", Adjust::Secs, -1),
        ("subtractyears", Adjust::Months, -12),
        ("subtractmonths", Adjust::MonthsUnit, -1),
        ("subtractdays", Adjust::Secs, -86_400),
        ("subtracthours", Adjust::Secs, -3_600),
        ("subtractminutes", Adjust::Secs, -60),
        ("subtractseconds", Adjust::Secs, -1),
        ("addminutes", Adjust::Secs, 60),
        ("addseconds", Adjust::Secs, 1),
        ("addhours", Adjust::Secs, 3_600),
    ];
    for (name, kind, mult) in table {
        if lower.starts_with(name) {
            let rest = &lower[name.len()..];
            let rest = rest.trim();
            let n = if rest.is_empty() {
                1
            } else {
                rest.parse::<i64>().ok()?
            };
            return Some((kind, n * mult));
        }
    }
    None
}

fn apply_pending(work: &mut DateTime<Local>, pending: &mut Pending) {
    if pending.months > 0 {
        if let Some(dt) = work.checked_add_months(Months::new(pending.months as u32)) {
            *work = dt;
        }
    } else if pending.months < 0 {
        if let Some(dt) = work.checked_sub_months(Months::new((-pending.months) as u32)) {
            *work = dt;
        }
    }
    if pending.secs != 0 {
        if let Some(dt) = work.checked_add_signed(Duration::seconds(pending.secs)) {
            *work = dt;
        }
    }
    *pending = Pending::default();
}

fn format_token(token: &str, work: &mut DateTime<Local>, pending: &mut Pending) -> Option<String> {
    let apply_date = !matches!(token, "%(clipboard)" | "%(cursor)");
    if apply_date {
        apply_pending(work, pending);
    }
    let s = match token {
        "Year" => work.format("%Y").to_string(),
        "YearShort" => work.format("%y").to_string(),
        "Month" => work.format("%m").to_string(),
        "Day" => work.format("%d").to_string(),
        "MonthName" => work.format("%B").to_string(),
        "MonthNameShort" => work.format("%b").to_string(),
        "WeekdayName" => work.format("%A").to_string(),
        "WeekdayNameShort" => work.format("%a").to_string(),
        "Hour" => work.format("%H").to_string(),
        "#Hour" => work.format("%-I").to_string(),
        "Minute" => work.format("%M").to_string(),
        "Second" => work.format("%S").to_string(),
        "AMorPM" => {
            if work.hour() < 12 {
                "AM".to_string()
            } else {
                "PM".to_string()
            }
        }
        "#DayOfWeek" => work.weekday().num_days_from_sunday().to_string(),
        "#DayOfWeekMon" => work.weekday().number_from_monday().to_string(),
        "#WeekOfYear" => work.format("%U").to_string(),
        "clipboard" => clipboard_text().unwrap_or_default(),
        "cursor" => String::new(),
        _ => return None,
    };
    Some(s)
}

fn clipboard_text() -> Option<String> {
    let mut cb = arboard::Clipboard::new().ok()?;
    cb.get_text().ok()
}

pub fn local_datetime(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> Option<DateTime<Local>> {
    Local.with_ymd_and_hms(y, mo, d, h, mi, s).single()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed() -> DateTime<Local> {
        local_datetime(2026, 11, 10, 14, 5, 9).unwrap()
    }

    #[test]
    fn formats_date_tokens() {
        let out = expand_at("%(WeekdayName), %(MonthName) %(Day), %(Year)", fixed());
        assert_eq!(out, "Tuesday, November 10, 2026");
    }

    #[test]
    fn subtracts_days_for_following_token() {
        let out = expand_at("%(SubDays 1)%(WeekdayName)", fixed());
        assert_eq!(out, "Monday");
    }

    #[test]
    fn adds_months_across_year() {
        let out = expand_at("%(AddMonths 2)%(Month)/%(Day)/%(Year)", fixed());
        assert_eq!(out, "01/10/2027");
    }

    #[test]
    fn formats_12_hour_clock() {
        let out = expand_at("%(#Hour):%(Minute) %(AMorPM)", fixed());
        assert_eq!(out, "2:05 PM");
    }

    #[test]
    fn leaves_unclosed_token_alone() {
        let out = expand_at("abc %(Year", fixed());
        assert_eq!(out, "abc %(Year");
    }
}
