//! RINEX data types and structures.
//!
//! Defines the core data structures for representing RINEX observation
//! and navigation data, supporting versions 2.xx, 3.xx, and 4.xx.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::utils::{Ecef, Epoch};

/// RINEX file version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RinexVersion {
    /// RINEX 2.xx
    V2(u8),
    /// RINEX 3.xx
    V3(u8),
    /// RINEX 4.xx
    V4(u8),
}

impl RinexVersion {
    /// Parse version from string (e.g., "3.05", "4.02")
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() < 2 {
            return None;
        }
        
        let major: u8 = parts[0].parse().ok()?;
        let minor: u8 = parts[1].chars().take(2).collect::<String>().parse().ok()?;
        
        match major {
            2 => Some(RinexVersion::V2(minor)),
            3 => Some(RinexVersion::V3(minor)),
            4 => Some(RinexVersion::V4(minor)),
            _ => None,
        }
    }

    /// Check if version is 2.xx
    pub fn is_v2(&self) -> bool {
        matches!(self, RinexVersion::V2(_))
    }

    /// Check if version is 3.xx
    pub fn is_v3(&self) -> bool {
        matches!(self, RinexVersion::V3(_))
    }

    /// Check if version is 4.xx
    pub fn is_v4(&self) -> bool {
        matches!(self, RinexVersion::V4(_))
    }

    /// Get major version number
    pub fn major(&self) -> u8 {
        match self {
            RinexVersion::V2(_) => 2,
            RinexVersion::V3(_) => 3,
            RinexVersion::V4(_) => 4,
        }
    }

    /// Get minor version number
    pub fn minor(&self) -> u8 {
        match self {
            RinexVersion::V2(m) | RinexVersion::V3(m) | RinexVersion::V4(m) => *m,
        }
    }
}

impl fmt::Display for RinexVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:02}", self.major(), self.minor())
    }
}

impl Default for RinexVersion {
    fn default() -> Self {
        RinexVersion::V3(5)
    }
}

/// GNSS system identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GnssSystem {
    Gps,
    Glonass,
    Galileo,
    Beidou,
    Qzss,
    Sbas,
    Navic,
    Mixed,
}

impl GnssSystem {
    /// Create from single character identifier
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'G' => Some(GnssSystem::Gps),
            'R' => Some(GnssSystem::Glonass),
            'E' => Some(GnssSystem::Galileo),
            'C' => Some(GnssSystem::Beidou),
            'J' => Some(GnssSystem::Qzss),
            'S' => Some(GnssSystem::Sbas),
            'I' => Some(GnssSystem::Navic),
            'M' => Some(GnssSystem::Mixed),
            _ => None,
        }
    }

    /// Convert to single character identifier
    pub fn to_char(&self) -> char {
        match self {
            GnssSystem::Gps => 'G',
            GnssSystem::Glonass => 'R',
            GnssSystem::Galileo => 'E',
            GnssSystem::Beidou => 'C',
            GnssSystem::Qzss => 'J',
            GnssSystem::Sbas => 'S',
            GnssSystem::Navic => 'I',
            GnssSystem::Mixed => 'M',
        }
    }

    /// Get full name
    pub fn name(&self) -> &'static str {
        match self {
            GnssSystem::Gps => "GPS",
            GnssSystem::Glonass => "GLONASS",
            GnssSystem::Galileo => "Galileo",
            GnssSystem::Beidou => "BeiDou",
            GnssSystem::Qzss => "QZSS",
            GnssSystem::Sbas => "SBAS",
            GnssSystem::Navic => "NavIC",
            GnssSystem::Mixed => "Mixed",
        }
    }

    /// Get all concrete systems (excluding Mixed)
    pub fn all() -> &'static [GnssSystem] {
        &[
            GnssSystem::Gps,
            GnssSystem::Glonass,
            GnssSystem::Galileo,
            GnssSystem::Beidou,
            GnssSystem::Qzss,
            GnssSystem::Sbas,
            GnssSystem::Navic,
        ]
    }
}

impl fmt::Display for GnssSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Satellite identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Satellite {
    pub system: GnssSystem,
    pub prn: u32,
}

impl Satellite {
    pub fn new(system: GnssSystem, prn: u32) -> Self {
        Self { system, prn }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.len() < 2 {
            return None;
        }
        let system = GnssSystem::from_char(s.chars().next()?)?;
        let prn: u32 = s[1..].trim().parse().ok()?;
        Some(Self { system, prn })
    }
}

