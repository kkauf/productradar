//! Calendar dates without a date crate: civil <-> day-number conversion, ISO weeks, formatting.

use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Date {
    pub y: i32,
    pub m: u32,
    pub d: u32,
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

impl Date {
    /// Days since 1970-01-01 (Howard Hinnant's days_from_civil).
    pub fn days(self) -> i64 {
        let y = if self.m <= 2 { self.y - 1 } else { self.y } as i64;
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = self.m as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.d as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    pub fn from_days(z: i64) -> Date {
        let z = z + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = (yoe + era * 400) as i32 + if m <= 2 { 1 } else { 0 };
        Date { y, m, d }
    }

    pub fn valid(self) -> bool {
        (1..=12).contains(&self.m) && self.d >= 1 && Date::from_days(self.days()) == self
    }

    pub fn human(self) -> String {
        format!("{} {} {}", self.d, MONTHS[self.m as usize - 1], self.y)
    }

    pub fn short(self) -> String {
        format!("{} {}", self.d, MONTHS[self.m as usize - 1])
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let dt = toml::value::Datetime::deserialize(de)?;
        match (dt.date, dt.time) {
            (Some(d), None) => Ok(Date { y: d.year as i32, m: d.month as u32, d: d.day as u32 }),
            _ => Err(serde::de::Error::custom("expected a plain date like 2026-09-28")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_names() {
        let d = Date { y: 2026, m: 9, d: 28 };
        assert_eq!(Date::from_days(d.days()), d);
        assert_eq!(Date { y: 1970, m: 1, d: 1 }.days(), 0);
        assert!(!Date { y: 2026, m: 2, d: 30 }.valid());
        assert_eq!(d.human(), "28 Sep 2026");
    }
}
