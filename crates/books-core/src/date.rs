//! Calendar days in Asia/Manila (UTC+08:00 all year; no daylight saving)
//! and period keys: `2026-Q3`, `2026-09`, `2026`. Due dates are Forms'
//! (ADR 0007), so nothing here knows one.

use std::fmt;

/// A calendar day. Ordering is calendar order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

pub const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap(year) => 29,
        _ => 28,
    }
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        ((1900..=9999).contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month))
        .then_some(Self { year, month, day })
    }

    /// `YYYY-MM-DD`, a real calendar day.
    pub fn parse(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let num = |s: &str| -> Option<u32> {
            s.bytes()
                .all(|c| c.is_ascii_digit())
                .then(|| s.parse().ok())?
        };
        let year = i32::try_from(num(&text[0..4])?).ok()?;
        let month = u8::try_from(num(&text[5..7])?).ok()?;
        let day = u8::try_from(num(&text[8..10])?).ok()?;
        Self::new(year, month, day)
    }

    pub fn month_start(self) -> Self {
        Self { day: 1, ..self }
    }

    pub fn month_end(self) -> Self {
        Self {
            day: days_in_month(self.year, self.month),
            ..self
        }
    }

    /// Days since 1970-01-01 (Howard Hinnant's civil algorithm).
    pub fn to_days(self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    pub fn from_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        Self {
            year: i32::try_from(year).unwrap_or(9999),
            month: u8::try_from(month).unwrap_or(1),
            day: u8::try_from(day).unwrap_or(1),
        }
    }

    pub fn add_days(self, days: i64) -> Self {
        Self::from_days(self.to_days() + days)
    }

    /// 0 is Monday.
    pub fn weekday(self) -> u8 {
        u8::try_from((self.to_days() + 3).rem_euclid(7)).unwrap_or(0)
    }

    /// The calendar day in Manila at a unix time in milliseconds.
    pub fn manila_from_unix_ms(ms: i64) -> Self {
        Self::from_days((ms.div_euclid(1000) + 8 * 3600).div_euclid(86_400))
    }

    /// Today in Manila. `BOOKS_TODAY` (a `YYYY-MM-DD`) pins it for tests
    /// and the demo book.
    pub fn today() -> Self {
        if let Some(pinned) = std::env::var("BOOKS_TODAY")
            .ok()
            .and_then(|t| Self::parse(&t))
        {
            return pinned;
        }
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or(0);
        Self::manila_from_unix_ms(ms)
    }

    /// `Oct 28`.
    pub fn short(self) -> String {
        format!("{} {:02}", MONTHS[usize::from(self.month - 1)], self.day)
    }

    pub fn quarter(self) -> u8 {
        (self.month - 1) / 3 + 1
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodKind {
    Month,
    Quarter,
    Year,
}

/// A period key and its calendar range, both ends included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    pub key: String,
    pub kind: PeriodKind,
    pub start: Date,
    pub end: Date,
}

impl Period {
    pub fn parse(key: &str) -> Option<Self> {
        let year = |s: &str| -> Option<i32> {
            (s.len() == 4 && s.bytes().all(|c| c.is_ascii_digit()))
                .then(|| s.parse().ok())
                .flatten()
        };
        let (kind, start, end) = if let Some((y, q)) = key.split_once("-Q") {
            let y = year(y)?;
            let q: u8 = match q {
                "1" => 1,
                "2" => 2,
                "3" => 3,
                "4" => 4,
                _ => return None,
            };
            let first = (q - 1) * 3 + 1;
            (
                PeriodKind::Quarter,
                Date::new(y, first, 1)?,
                Date::new(y, first + 2, 1)?.month_end(),
            )
        } else if let Some((y, m)) = key.split_once('-') {
            let y = year(y)?;
            if m.len() != 2 || !m.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let m: u8 = m.parse().ok()?;
            let start = Date::new(y, m, 1)?;
            (PeriodKind::Month, start, start.month_end())
        } else {
            let y = year(key)?;
            (PeriodKind::Year, Date::new(y, 1, 1)?, Date::new(y, 12, 31)?)
        };
        Some(Self {
            key: key.to_string(),
            kind,
            start,
            end,
        })
    }

    pub fn of(date: Date, kind: PeriodKind) -> Self {
        let key = match kind {
            PeriodKind::Month => format!("{:04}-{:02}", date.year, date.month),
            PeriodKind::Quarter => format!("{:04}-Q{}", date.year, date.quarter()),
            PeriodKind::Year => format!("{:04}", date.year),
        };
        Self::parse(&key).expect("a key built from a real date parses")
    }

    pub fn contains(&self, date: Date) -> bool {
        self.start <= date && date <= self.end
    }

    /// The period just before this one, of the same kind.
    pub fn previous(&self) -> Self {
        Self::of(self.start.add_days(-1), self.kind)
    }

    /// `October 2026`, `Q3 2026`, `2026`.
    pub fn label(&self) -> String {
        match self.kind {
            PeriodKind::Month => format!(
                "{} {}",
                MONTH_NAMES[usize::from(self.start.month - 1)],
                self.start.year
            ),
            PeriodKind::Quarter => format!("Q{} {}", self.start.quarter(), self.start.year),
            PeriodKind::Year => self.start.year.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_parse_only_real_days() {
        assert_eq!(Date::parse("2026-10-29"), Date::new(2026, 10, 29));
        assert!(Date::parse("2026-02-29").is_none());
        assert!(Date::parse("2028-02-29").is_some());
        for bad in [
            "2026-13-01",
            "2026-00-10",
            "2026-1-01",
            "26-01-01",
            "2026/01/01",
            "",
        ] {
            assert!(Date::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn day_numbers_round_trip() {
        let d = Date::new(2026, 10, 29).unwrap();
        assert_eq!(Date::from_days(d.to_days()), d);
        assert_eq!(Date::new(1970, 1, 1).unwrap().to_days(), 0);
        assert_eq!(d.add_days(3).to_string(), "2026-11-01");
        assert_eq!(d.weekday(), 3, "Thursday");
    }

    #[test]
    fn manila_days_follow_utc_plus_eight() {
        // 2026-09-30T20:00:00Z is 04:00 on 1 Oct in Manila
        let ms = Date::new(2026, 9, 30).unwrap().to_days() * 86_400_000 + 20 * 3_600_000;
        assert_eq!(Date::manila_from_unix_ms(ms).to_string(), "2026-10-01");
        assert_eq!(
            Period::of(Date::manila_from_unix_ms(ms), PeriodKind::Quarter).key,
            "2026-Q4"
        );
    }

    #[test]
    fn periods_resolve_to_ranges() {
        let q3 = Period::parse("2026-Q3").unwrap();
        assert_eq!(
            (q3.start.to_string(), q3.end.to_string()),
            ("2026-07-01".into(), "2026-09-30".into())
        );
        let sep = Period::parse("2026-09").unwrap();
        assert_eq!(sep.end.to_string(), "2026-09-30");
        assert_eq!(Period::parse("2026").unwrap().end.to_string(), "2026-12-31");
        assert!(Period::parse("2026-Q5").is_none());
        assert!(Period::parse("2026-13").is_none());
        assert_eq!(sep.previous().key, "2026-08");
        assert_eq!(Period::parse("2026-01").unwrap().previous().key, "2025-12");
        assert_eq!(Period::parse("2026-10").unwrap().label(), "October 2026");
    }
}
