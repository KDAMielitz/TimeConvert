use chrono::{
    DateTime, Duration, LocalResult, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;
use std::collections::HashMap;

/// Parses a time typed by the user: 24-hour (`19:30`, `1930`, `930`) or
/// 12-hour with an AM/PM suffix (`7:30 PM`, `730pm`, `12am`).
pub fn parse_time(input: &str) -> Option<NaiveTime> {
    let lower = input.to_ascii_lowercase();
    for (suffix, pm) in [("am", false), ("pm", true)] {
        if let Some(core) = lower.strip_suffix(suffix) {
            let core = core.trim_end();
            // A bare hour such as "7pm" or "12 am" is also fine.
            let time = if !core.is_empty() && core.len() <= 2 && core.bytes().all(|b| b.is_ascii_digit())
            {
                parse_time_24(&format!("{core}:00"))?
            } else {
                parse_time_24(core)?
            };
            let hour = time.hour12().1;
            if time.hour() == 0 || time.hour() > 12 {
                return None;
            }
            let hour24 = match (hour, pm) {
                (12, false) => 0,
                (12, true) => 12,
                (h, true) => h + 12,
                (h, false) => h,
            };
            return NaiveTime::from_hms_opt(hour24, time.minute(), 0);
        }
    }
    parse_time_24(input)
}

/// Parses a time for the 12-hour mode, where AM/PM comes from a separate toggle.
/// An explicit suffix (`7:30pm`) wins; hours 1-12 follow the toggle; hours 0 and
/// 13-23 are unambiguous 24-hour times and are taken as typed.
pub fn parse_time_12(input: &str, pm: bool) -> Option<NaiveTime> {
    let time = parse_time(input)?;
    let lower = input.to_ascii_lowercase();
    if lower.ends_with("am") || lower.ends_with("pm") {
        return Some(time);
    }
    let hour = time.hour();
    if (1..=12).contains(&hour) {
        let hour24 = match (hour, pm) {
            (12, false) => 0,
            (12, true) => 12,
            (h, true) => h + 12,
            (h, false) => h,
        };
        NaiveTime::from_hms_opt(hour24, time.minute(), 0)
    } else {
        Some(time)
    }
}

fn parse_time_24(input: &str) -> Option<NaiveTime> {
    let bytes = input.as_bytes();
    let (hours, minutes) = match bytes {
        [_, m1, m2] if bytes.iter().all(u8::is_ascii_digit) => (&bytes[..1], [*m1, *m2]),
        [_, _, _, _] if bytes.iter().all(u8::is_ascii_digit) => (&bytes[..2], [bytes[2], bytes[3]]),
        [_, b':', _, _]
            if bytes[0].is_ascii_digit() && bytes[2..].iter().all(u8::is_ascii_digit) =>
        {
            (&bytes[..1], [bytes[2], bytes[3]])
        }
        [_, _, b':', _, _]
            if bytes[..2].iter().all(u8::is_ascii_digit)
                && bytes[3..].iter().all(u8::is_ascii_digit) =>
        {
            (&bytes[..2], [bytes[3], bytes[4]])
        }
        _ => return None,
    };
    let hour = hours.iter().fold(0u32, |n, b| n * 10 + u32::from(b - b'0'));
    let minute = u32::from(minutes[0] - b'0') * 10 + u32::from(minutes[1] - b'0');
    NaiveTime::from_hms_opt(hour, minute, 0)
}

pub struct Resolution {
    pub instant: DateTime<Utc>,
    pub note: Option<String>,
}
pub fn resolve(tz: Tz, local: NaiveDateTime) -> Option<Resolution> {
    match tz.from_local_datetime(&local) {
        LocalResult::Single(v) => Some(Resolution {
            instant: v.to_utc(),
            note: None,
        }),
        LocalResult::Ambiguous(a, b) => Some(Resolution {
            instant: a.min(b).to_utc(),
            note: Some("Ambiguous time: earlier occurrence selected".into()),
        }),
        LocalResult::None => {
            // Advance to the first valid minute after the gap, including date-line changes.
            (1..=2880).find_map(
                |m| match tz.from_local_datetime(&(local + Duration::minutes(m))) {
                    LocalResult::Single(v) => Some(Resolution {
                        instant: v.to_utc(),
                        note: Some(format!("Time does not exist; advanced {m} minutes")),
                    }),
                    LocalResult::Ambiguous(a, b) => Some(Resolution {
                        instant: a.min(b).to_utc(),
                        note: Some(format!("Time does not exist; advanced {m} minutes")),
                    }),
                    LocalResult::None => None,
                },
            )
        }
    }
}
#[derive(Clone)]
pub struct Zone {
    pub tz: Tz,
    pub label: String,
    pub search: String,
    /// Space-separated lowercase abbreviations such as "cdt cst".
    pub abbrs: String,
}
pub fn zones() -> Vec<Zone> {
    let countries: HashMap<&str, &str> = include_str!("../iso3166.tab")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .collect();
    let mut result = Vec::new();
    for line in include_str!("../zone.tab")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() < 3 {
            continue;
        }
        if let Ok(tz) = fields[2].parse::<Tz>() {
            for code in fields[0].split(',') {
                if let Some(country) = countries.get(code) {
                    let label = format!("{} - {}", country, fields[2].replace('_', " "));
                    let aliases = match code {
                        "KR" => "south korea republic of korea",
                        "KP" => "north korea",
                        "GB" => "uk united kingdom britain great britain",
                        "US" => "usa united states america",
                        "CZ" => "czechia czech republic",
                        "RU" => "russia",
                        "VN" => "vietnam",
                        "TW" => "taiwan",
                        "TR" => "turkey turkiye",
                        _ => "",
                    };
                    let abbrs = abbreviations_for(fields[2]).to_string();
                    let search =
                        format!("{} {} {} {} {}", label, fields[2], code, aliases, abbrs)
                            .to_lowercase();
                    result.push(Zone {
                        tz,
                        label,
                        search,
                        abbrs,
                    });
                }
            }
        }
    }
    result.push(Zone {
        tz: Tz::UTC,
        label: "UTC - Coordinated Universal Time".to_string(),
        search: "utc - coordinated universal time z zulu".to_string(),
        abbrs: "utc z".to_string(),
    });
    result.sort_by(|a, b| a.label.cmp(&b.label));
    result
}
/// Common time-zone abbreviations. Abbreviations are ambiguous worldwide
/// (e.g. CST, IST), so each maps to the IANA zones it is commonly used for;
/// the zone's own rules still decide DST.
const ABBREVIATIONS: &[(&str, &str)] = &[
    ("America/New_York", "est edt et"),
    ("America/Toronto", "est edt et"),
    ("America/Chicago", "cst cdt ct"),
    ("America/Mexico_City", "cst"),
    ("America/Denver", "mst mdt mt"),
    ("America/Phoenix", "mst"),
    ("America/Los_Angeles", "pst pdt pt"),
    ("America/Vancouver", "pst pdt pt"),
    ("America/Anchorage", "akst akdt"),
    ("Pacific/Honolulu", "hst"),
    ("America/Halifax", "ast adt"),
    ("America/St_Johns", "nst ndt"),
    ("America/Sao_Paulo", "brt"),
    ("America/Argentina/Buenos_Aires", "art"),
    ("Europe/London", "gmt bst"),
    ("Europe/Dublin", "gmt ist"),
    ("Europe/Lisbon", "wet west"),
    ("Europe/Berlin", "cet cest"),
    ("Europe/Paris", "cet cest"),
    ("Europe/Madrid", "cet cest"),
    ("Europe/Rome", "cet cest"),
    ("Europe/Amsterdam", "cet cest"),
    ("Europe/Brussels", "cet cest"),
    ("Europe/Vienna", "cet cest"),
    ("Europe/Zurich", "cet cest"),
    ("Europe/Stockholm", "cet cest"),
    ("Europe/Oslo", "cet cest"),
    ("Europe/Copenhagen", "cet cest"),
    ("Europe/Warsaw", "cet cest"),
    ("Europe/Prague", "cet cest"),
    ("Europe/Budapest", "cet cest"),
    ("Europe/Athens", "eet eest"),
    ("Europe/Helsinki", "eet eest"),
    ("Europe/Kyiv", "eet eest"),
    ("Europe/Bucharest", "eet eest"),
    ("Africa/Cairo", "eet"),
    ("Europe/Moscow", "msk"),
    ("Europe/Istanbul", "trt"),
    ("Africa/Johannesburg", "sast"),
    ("Africa/Lagos", "wat"),
    ("Africa/Nairobi", "eat"),
    ("Asia/Jerusalem", "ist idt"),
    ("Asia/Dubai", "gst"),
    ("Asia/Karachi", "pkt"),
    ("Asia/Kolkata", "ist"),
    ("Asia/Shanghai", "cst"),
    ("Asia/Hong_Kong", "hkt"),
    ("Asia/Singapore", "sgt"),
    ("Asia/Tokyo", "jst"),
    ("Asia/Seoul", "kst"),
    ("Australia/Sydney", "aest aedt"),
    ("Australia/Melbourne", "aest aedt"),
    ("Australia/Brisbane", "aest"),
    ("Australia/Adelaide", "acst acdt"),
    ("Australia/Darwin", "acst"),
    ("Australia/Perth", "awst"),
    ("Pacific/Auckland", "nzst nzdt"),
];

