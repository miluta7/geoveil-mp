//! Utility modules for the GNSS multipath library.

pub mod constants;
mod coordinates;
mod error;
mod time;

pub use constants::*;
pub use coordinates::*;
pub use error::{Error, Result, ResultExt};
pub use time::*;

/// Helper trait for numeric operations
pub trait NumericExt {
    /// Round to specified decimal places
    fn round_to(self, decimals: u32) -> Self;
}

impl NumericExt for f64 {
    fn round_to(self, decimals: u32) -> Self {
        let multiplier = 10_f64.powi(decimals as i32);
        (self * multiplier).round() / multiplier
    }
}

/// Check if a string represents a valid satellite identifier
pub fn is_valid_satellite_id(s: &str) -> bool {
    if s.len() < 2 || s.len() > 3 {
        return false;
    }
    
    let system = s.chars().next().unwrap();
    let prn: Option<u32> = s[1..].parse().ok();
    
    match (system, prn) {
        ('G', Some(n)) => n >= 1 && n <= 32,  // GPS
        ('R', Some(n)) => n >= 1 && n <= 27,  // GLONASS
        ('E', Some(n)) => n >= 1 && n <= 36,  // Galileo
        ('C', Some(n)) => n >= 1 && n <= 63,  // BeiDou
        ('J', Some(n)) => n >= 1 && n <= 10,  // QZSS
        ('I', Some(n)) => n >= 1 && n <= 14,  // NavIC/IRNSS
        ('S', Some(n)) => n >= 100 && n <= 199, // SBAS
        _ => false,
    }
}

/// Parse satellite identifier into system and PRN
pub fn parse_satellite_id(s: &str) -> Option<(char, u32)> {
    if s.len() < 2 {
        return None;
    }
    
    let system = s.chars().next()?;
    let prn: u32 = s[1..].trim().parse().ok()?;
    
    Some((system, prn))
}

/// Format satellite identifier
pub fn format_satellite_id(system: char, prn: u32) -> String {
    format!("{}{:02}", system, prn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_to() {
        assert_eq!(3.14159_f64.round_to(2), 3.14);
        assert_eq!(3.14159_f64.round_to(4), 3.1416);
    }

    #[test]
    fn test_satellite_id_validation() {
        assert!(is_valid_satellite_id("G01"));
        assert!(is_valid_satellite_id("G32"));
        assert!(is_valid_satellite_id("E36"));
        assert!(is_valid_satellite_id("C63"));
        assert!(!is_valid_satellite_id("G33")); // GPS max is 32
        assert!(!is_valid_satellite_id("X01")); // Invalid system
        assert!(!is_valid_satellite_id("")); // Empty
    }

    #[test]
    fn test_parse_satellite_id() {
        assert_eq!(parse_satellite_id("G15"), Some(('G', 15)));
        assert_eq!(parse_satellite_id("E01"), Some(('E', 1)));
        assert_eq!(parse_satellite_id("R 5"), Some(('R', 5)));
        assert_eq!(parse_satellite_id(""), None);
    }

    #[test]
    fn test_format_satellite_id() {
        assert_eq!(format_satellite_id('G', 1), "G01");
        assert_eq!(format_satellite_id('E', 15), "E15");
    }
}
