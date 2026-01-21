//! Physical constants and GNSS system parameters.

use std::f64::consts::PI;

/// Speed of light in vacuum (m/s)
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;

/// Gravitational constant × Earth mass (m³/s²) - WGS84
pub const GM_WGS84: f64 = 3.986005e14;

/// Gravitational constant × Earth mass (m³/s²) - PZ-90 (GLONASS)
pub const GM_PZ90: f64 = 3.9860044e14;

/// Earth's angular velocity (rad/s) - WGS84
pub const OMEGA_EARTH_WGS84: f64 = 7.2921151467e-5;

/// Earth's angular velocity (rad/s) - PZ-90
pub const OMEGA_EARTH_PZ90: f64 = 7.292115e-5;

/// Earth's semi-major axis (m) - WGS84
pub const EARTH_RADIUS_WGS84: f64 = 6_378_137.0;

/// Earth's semi-major axis (m) - PZ-90
pub const EARTH_RADIUS_PZ90: f64 = 6_378_136.0;

/// Earth's flattening - WGS84
pub const EARTH_FLATTENING_WGS84: f64 = 1.0 / 298.257223563;

/// Earth's second zonal harmonic (J2) - WGS84
pub const J2_WGS84: f64 = 1.082627e-3;

/// Earth's second zonal harmonic (J2) - PZ-90
pub const J2_PZ90: f64 = 1.0826257e-3;

/// GPS week rollover period (weeks)
pub const GPS_WEEK_ROLLOVER: u32 = 1024;

/// Seconds per GPS week
pub const SECONDS_PER_WEEK: f64 = 604_800.0;

/// Seconds per day
pub const SECONDS_PER_DAY: f64 = 86_400.0;

/// GPS-UTC time offset at GPS epoch (seconds)
pub const GPS_UTC_OFFSET_1980: f64 = 315_964_800.0;

/// Modified Julian Date offset
pub const MJD_OFFSET: f64 = 2_400_000.5;

/// Julian Date at GPS epoch (January 6, 1980 00:00:00 UTC)
pub const JD_GPS_EPOCH: f64 = 2_444_244.5;

/// Conversion from degrees to radians
pub const DEG_TO_RAD: f64 = PI / 180.0;

/// Conversion from radians to degrees
pub const RAD_TO_DEG: f64 = 180.0 / PI;

/// GNSS System carrier frequencies
pub mod frequencies {
    /// GPS frequencies (Hz)
    pub mod gps {
        pub const L1: f64 = 1_575_420_000.0;
        pub const L2: f64 = 1_227_600_000.0;
        pub const L5: f64 = 1_176_450_000.0;
        
        /// L1 wavelength (m)
        pub const L1_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / L1;
        /// L2 wavelength (m)
        pub const L2_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / L2;
        /// L5 wavelength (m)
        pub const L5_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / L5;
        
        /// Alpha factor for L1/L2
        pub const ALPHA_L1_L2: f64 = (L1 * L1) / (L2 * L2);
        /// Alpha factor for L1/L5
        pub const ALPHA_L1_L5: f64 = (L1 * L1) / (L5 * L5);
    }

    /// GLONASS frequencies (Hz)
    pub mod glonass {
        /// G1 base frequency
        pub const G1_BASE: f64 = 1_602_000_000.0;
        /// G1 frequency step per channel
        pub const G1_STEP: f64 = 562_500.0;
        /// G2 base frequency
        pub const G2_BASE: f64 = 1_246_000_000.0;
        /// G2 frequency step per channel
        pub const G2_STEP: f64 = 437_500.0;
        /// G3 (CDMA L3)
        pub const G3: f64 = 1_202_025_000.0;
        
        /// Get G1 frequency for a given channel number (-7 to +6)
        pub fn g1_frequency(channel: i8) -> f64 {
            G1_BASE + (channel as f64) * G1_STEP
        }
        
