//! Navigation and ephemeris data types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::rinex::{GnssSystem, Satellite};
use crate::utils::{Ecef, Epoch};

/// Keplerian orbital elements for GPS, Galileo, BeiDou
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KeplerianElements {
    /// Reference epoch (TOE for GPS/Galileo, or TOC)
    pub toe: f64,
    /// Square root of semi-major axis (m^0.5)
    pub sqrt_a: f64,
    /// Eccentricity
    pub e: f64,
    /// Inclination at reference time (rad)
    pub i0: f64,
    /// Longitude of ascending node at reference time (rad)
    pub omega0: f64,
    /// Argument of perigee (rad)
    pub omega: f64,
    /// Mean anomaly at reference time (rad)
    pub m0: f64,
    /// Rate of right ascension (rad/s)
    pub omega_dot: f64,
    /// Rate of inclination angle (rad/s)
    pub idot: f64,
    /// Mean motion difference (rad/s)
    pub delta_n: f64,
    /// Latitude argument correction (amplitude, cos) (rad)
    pub cuc: f64,
    /// Latitude argument correction (amplitude, sin) (rad)
    pub cus: f64,
    /// Orbital radius correction (amplitude, cos) (m)
    pub crc: f64,
    /// Orbital radius correction (amplitude, sin) (m)
    pub crs: f64,
    /// Inclination correction (amplitude, cos) (rad)
    pub cic: f64,
    /// Inclination correction (amplitude, sin) (rad)
    pub cis: f64,
    /// Clock bias (s)
    pub af0: f64,
    /// Clock drift (s/s)
    pub af1: f64,
    /// Clock drift rate (s/s²)
    pub af2: f64,
    /// GPS week number
    pub week: u32,
    /// Time of clock (s)
    pub toc: f64,
    /// IODC (Issue of Data, Clock)
    pub iodc: Option<u32>,
    /// IODE (Issue of Data, Ephemeris)
    pub iode: Option<u32>,
    /// Group delay (TGD) (s)
    pub tgd: Option<f64>,
    /// SV accuracy (m)
    pub sv_accuracy: Option<f64>,
    /// SV health
    pub sv_health: Option<u32>,
    /// Fit interval (hours)
    pub fit_interval: Option<f64>,
}

impl Default for KeplerianElements {
    fn default() -> Self {
        Self {
            toe: 0.0,
            sqrt_a: 0.0,
            e: 0.0,
            i0: 0.0,
            omega0: 0.0,
            omega: 0.0,
            m0: 0.0,
            omega_dot: 0.0,
            idot: 0.0,
            delta_n: 0.0,
            cuc: 0.0,
            cus: 0.0,
            crc: 0.0,
            crs: 0.0,
            cic: 0.0,
            cis: 0.0,
            af0: 0.0,
            af1: 0.0,
            af2: 0.0,
            week: 0,
            toc: 0.0,
            iodc: None,
            iode: None,
            tgd: None,
            sv_accuracy: None,
            sv_health: None,
            fit_interval: None,
        }
    }
}

/// GLONASS state vector ephemeris
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GlonassEphemeris {
    /// Reference epoch
    pub epoch: Epoch,
    /// Position X (km in PZ-90)
    pub x: f64,
    /// Position Y (km)
    pub y: f64,
    /// Position Z (km)
    pub z: f64,
    /// Velocity X (km/s)
    pub vx: f64,
    /// Velocity Y (km/s)
    pub vy: f64,
    /// Velocity Z (km/s)
    pub vz: f64,
    /// Acceleration X (km/s²)
    pub ax: f64,
    /// Acceleration Y (km/s²)
    pub ay: f64,
    /// Acceleration Z (km/s²)
    pub az: f64,
    /// Clock bias (s) - τn (tau_n)
    pub tau_n: f64,
    /// Clock frequency bias - γn (gamma_n)
    pub gamma_n: f64,
    /// Message frame time (s)
    pub tk: f64,
    /// Frequency channel number (-7 to +6)
    pub frequency_channel: i8,
    /// Health
    pub health: Option<u8>,
    /// Age of operation (days)
    pub age: Option<u8>,
}

