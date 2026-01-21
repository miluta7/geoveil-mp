//! Time handling utilities for GNSS applications.
//!
//! Provides conversions between different time scales:
//! - GPS Time (GPST)
//! - UTC
//! - Galileo System Time (GST)
//! - BeiDou Time (BDT)
//! - GLONASS Time (GLONASST/UTC+3)

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::constants::{
    GPS_UTC_OFFSET_1980, JD_GPS_EPOCH, MJD_OFFSET, SECONDS_PER_DAY, SECONDS_PER_WEEK,
};
use super::error::{Error, Result};

/// GPS week and time of week representation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpsTime {
    /// GPS week number
    pub week: u32,
    /// Time of week in seconds
    pub tow: f64,
}

impl GpsTime {
    /// Create a new GPS time
    pub fn new(week: u32, tow: f64) -> Self {
        Self { week, tow }
    }

    /// Normalize the time (ensure TOW is within valid range)
    pub fn normalize(&mut self) {
        while self.tow < 0.0 {
            self.tow += SECONDS_PER_WEEK;
            self.week = self.week.saturating_sub(1);
        }
        while self.tow >= SECONDS_PER_WEEK {
            self.tow -= SECONDS_PER_WEEK;
            self.week += 1;
        }
    }

    /// Convert to total seconds since GPS epoch
    pub fn to_seconds(&self) -> f64 {
        (self.week as f64) * SECONDS_PER_WEEK + self.tow
    }

    /// Create from total seconds since GPS epoch
    pub fn from_seconds(seconds: f64) -> Self {
        let week = (seconds / SECONDS_PER_WEEK).floor() as u32;
        let tow = seconds - (week as f64) * SECONDS_PER_WEEK;
        Self { week, tow }
    }

    /// Convert to DateTime<Utc> (GPS time, not UTC)
    pub fn to_datetime(&self) -> DateTime<Utc> {
        // GPS epoch: January 6, 1980 00:00:00 UTC
        let gps_epoch = Utc.with_ymd_and_hms(1980, 1, 6, 0, 0, 0).unwrap();
        let total_seconds = self.to_seconds();
        let duration = Duration::milliseconds((total_seconds * 1000.0) as i64);
        gps_epoch + duration
    }

    /// Create from DateTime (assumes GPS time scale)
    pub fn from_datetime(dt: &DateTime<Utc>) -> Self {
        let gps_epoch = Utc.with_ymd_and_hms(1980, 1, 6, 0, 0, 0).unwrap();
        let duration = dt.signed_duration_since(gps_epoch);
        let total_seconds = duration.num_milliseconds() as f64 / 1000.0;
        Self::from_seconds(total_seconds)
    }

    /// Time difference in seconds
    pub fn diff(&self, other: &GpsTime) -> f64 {
        self.to_seconds() - other.to_seconds()
    }
}

impl fmt::Display for GpsTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GPS Week {} + {:.3}s", self.week, self.tow)
    }
}

/// GNSS epoch representation with high precision
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Epoch {
    /// Year (4-digit)
    pub year: i32,
    /// Month (1-12)
    pub month: u32,
    /// Day of month (1-31)
    pub day: u32,
    /// Hour (0-23)
    pub hour: u32,
    /// Minute (0-59)
    pub minute: u32,
    /// Second (0.0-59.999...) - stored as integer nanoseconds for Hash/Eq
    pub second: f64,
}

impl Default for Epoch {
    fn default() -> Self {
        Self {
            year: 2000,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0.0,
        }
    }
}

impl std::hash::Hash for Epoch {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.year.hash(state);
        self.month.hash(state);
        self.day.hash(state);
        self.hour.hash(state);
        self.minute.hash(state);
        // Convert seconds to nanoseconds for consistent hashing
        ((self.second * 1e9) as i64).hash(state);
    }
}

impl Eq for Epoch {}