impl fmt::Display for Satellite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:02}", self.system.to_char(), self.prn)
    }
}

/// Observation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObservationType {
    Code,
    Phase,
    Doppler,
    Snr,
    Channel,
}

impl ObservationType {
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'C' => Some(ObservationType::Code),
            'L' => Some(ObservationType::Phase),
            'D' => Some(ObservationType::Doppler),
            'S' => Some(ObservationType::Snr),
            'X' => Some(ObservationType::Channel),
            _ => None,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            ObservationType::Code => 'C',
            ObservationType::Phase => 'L',
            ObservationType::Doppler => 'D',
            ObservationType::Snr => 'S',
            ObservationType::Channel => 'X',
        }
    }
}

/// Signal code (3-character)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignalCode {
    pub obs_type: ObservationType,
    pub band: u8,
    pub attribute: char,
}

impl SignalCode {
    pub fn new(obs_type: ObservationType, band: u8, attribute: char) -> Self {
        Self { obs_type, band, attribute }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.len() != 3 {
            return None;
        }
        let chars: Vec<char> = s.chars().collect();
        let obs_type = ObservationType::from_char(chars[0])?;
        let band: u8 = chars[1].to_digit(10)? as u8;
        let attribute = chars[2].to_ascii_uppercase();
        Some(Self { obs_type, band, attribute })
    }

    pub fn is_code(&self) -> bool {
        self.obs_type == ObservationType::Code
    }

    pub fn is_phase(&self) -> bool {
        self.obs_type == ObservationType::Phase
    }

    pub fn is_snr(&self) -> bool {
        self.obs_type == ObservationType::Snr
    }
}

impl fmt::Display for SignalCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.obs_type.to_char(), self.band, self.attribute)
    }
}

/// Single observation value with flags
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ObservationValue {
    pub value: f64,
    pub lli: Option<u8>,
    pub ssi: Option<u8>,
}

impl ObservationValue {
    pub fn new(value: f64) -> Self {
        Self { value, lli: None, ssi: None }
    }

    pub fn with_flags(value: f64, lli: Option<u8>, ssi: Option<u8>) -> Self {
        Self { value, lli, ssi }
    }

    pub fn has_loss_of_lock(&self) -> bool {
        self.lli.map(|l| l & 0x01 != 0).unwrap_or(false)
    }

    pub fn has_cycle_slip(&self) -> bool {
        self.lli.map(|l| l & 0x02 != 0).unwrap_or(false)
    }
}

/// Observations for a single satellite at a single epoch
pub type SatelliteObservations = HashMap<SignalCode, ObservationValue>;

/// All observations for a single epoch
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpochObservations {
    pub epoch: Epoch,
    pub flag: u8,
    pub clock_offset: Option<f64>,
    pub satellites: HashMap<Satellite, SatelliteObservations>,
}

impl EpochObservations {
    pub fn new(epoch: Epoch) -> Self {
        Self {
            epoch,
            flag: 0,
            clock_offset: None,
            satellites: HashMap::new(),
        }
    }

    pub fn num_satellites(&self) -> usize {
        self.satellites.len()
    }

    pub fn get_obs(&self, sat: &Satellite, code: &SignalCode) -> Option<f64> {
        self.satellites.get(sat).and_then(|obs| obs.get(code)).map(|v| v.value)
    }
}

/// RINEX observation file header
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Header {
    pub version: RinexVersion,
    pub file_type: char,
    pub satellite_system: Option<GnssSystem>,
    pub program: String,
    pub run_by: String,
    pub date: String,
    pub marker_name: String,
    pub marker_number: String,
    pub marker_type: Option<String>,
    pub observer: String,
    pub agency: String,
    pub receiver_number: String,
    pub receiver_type: String,
    pub receiver_version: String,
    pub antenna_number: String,
    pub antenna_type: String,
    pub approx_position: Option<Ecef>,
    pub antenna_delta: Option<[f64; 3]>,
    pub obs_types: HashMap<GnssSystem, Vec<SignalCode>>,
    pub obs_types_v2: Vec<String>,
    pub time_first_obs: Option<Epoch>,
    pub time_last_obs: Option<Epoch>,
    pub time_system: String,
    pub interval: Option<f64>,
    pub leap_seconds: Option<i32>,
    pub num_satellites: Option<u32>,
    pub glonass_slot_frq: HashMap<u32, i8>,
    pub glonass_cod_phs_bis: Option<HashMap<String, f64>>,
    pub comments: Vec<String>,
}