        /// Get G2 frequency for a given channel number (-7 to +6)
        pub fn g2_frequency(channel: i8) -> f64 {
            G2_BASE + (channel as f64) * G2_STEP
        }
    }

    /// Galileo frequencies (Hz)
    pub mod galileo {
        pub const E1: f64 = 1_575_420_000.0;  // Same as GPS L1
        pub const E5A: f64 = 1_176_450_000.0; // Same as GPS L5
        pub const E5B: f64 = 1_207_140_000.0;
        pub const E5: f64 = 1_191_795_000.0;  // E5a+E5b
        pub const E6: f64 = 1_278_750_000.0;
        
        pub const E1_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / E1;
        pub const E5A_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / E5A;
        pub const E5B_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / E5B;
        pub const E6_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / E6;
    }

    /// BeiDou frequencies (Hz)
    pub mod beidou {
        pub const B1I: f64 = 1_561_098_000.0;
        pub const B1C: f64 = 1_575_420_000.0; // Same as GPS L1
        pub const B2I: f64 = 1_207_140_000.0;
        pub const B2A: f64 = 1_176_450_000.0; // Same as GPS L5
        pub const B2B: f64 = 1_207_140_000.0;
        pub const B3I: f64 = 1_268_520_000.0;
        
        pub const B1I_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / B1I;
        pub const B1C_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / B1C;
        pub const B2I_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / B2I;
        pub const B3I_WAVELENGTH: f64 = super::super::SPEED_OF_LIGHT / B3I;
    }

    /// QZSS frequencies (Hz)
    pub mod qzss {
        pub const L1: f64 = 1_575_420_000.0;
        pub const L2: f64 = 1_227_600_000.0;
        pub const L5: f64 = 1_176_450_000.0;
        pub const L6: f64 = 1_278_750_000.0;
    }

    /// NavIC/IRNSS frequencies (Hz)
    pub mod navic {
        pub const L5: f64 = 1_176_450_000.0;
        pub const S: f64 = 2_492_028_000.0;
        pub const L1: f64 = 1_575_420_000.0; // Added in newer satellites
    }

    /// SBAS frequencies (Hz)
    pub mod sbas {
        pub const L1: f64 = 1_575_420_000.0;
        pub const L5: f64 = 1_176_450_000.0;
    }
}

/// Maximum number of satellites per system
pub mod max_satellites {
    pub const GPS: usize = 32;
    pub const GLONASS: usize = 27;
    pub const GALILEO: usize = 36;
    pub const BEIDOU: usize = 63;
    pub const QZSS: usize = 10;
    pub const SBAS: usize = 39;
    pub const NAVIC: usize = 14;
}

/// System identifiers (RINEX convention)
pub mod system_identifiers {
    pub const GPS: char = 'G';
    pub const GLONASS: char = 'R';
    pub const GALILEO: char = 'E';
    pub const BEIDOU: char = 'C';
    pub const QZSS: char = 'J';
    pub const SBAS: char = 'S';
    pub const NAVIC: char = 'I';
    pub const MIXED: char = 'M';
}

/// Default threshold values for quality control
pub mod thresholds {
    /// Default elevation cutoff angle (degrees)
    pub const ELEVATION_CUTOFF: f64 = 10.0;
    
    /// Ionospheric rate threshold for cycle slip detection (m/s)
    pub const ION_RATE_THRESHOLD: f64 = 0.0667;
    
    /// Code-phase threshold for cycle slip detection (m/s)
    pub const CODE_PHASE_THRESHOLD: f64 = 6.667;
    
    /// Position convergence threshold (m)
    pub const POSITION_CONVERGENCE: f64 = 1e-8;
    
    /// Maximum iterations for position estimation
    pub const MAX_ITERATIONS: usize = 20;
    
    /// Kepler equation convergence threshold
    pub const KEPLER_CONVERGENCE: f64 = 1e-12;
    