impl Epoch {
    /// Create a new epoch
    pub fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: f64) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Parse epoch from RINEX format (various versions) or ISO 8601
    /// 
    /// RINEX 2: YY MM DD HH MM SS.SSSSSSS
    /// RINEX 3/4: YYYY MM DD HH MM SS.SSSSSSS
    /// ISO 8601: YYYY-MM-DDTHH:MM:SS.SSSZ or YYYY-MM-DD HH:MM:SS
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        
        // Try ISO 8601 format first (contains 'T' or '-' with ':')
        if s.contains('T') || (s.contains('-') && s.contains(':')) {
            return Self::parse_iso(s);
        }
        
        // Fall back to RINEX whitespace-separated format
        let parts: Vec<&str> = s.split_whitespace().collect();
        
        if parts.len() < 6 {
            return Err(Error::InvalidEpoch(format!(
                "Insufficient components in epoch string: '{}'",
                s
            )));
        }

        let year: i32 = parts[0].parse()?;
        let month: u32 = parts[1].parse()?;
        let day: u32 = parts[2].parse()?;
        let hour: u32 = parts[3].parse()?;
        let minute: u32 = parts[4].parse()?;
        let second: f64 = parts[5].parse()?;

        // Handle 2-digit years (RINEX 2)
        let year = if year < 80 {
            2000 + year
        } else if year < 100 {
            1900 + year
        } else {
            year
        };

        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }
    
    /// Parse ISO 8601 format: YYYY-MM-DDTHH:MM:SS.SSSZ or YYYY-MM-DD HH:MM:SS+00:00
    fn parse_iso(s: &str) -> Result<Self> {
        // Remove trailing Z or timezone
        let s = s.trim_end_matches('Z')
            .trim_end_matches("+00:00")
            .trim_end_matches("+00");
        
        // Split date and time
        let (date_part, time_part) = if s.contains('T') {
            let parts: Vec<&str> = s.splitn(2, 'T').collect();
            if parts.len() != 2 {
                return Err(Error::InvalidEpoch(format!("Invalid ISO epoch: '{}'", s)));
            }
            (parts[0], parts[1])
        } else if s.contains(' ') {
            let parts: Vec<&str> = s.splitn(2, ' ').collect();
            if parts.len() != 2 {
                return Err(Error::InvalidEpoch(format!("Invalid ISO epoch: '{}'", s)));
            }
            (parts[0], parts[1])
        } else {
            return Err(Error::InvalidEpoch(format!("Cannot parse epoch: '{}'", s)));
        };
        
        // Parse date: YYYY-MM-DD
        let date_parts: Vec<&str> = date_part.split('-').collect();
        if date_parts.len() != 3 {
            return Err(Error::InvalidEpoch(format!("Invalid date format: '{}'", date_part)));
        }
        
        let year: i32 = date_parts[0].parse()
            .map_err(|_| Error::InvalidEpoch(format!("Invalid year: '{}'", date_parts[0])))?;
        let month: u32 = date_parts[1].parse()
            .map_err(|_| Error::InvalidEpoch(format!("Invalid month: '{}'", date_parts[1])))?;
        let day: u32 = date_parts[2].parse()
            .map_err(|_| Error::InvalidEpoch(format!("Invalid day: '{}'", date_parts[2])))?;
        
        // Parse time: HH:MM:SS.SSS or HH:MM:SS
        let time_parts: Vec<&str> = time_part.split(':').collect();
        if time_parts.len() < 3 {
            return Err(Error::InvalidEpoch(format!("Invalid time format: '{}'", time_part)));
        }
        
        let hour: u32 = time_parts[0].parse()
            .map_err(|_| Error::InvalidEpoch(format!("Invalid hour: '{}'", time_parts[0])))?;
        let minute: u32 = time_parts[1].parse()
            .map_err(|_| Error::InvalidEpoch(format!("Invalid minute: '{}'", time_parts[1])))?;
        let second: f64 = time_parts[2].parse()
            .map_err(|_| Error::InvalidEpoch(format!("Invalid second: '{}'", time_parts[2])))?;
        
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    /// Convert to GPS time
    pub fn to_gps_time(&self) -> GpsTime {
        let dt = self.to_datetime();
        GpsTime::from_datetime(&dt)
    }

    /// Create from GPS time
    pub fn from_gps_time(gps: &GpsTime) -> Self {
        let dt = gps.to_datetime();
        Self::from_datetime(&dt)
    }

    /// Convert to DateTime<Utc>
    pub fn to_datetime(&self) -> DateTime<Utc> {
        let whole_seconds = self.second.floor() as u32;
        let nanos = ((self.second - self.second.floor()) * 1e9) as u32;
        
        Utc.with_ymd_and_hms(
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            whole_seconds,
        )
        .single()
        .unwrap_or_else(|| Utc::now())
        + Duration::nanoseconds(nanos as i64)
    }

    /// Create from DateTime
    pub fn from_datetime(dt: &DateTime<Utc>) -> Self {
        Self {
            year: dt.year(),
            month: dt.month(),
            day: dt.day(),
            hour: dt.hour(),
            minute: dt.minute(),
            second: dt.second() as f64 + dt.nanosecond() as f64 / 1e9,
        }
    }

    /// Convert to Modified Julian Date
    pub fn to_mjd(&self) -> f64 {
        let jd = self.to_julian_date();
        jd - MJD_OFFSET
    }

    /// Convert to Julian Date
    pub fn to_julian_date(&self) -> f64 {
        let y = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        };
        let m = if self.month <= 2 {
            self.month + 12
        } else {
            self.month
        };

        let a = (y / 100) as f64;
        let b = 2.0 - a + (a / 4.0).floor();

        let jd = (365.25 * (y + 4716) as f64).floor()
            + (30.6001 * (m + 1) as f64).floor()
            + self.day as f64
            + b
            - 1524.5;

        let day_fraction =
            (self.hour as f64 + self.minute as f64 / 60.0 + self.second / 3600.0) / 24.0;

        jd + day_fraction
    }

    /// Convert to day of year (DOY)
    pub fn day_of_year(&self) -> u32 {
        let date = NaiveDate::from_ymd_opt(self.year, self.month, self.day)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
        date.ordinal()
    }

    /// Convert to seconds of day
    pub fn seconds_of_day(&self) -> f64 {
        self.hour as f64 * 3600.0 + self.minute as f64 * 60.0 + self.second
    }

    /// Time difference in seconds
    pub fn diff(&self, other: &Epoch) -> f64 {
        let jd1 = self.to_julian_date();
        let jd2 = other.to_julian_date();
        (jd1 - jd2) * SECONDS_PER_DAY
    }

    /// Add seconds to epoch
    pub fn add_seconds(&self, seconds: f64) -> Self {
        let dt = self.to_datetime();
        let new_dt = dt + Duration::milliseconds((seconds * 1000.0) as i64);
        Self::from_datetime(&new_dt)
    }

    /// Format as RINEX 3/4 style
    pub fn to_rinex_string(&self) -> String {
        format!(
            "{:4} {:02} {:02} {:02} {:02} {:11.7}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// Format as ISO 8601
    pub fn to_iso_string(&self) -> String {
        let dt = self.to_datetime();
        dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }
}

impl fmt::Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:09.6}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

