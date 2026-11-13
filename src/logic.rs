use chrono::{DateTime, Duration, LocalResult, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::collections::HashMap;

pub fn parse_time(input: &str) -> Option<NaiveTime> {
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
                    let search =
                        format!("{} {} {} {}", label, fields[2], code, aliases).to_lowercase();
                    result.push(Zone { tz, label, search });
                }
            }
        }
    }
    result.sort_by(|a, b| a.label.cmp(&b.label));
    result
}
pub fn matches(zone: &Zone, query: &str) -> bool {
    query
        .split_whitespace()
        .all(|word| zone.search.contains(word))
}
pub fn day_label(diff: i64, reference: usize, row: usize) -> String {
    if row == reference {
        "  (reference)".into()
    } else {
        format!(
            "  ({diff:+} day{} vs {})",
            if diff.abs() == 1 { "" } else { "s" },
            (b'A' + reference as u8) as char
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
    }
}