impl Default for GlonassEphemeris {
    fn default() -> Self {
        Self {
            epoch: Epoch::new(2000, 1, 1, 0, 0, 0.0),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            ax: 0.0,
            ay: 0.0,
            az: 0.0,
            tau_n: 0.0,
            gamma_n: 0.0,
            tk: 0.0,
            frequency_channel: 0,
            health: None,
            age: None,
        }
    }
}

/// Ephemeris data (union of different types)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Ephemeris {
    /// Keplerian elements (GPS, Galileo, BeiDou, QZSS, NavIC)
    Keplerian(KeplerianElements),
    /// GLONASS state vector
    Glonass(GlonassEphemeris),
}

impl Ephemeris {
    /// Get reference epoch
    pub fn reference_epoch(&self) -> Epoch {
        match self {
            Ephemeris::Keplerian(k) => {
                // Convert TOC to epoch (simplified)
                let seconds_in_week = 604800.0;
                let _weeks_since_1980 = k.week;
                // Approximate conversion
                Epoch::new(2000 + (k.week / 52) as i32, 1, 1, 0, 0, k.toc)
            }
            Ephemeris::Glonass(g) => g.epoch,
        }
    }

    /// Check if ephemeris is for Keplerian system
    pub fn is_keplerian(&self) -> bool {
        matches!(self, Ephemeris::Keplerian(_))
    }

    /// Check if ephemeris is for GLONASS
    pub fn is_glonass(&self) -> bool {
        matches!(self, Ephemeris::Glonass(_))
    }

    /// Get clock bias
    pub fn clock_bias(&self) -> f64 {
        match self {
            Ephemeris::Keplerian(k) => k.af0,
            Ephemeris::Glonass(g) => g.tau_n,
        }
    }

    /// Get clock drift
    pub fn clock_drift(&self) -> f64 {
        match self {
            Ephemeris::Keplerian(k) => k.af1,
            Ephemeris::Glonass(g) => g.gamma_n,
        }
    }
}

/// Satellite position at a specific epoch
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SatellitePosition {
    /// Satellite identifier
    pub satellite: Satellite,
    /// Epoch
    pub epoch: Epoch,
    /// Position in ECEF (m)
    pub position: Ecef,
    /// Velocity in ECEF (m/s), if available
    pub velocity: Option<[f64; 3]>,
    /// Clock bias (s)
    pub clock_bias: f64,
    /// Clock drift (s/s)
    pub clock_drift: f64,
    /// Relativistic correction (s)
    pub relativistic_correction: f64,
}

impl SatellitePosition {
    /// Create a new satellite position
    pub fn new(satellite: Satellite, epoch: Epoch, position: Ecef) -> Self {
        Self {
            satellite,
            epoch,
            position,
            velocity: None,
            clock_bias: 0.0,
            clock_drift: 0.0,
            relativistic_correction: 0.0,
        }
    }

    /// Total clock correction (bias + relativistic)
    pub fn total_clock_correction(&self) -> f64 {
        self.clock_bias + self.relativistic_correction
    }
}

/// SP3 precise orbit entry
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sp3Entry {
    /// Epoch
    pub epoch: Epoch,
    /// Position X (m)
    pub x: f64,
    /// Position Y (m)
    pub y: f64,
    /// Position Z (m)
    pub z: f64,
    /// Clock bias (microseconds)
    pub clock: f64,
    /// Standard deviations (if available)
    pub x_sdev: Option<f64>,
    pub y_sdev: Option<f64>,
    pub z_sdev: Option<f64>,
    pub clock_sdev: Option<f64>,
    /// Velocity (if available)
    pub vx: Option<f64>,
    pub vy: Option<f64>,
    pub vz: Option<f64>,
}

impl Sp3Entry {
    /// Get position as ECEF
    pub fn position(&self) -> Ecef {
        Ecef::new(self.x, self.y, self.z)
    }

    /// Get clock bias in seconds
    pub fn clock_seconds(&self) -> f64 {
        self.clock * 1e-6
    }
}