impl PartialOrd for Epoch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let diff = self.diff(other);
        if diff.abs() < 1e-9 {
            Some(std::cmp::Ordering::Equal)
        } else if diff > 0.0 {
            Some(std::cmp::Ordering::Greater)
        } else {
            Some(std::cmp::Ordering::Less)
        }
    }
}

/// Leap seconds table (GPS Time - UTC)
/// Updated through 2024
const LEAP_SECONDS: &[(i32, u32, u32, i32)] = &[
    // (year, month, day, cumulative_leap_seconds)
    (1981, 7, 1, 1),
    (1982, 7, 1, 2),
    (1983, 7, 1, 3),
    (1985, 7, 1, 4),
    (1988, 1, 1, 5),
    (1990, 1, 1, 6),
    (1991, 1, 1, 7),
    (1992, 7, 1, 8),
    (1993, 7, 1, 9),
    (1994, 7, 1, 10),
    (1996, 1, 1, 11),
    (1997, 7, 1, 12),
    (1999, 1, 1, 13),
    (2006, 1, 1, 14),
    (2009, 1, 1, 15),
    (2012, 7, 1, 16),
    (2015, 7, 1, 17),
    (2017, 1, 1, 18),
];

/// Get leap seconds at a given epoch (GPS Time - UTC)
pub fn get_leap_seconds(epoch: &Epoch) -> i32 {
    for &(year, month, day, leap) in LEAP_SECONDS.iter().rev() {
        if epoch.year > year
            || (epoch.year == year && epoch.month > month)
            || (epoch.year == year && epoch.month == month && epoch.day >= day)
        {
            return leap;
        }
    }
    0
}

/// Convert GPS time to UTC
pub fn gps_to_utc(epoch: &Epoch) -> Epoch {
    let leap = get_leap_seconds(epoch);
    epoch.add_seconds(-(leap as f64))
}

/// Convert UTC to GPS time
pub fn utc_to_gps(epoch: &Epoch) -> Epoch {
    let leap = get_leap_seconds(epoch);
    epoch.add_seconds(leap as f64)
}

/// Convert GLONASS time (UTC+3, no leap seconds) to GPS time
pub fn glonass_to_gps(epoch: &Epoch) -> Epoch {
    // First convert from UTC+3 to UTC
    let utc = epoch.add_seconds(-3.0 * 3600.0);
    // Then convert from UTC to GPS
    utc_to_gps(&utc)
}

/// Convert GPS time to GLONASS time
pub fn gps_to_glonass(epoch: &Epoch) -> Epoch {
    // First convert from GPS to UTC
    let utc = gps_to_utc(epoch);
    // Then convert from UTC to UTC+3
    utc.add_seconds(3.0 * 3600.0)
}