impl Header {
    pub fn get_obs_types(&self, system: &GnssSystem) -> Option<&Vec<SignalCode>> {
        self.obs_types.get(system)
    }

    pub fn has_position(&self) -> bool {
        self.approx_position.as_ref().map(|p| p.is_valid()).unwrap_or(false)
    }
}

/// Complete RINEX observation data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservationData {
    /// Header information
    pub header: Header,
    /// Epochs with observations
    pub epochs: Vec<EpochObservations>,
}

impl ObservationData {
    /// Create new observation data
    pub fn new(header: Header) -> Self {
        Self {
            header,
            epochs: Vec::new(),
        }
    }

    /// Get number of epochs
    pub fn num_epochs(&self) -> usize {
        self.epochs.len()
    }

    /// Get all unique satellites
    pub fn satellites(&self) -> Vec<Satellite> {
        let mut sats: Vec<Satellite> = self.epochs
            .iter()
            .flat_map(|e| e.satellites.keys().cloned())
            .collect();
        sats.sort_by_key(|s| (s.system.to_char(), s.prn));
        sats.dedup();
        sats
    }

    /// Get satellites for a specific system
    pub fn satellites_for_system(&self, system: GnssSystem) -> Vec<Satellite> {
        self.satellites()
            .into_iter()
            .filter(|s| s.system == system)
            .collect()
    }

    /// Get observation at epoch index for satellite and code
    pub fn get_obs(&self, epoch_idx: usize, sat: &Satellite, code: &SignalCode) -> Option<f64> {
        self.epochs.get(epoch_idx).and_then(|e| e.get_obs(sat, code))
    }

    /// Get time series for a satellite and signal code
    pub fn get_time_series(&self, sat: &Satellite, code: &SignalCode) -> Vec<(Epoch, f64)> {
        self.epochs
            .iter()
            .filter_map(|e| {
                e.get_obs(sat, code).map(|v| (e.epoch, v))
            })
            .collect()
    }

    /// Get systems present in the data
    pub fn systems(&self) -> Vec<GnssSystem> {
        let mut systems: Vec<GnssSystem> = self.satellites()
            .iter()
            .map(|s| s.system)
            .collect();
        systems.sort_by_key(|s| s.to_char());
        systems.dedup();
        systems
    }

    /// Get available signal codes for a system
    pub fn signal_codes_for_system(&self, system: GnssSystem) -> Vec<SignalCode> {
        let mut codes: Vec<SignalCode> = self.epochs
            .iter()
            .flat_map(|e| {
                e.satellites
                    .iter()
                    .filter(|(s, _)| s.system == system)
                    .flat_map(|(_, obs)| obs.keys().cloned())
            })
            .collect();
        codes.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        codes.dedup();
        codes
    }

    /// Get observation interval
    pub fn interval(&self) -> Option<f64> {
        if self.epochs.len() < 2 {
            return self.header.interval;
        }
        
        // Calculate from first two epochs
        let diff = self.epochs[1].epoch.diff(&self.epochs[0].epoch);
        Some(diff.abs())
    }

    /// Get time span in seconds
    pub fn time_span(&self) -> Option<f64> {
        if self.epochs.is_empty() {
            return None;
        }
        let first = &self.epochs.first()?.epoch;
        let last = &self.epochs.last()?.epoch;
        Some(last.diff(first).abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rinex_version_parse() {
        assert_eq!(RinexVersion::parse("3.05"), Some(RinexVersion::V3(5)));
        assert_eq!(RinexVersion::parse("4.02"), Some(RinexVersion::V4(2)));
        assert_eq!(RinexVersion::parse("2.11"), Some(RinexVersion::V2(11)));
    }

    #[test]
    fn test_satellite_parse() {
        let sat = Satellite::parse("G15").unwrap();
        assert_eq!(sat.system, GnssSystem::Gps);
        assert_eq!(sat.prn, 15);

        let sat2 = Satellite::parse("E 1").unwrap();
        assert_eq!(sat2.system, GnssSystem::Galileo);
        assert_eq!(sat2.prn, 1);
    }

    #[test]
    fn test_signal_code_parse() {
        let code = SignalCode::parse("C1C").unwrap();
        assert_eq!(code.obs_type, ObservationType::Code);
        assert_eq!(code.band, 1);
        assert_eq!(code.attribute, 'C');

        let code2 = SignalCode::parse("L2W").unwrap();
        assert!(code2.is_phase());
        assert_eq!(code2.band, 2);
    }
}