    /// SNR threshold for good quality (dB-Hz)
    pub const SNR_GOOD: f64 = 35.0;
    
    /// SNR threshold for acceptable quality (dB-Hz)
    pub const SNR_ACCEPTABLE: f64 = 25.0;
}

/// Time system identifiers
pub mod time_systems {
    pub const GPS: &str = "GPS";
    pub const GLONASS: &str = "GLO";
    pub const GALILEO: &str = "GAL";
    pub const BEIDOU: &str = "BDT";
    pub const QZSS: &str = "QZS";
    pub const NAVIC: &str = "IRN";
    pub const UTC: &str = "UTC";
    pub const TAI: &str = "TAI";
}

/// Calculate alpha factor (frequency ratio squared) for multipath computation
pub fn alpha_factor(f1: f64, f2: f64) -> f64 {
    (f1 * f1) / (f2 * f2)
}

/// Calculate wavelength from frequency
pub fn wavelength(frequency: f64) -> f64 {
    SPEED_OF_LIGHT / frequency
}

/// Get frequency for a given system and band
pub fn get_frequency(system: char, band: u8, glonass_channel: Option<i8>) -> Option<f64> {
    use frequencies::*;
    
    match (system, band) {
        ('G', 1) => Some(gps::L1),
        ('G', 2) => Some(gps::L2),
        ('G', 5) => Some(gps::L5),
        ('R', 1) => Some(glonass::g1_frequency(glonass_channel.unwrap_or(0))),
        ('R', 2) => Some(glonass::g2_frequency(glonass_channel.unwrap_or(0))),
        ('R', 3) => Some(glonass::G3),
        ('E', 1) => Some(galileo::E1),
        ('E', 5) => Some(galileo::E5A),
        ('E', 6) => Some(galileo::E6),
        ('E', 7) => Some(galileo::E5B),
        ('E', 8) => Some(galileo::E5),
        ('C', 1) => Some(beidou::B1C),
        ('C', 2) => Some(beidou::B1I),
        ('C', 5) => Some(beidou::B2A),
        ('C', 6) => Some(beidou::B3I),
        ('C', 7) => Some(beidou::B2I),
        ('J', 1) => Some(qzss::L1),
        ('J', 2) => Some(qzss::L2),
        ('J', 5) => Some(qzss::L5),
        ('J', 6) => Some(qzss::L6),
        ('I', 5) => Some(navic::L5),
        ('I', 9) => Some(navic::S),
        ('S', 1) => Some(sbas::L1),
        ('S', 5) => Some(sbas::L5),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_gps_l1_wavelength() {
        let expected = SPEED_OF_LIGHT / frequencies::gps::L1;
        assert_relative_eq!(frequencies::gps::L1_WAVELENGTH, expected, epsilon = 1e-10);
        assert_relative_eq!(frequencies::gps::L1_WAVELENGTH, 0.190293672798, epsilon = 1e-6);
    }

    #[test]
    fn test_alpha_factor() {
        let alpha = alpha_factor(frequencies::gps::L1, frequencies::gps::L2);
        assert_relative_eq!(alpha, frequencies::gps::ALPHA_L1_L2, epsilon = 1e-10);
    }

    #[test]
    fn test_glonass_frequencies() {
        // Channel 0
        assert_relative_eq!(
            frequencies::glonass::g1_frequency(0),
            frequencies::glonass::G1_BASE,
            epsilon = 1e-6
        );
        
        // Channel +1
        assert_relative_eq!(
            frequencies::glonass::g1_frequency(1),
            frequencies::glonass::G1_BASE + frequencies::glonass::G1_STEP,
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_get_frequency() {
        assert_eq!(get_frequency('G', 1, None), Some(frequencies::gps::L1));
        assert_eq!(get_frequency('E', 1, None), Some(frequencies::galileo::E1));
        assert_eq!(get_frequency('X', 1, None), None);
    }
}