/// Convert BeiDou time to GPS time
/// BDT = GPS - 14 seconds (BDT epoch: January 1, 2006 00:00:00 UTC)
pub fn bdt_to_gps(epoch: &Epoch) -> Epoch {
    epoch.add_seconds(14.0)
}

/// Convert GPS time to BeiDou time
pub fn gps_to_bdt(epoch: &Epoch) -> Epoch {
    epoch.add_seconds(-14.0)
}

/// Convert Galileo time to GPS time
/// GST and GPS are aligned (both count from their respective epochs without leap seconds)
/// The difference is a constant offset based on epoch difference
pub fn gst_to_gps(epoch: &Epoch) -> Epoch {
    // GST epoch: August 22, 1999 00:00:00
    // GST is aligned with GPS time (no additional offset needed for practical purposes)
    *epoch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_parsing() {
        let s = "2024 01 15 12 30 45.1234567";
        let epoch = Epoch::parse(s).unwrap();
        assert_eq!(epoch.year, 2024);
        assert_eq!(epoch.month, 1);
        assert_eq!(epoch.day, 15);
        assert_eq!(epoch.hour, 12);
        assert_eq!(epoch.minute, 30);
        assert!((epoch.second - 45.1234567).abs() < 1e-6);
    }

    #[test]
    fn test_two_digit_year() {
        let s1 = "24 01 15 12 30 45.0";
        let epoch1 = Epoch::parse(s1).unwrap();
        assert_eq!(epoch1.year, 2024);

        let s2 = "99 01 15 12 30 45.0";
        let epoch2 = Epoch::parse(s2).unwrap();
        assert_eq!(epoch2.year, 1999);
    }

    #[test]
    fn test_gps_time_conversion() {
        let epoch = Epoch::new(2024, 1, 15, 0, 0, 0.0);
        let gps = epoch.to_gps_time();
        let back = Epoch::from_gps_time(&gps);
        
        assert!((epoch.diff(&back)).abs() < 0.001);
    }

    #[test]
    fn test_leap_seconds() {
        let epoch_2020 = Epoch::new(2020, 1, 1, 0, 0, 0.0);
        assert_eq!(get_leap_seconds(&epoch_2020), 18);

        let epoch_2000 = Epoch::new(2000, 1, 1, 0, 0, 0.0);
        assert_eq!(get_leap_seconds(&epoch_2000), 13);
    }

    #[test]
    fn test_day_of_year() {
        let epoch = Epoch::new(2024, 1, 15, 0, 0, 0.0);
        assert_eq!(epoch.day_of_year(), 15);

        let epoch_dec = Epoch::new(2024, 12, 31, 0, 0, 0.0);
        assert_eq!(epoch_dec.day_of_year(), 366); // 2024 is a leap year
    }

    #[test]
    fn test_julian_date() {
        // J2000.0: January 1, 2000, 12:00:00 TT = JD 2451545.0
        let j2000 = Epoch::new(2000, 1, 1, 12, 0, 0.0);
        let jd = j2000.to_julian_date();
        assert!((jd - 2451545.0).abs() < 0.001);
    }
    
    #[test]
    fn test_iso_parsing() {
        // ISO 8601 with T separator and Z suffix
        let s1 = "2025-12-21T12:00:00.000Z";
        let epoch1 = Epoch::parse(s1).unwrap();
        assert_eq!(epoch1.year, 2025);
        assert_eq!(epoch1.month, 12);
        assert_eq!(epoch1.day, 21);
        assert_eq!(epoch1.hour, 12);
        assert_eq!(epoch1.minute, 0);
        assert!((epoch1.second - 0.0).abs() < 0.001);
        
        // ISO 8601 with +00:00 timezone
        let s2 = "2025-12-21 08:30:45+00:00";
        let epoch2 = Epoch::parse(s2).unwrap();
        assert_eq!(epoch2.year, 2025);
        assert_eq!(epoch2.month, 12);
        assert_eq!(epoch2.day, 21);
        assert_eq!(epoch2.hour, 8);
        assert_eq!(epoch2.minute, 30);
        assert!((epoch2.second - 45.0).abs() < 0.001);
        
        // ISO 8601 with T separator no timezone
        let s3 = "2025-12-21T23:59:59.999";
        let epoch3 = Epoch::parse(s3).unwrap();
        assert_eq!(epoch3.year, 2025);
        assert_eq!(epoch3.hour, 23);
        assert_eq!(epoch3.minute, 59);
        assert!((epoch3.second - 59.999).abs() < 0.001);
    }
}