/// Complete SP3 file data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Sp3Data {
    /// Version (a, b, c, d)
    pub version: char,
    /// Position/velocity flag
    pub pos_vel_flag: char,
    /// Start epoch
    pub start_epoch: Option<Epoch>,
    /// Number of epochs
    pub num_epochs: usize,
    /// Data used descriptor
    pub data_used: String,
    /// Coordinate system
    pub coord_system: String,
    /// Orbit type
    pub orbit_type: String,
    /// Agency
    pub agency: String,
    /// GPS week
    pub gps_week: u32,
    /// Seconds of week
    pub seconds_of_week: f64,
    /// Epoch interval (seconds)
    pub interval: f64,
    /// Modified Julian Day
    pub mjd: f64,
    /// Fractional day
    pub fractional_day: f64,
    /// Number of satellites
    pub num_satellites: usize,
    /// Satellite list
    pub satellites: Vec<Satellite>,
    /// Accuracy exponents per satellite
    pub accuracy: HashMap<Satellite, u8>,
    /// File type (G=GPS, M=Mixed, etc.)
    pub file_type: char,
    /// Time system
    pub time_system: String,
    /// Comments
    pub comments: Vec<String>,
    /// Satellite data entries
    pub entries: HashMap<Satellite, Vec<Sp3Entry>>,
}

impl Sp3Data {
    /// Get entries for a satellite
    pub fn get_satellite(&self, sat: &Satellite) -> Option<&Vec<Sp3Entry>> {
        self.entries.get(sat)
    }

    /// Get all epochs
    pub fn epochs(&self) -> Vec<Epoch> {
        let mut epochs: Vec<Epoch> = self.entries
            .values()
            .flat_map(|entries| entries.iter().map(|e| e.epoch))
            .collect();
        epochs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        epochs.dedup();
        epochs
    }
}

/// Navigation data container
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationData {
    /// Ephemeris data per satellite, sorted by epoch
    pub ephemerides: HashMap<Satellite, Vec<Ephemeris>>,
    /// Ionospheric correction parameters (Klobuchar/NeQuick)
    pub ionospheric_params: Option<IonosphericParams>,
    /// Leap seconds
    pub leap_seconds: Option<i32>,
    /// Time system corrections
    pub time_corrections: HashMap<String, TimeCorrection>,
}

impl NavigationData {
    /// Get ephemeris for satellite closest to given epoch
    pub fn get_ephemeris(&self, sat: &Satellite, epoch: &Epoch) -> Option<&Ephemeris> {
        let ephs = self.ephemerides.get(sat)?;
        if ephs.is_empty() {
            return None;
        }

        // Find closest ephemeris by reference epoch
        let mut best: Option<&Ephemeris> = None;
        let mut best_diff = f64::MAX;

        for eph in ephs {
            let diff = epoch.diff(&eph.reference_epoch()).abs();
            if diff < best_diff {
                best_diff = diff;
                best = Some(eph);
            }
        }

        best
    }

    /// Add ephemeris for a satellite
    pub fn add_ephemeris(&mut self, sat: Satellite, eph: Ephemeris) {
        self.ephemerides.entry(sat).or_insert_with(Vec::new).push(eph);
    }
}

/// Ionospheric correction parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IonosphericParams {
    /// Klobuchar model (GPS)
    Klobuchar {
        alpha: [f64; 4],
        beta: [f64; 4],
    },
    /// NeQuick model (Galileo)
    NeQuick {
        ai0: f64,
        ai1: f64,
        ai2: f64,
        storm_flag: bool,
    },
    /// BeiDou ionospheric model
    BeiDou {
        alpha: [f64; 4],
        beta: [f64; 4],
    },
}

/// Time system correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCorrection {
    /// A0 coefficient
    pub a0: f64,
    /// A1 coefficient
    pub a1: f64,
    /// Reference time
    pub reference_time: f64,
    /// Reference week
    pub reference_week: u32,
    /// Source system
    pub source: String,
    /// Target system
    pub target: String,
}