fn abbreviations_for(name: &str) -> &'static str {
    ABBREVIATIONS
        .iter()
        .find(|(tz, _)| *tz == name)
        .map_or("", |(_, abbr)| abbr)
}

pub fn matches(zone: &Zone, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    // A query that is exactly an abbreviation (e.g. "cdt") matches only the
    // zones that use it, instead of every label containing those letters.
    if !query.is_empty() && !query.contains(char::is_whitespace) && is_abbreviation(&query) {
        return zone.abbrs.split(' ').any(|a| a == query);
    }
    query
        .split_whitespace()
        .all(|word| zone.search.contains(word))
}

fn is_abbreviation(word: &str) -> bool {
    word == "utc"
        || word == "z"
        || ABBREVIATIONS
            .iter()
            .any(|(_, abbr)| abbr.split(' ').any(|a| a == word))
}
pub fn row_label(mut row: usize) -> String {
    let mut result = String::new();
    loop {
        result.insert(0, (b'A' + (row % 26) as u8) as char);
        if row < 26 {
            break;
        }
        row = row / 26 - 1;
    }
    result
}
pub fn day_label(diff: i64, reference: usize, row: usize) -> String {
    if row == reference {
        "  (reference)".into()
    } else {
        format!(
            "  ({diff:+} day{} vs {})",
            if diff.abs() == 1 { "" } else { "s" },
            row_label(reference)
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    #[test]
    fn time_input_formats() {
        for (input, hour, minute) in [
            ("1930", 19, 30),
            ("0905", 9, 5),
            ("930", 9, 30),
            ("0000", 0, 0),
            ("19:30", 19, 30),
            ("9:05", 9, 5),
        ] {
            assert_eq!(
                parse_time(input),
                NaiveTime::from_hms_opt(hour, minute, 0),
                "{input}"
            );
        }
        for input in [
            "2460", "1960", "2400", "9:60", "ab30", "9:0x", "12:345", " 930", "", "93",
        ] {
            assert_eq!(parse_time(input), None, "{input}");
        }
    }
    #[test]
    fn twelve_hour_toggle_input() {
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0);
        assert_eq!(parse_time_12("12:45", false), t(0, 45));
        assert_eq!(parse_time_12("12:45", true), t(12, 45));
        assert_eq!(parse_time_12("7:30", false), t(7, 30));
        assert_eq!(parse_time_12("7:30", true), t(19, 30));
        assert_eq!(parse_time_12("730", true), t(19, 30));
        assert_eq!(parse_time_12("1:05", true), t(13, 5));
        // Explicit suffix beats the toggle.
        assert_eq!(parse_time_12("7:30 am", true), t(7, 30));
        assert_eq!(parse_time_12("7:30 PM", false), t(19, 30));
        // Unambiguous 24-hour input is taken as typed.
        assert_eq!(parse_time_12("1930", false), t(19, 30));
        assert_eq!(parse_time_12("0:15", true), t(0, 15));
        assert_eq!(parse_time_12("00:15", true), t(0, 15));
        assert_eq!(parse_time_12("13:00", false), t(13, 0));
        assert_eq!(parse_time_12("25:00", true), None);
        assert_eq!(parse_time_12("7:75", true), None);
        assert_eq!(parse_time_12("", false), None);
    }
    #[test]
    fn twelve_hour_input() {
        for (input, hour, minute) in [
            ("7:30 PM", 19, 30),
            ("7:30pm", 19, 30),
            ("730pm", 19, 30),
            ("0730 PM", 19, 30),
            ("7:30 am", 7, 30),
            ("12:00 AM", 0, 0),
            ("12am", 0, 0),
            ("12:15 PM", 12, 15),
            ("12pm", 12, 0),
            ("7pm", 19, 0),
            ("11:59 PM", 23, 59),
            ("1:05 AM", 1, 5),
        ] {
            assert_eq!(
                parse_time(input),
                NaiveTime::from_hms_opt(hour, minute, 0),
                "{input}"
            );
        }
        for input in ["0:30 am", "13:00 pm", "13pm", "7:60 pm", "pm", " 7pm", "7:3x pm", "99pm"] {
            assert_eq!(parse_time(input), None, "{input}");
        }
    }
    fn date(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }
    #[test]
    fn gap() {
        let r = resolve(chrono_tz::America::New_York, date(2024, 3, 10, 2, 30)).unwrap();
        assert_eq!(r.instant.naive_utc(), date(2024, 3, 10, 7, 0));
        assert!(r.note.is_some());
    }
    #[test]
    fn fold() {
        let r = resolve(chrono_tz::America::New_York, date(2024, 11, 3, 1, 30)).unwrap();
        assert_eq!(r.instant.naive_utc(), date(2024, 11, 3, 5, 30));
        assert!(r.note.is_some());
    }
    #[test]
    fn rollover() {
        let r = resolve(chrono_tz::America::New_York, date(2024, 1, 1, 23, 30)).unwrap();
        assert_eq!(
            r.instant
                .with_timezone(&chrono_tz::Asia::Tokyo)
                .date_naive(),
            date(2024, 1, 2, 0, 0).date()
        );
    }
    #[test]
    fn country_search() {
        let z = zones();
        assert!(z
            .iter()
            .any(|x| x.tz == chrono_tz::Asia::Tokyo && matches(x, "japan")));
        assert!(z
            .iter()
            .any(|x| x.tz == chrono_tz::Europe::Berlin && matches(x, "germany")));
    }
    #[test]
    fn abbreviation_search() {
        let z = zones();
        for (q, tz) in [
            ("UTC", chrono_tz::UTC),
            ("cdt", chrono_tz::America::Chicago),
            ("CST", chrono_tz::America::Chicago),
            ("cst", chrono_tz::Asia::Shanghai),
            ("AEST", chrono_tz::Australia::Sydney),
            ("pst", chrono_tz::America::Los_Angeles),
            ("cest", chrono_tz::Europe::Berlin),
        ] {
            assert!(z.iter().any(|x| x.tz == tz && matches(x, q)), "{q}");
        }
        // An exact abbreviation is not diluted by substring hits.
        assert!(!z
            .iter()
            .any(|x| x.tz == chrono_tz::Europe::Oslo && matches(x, "est")));
    }
    #[test]
    fn search_tokens() {
        let z = zones();
        for (q, tz) in [
            ("oslo", chrono_tz::Europe::Oslo),
            ("south korea", chrono_tz::Asia::Seoul),
            ("uk london", chrono_tz::Europe::London),
        ] {
            assert!(z.iter().any(|x| x.tz == tz && matches(x, q)), "{q}");
        }
    }
    #[test]
    fn offsets() {
        let now = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap();
        assert_eq!(
            now.with_timezone(&chrono_tz::Asia::Kolkata)
                .format("%:z")
                .to_string(),
            "+05:30"
        );
        assert_eq!(
            now.with_timezone(&chrono_tz::Pacific::Kiritimati)
                .format("%:z")
                .to_string(),
            "+14:00"
        );
    }
    #[test]
    fn day_labels() {
        assert_eq!(day_label(1, 0, 1), "  (+1 day vs A)");
        assert_eq!(day_label(-1, 1, 0), "  (-1 day vs B)");
        assert_eq!(day_label(0, 1, 0), "  (+0 days vs B)");
        assert_eq!(day_label(0, 0, 0), "  (reference)");
        assert_eq!(row_label(25), "Z");
        assert_eq!(row_label(26), "AA");
        assert_eq!(row_label(702), "AAA");
    }
}
