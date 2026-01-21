//! Multipath analysis core functionality.
//!
//! Implements code multipath estimation using linear combinations of
//! code and phase observations on dual-frequency receivers.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::rinex::{
    EpochObservations, GnssSystem, ObservationData, ObservationType, 
    Satellite, SignalCode,
};
use crate::navigation::SatellitePosition;
use crate::utils::{
    constants::{alpha_factor, get_frequency, thresholds, SPEED_OF_LIGHT},
    AzEl, Ecef, Epoch, Result,
};

/// Multipath estimate for a single observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipathEstimate {
    /// Satellite
    pub satellite: Satellite,
    /// Epoch
    pub epoch: Epoch,
    /// Primary signal code (e.g., C1C)
    pub primary_code: String,
    /// Secondary signal code (e.g., C2W)
    pub secondary_code: String,
    /// Multipath value (meters)
    pub mp_value: f64,
    /// Elevation angle (degrees)
    pub elevation: f64,
    /// Azimuth angle (degrees)
    pub azimuth: f64,
    /// SNR if available (dB-Hz)
    pub snr: Option<f64>,
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Elevation cutoff angle (degrees)
    pub elevation_cutoff: f64,
    /// Systems to include
    pub systems: Vec<GnssSystem>,
    /// Signal pairs to analyze (if empty, auto-detect)
    pub signal_pairs: Vec<(SignalCode, SignalCode)>,
    /// Ionospheric rate threshold for cycle slip (m/s)
    pub ion_threshold: f64,
    /// Code-phase threshold for cycle slip (m/s)
    pub code_phase_threshold: f64,
    /// Include SNR analysis
    pub include_snr: bool,
    /// Weight by elevation
    pub elevation_weighting: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            elevation_cutoff: thresholds::ELEVATION_CUTOFF,
            systems: vec![
                GnssSystem::Gps,
                GnssSystem::Glonass,
                GnssSystem::Galileo,
                GnssSystem::Beidou,
            ],
            signal_pairs: Vec::new(),
            ion_threshold: thresholds::ION_RATE_THRESHOLD,
            code_phase_threshold: thresholds::CODE_PHASE_THRESHOLD,
            include_snr: true,
            elevation_weighting: true,
        }
    }
}

impl AnalysisConfig {
    /// Create with custom elevation cutoff
    pub fn with_elevation_cutoff(mut self, cutoff: f64) -> Self {
        self.elevation_cutoff = cutoff;
        self
    }

    /// Set systems to analyze
    pub fn with_systems(mut self, systems: &[&str]) -> Self {
        self.systems = systems
            .iter()
            .filter_map(|s| GnssSystem::from_char(s.chars().next()?))
            .collect();
        self
    }

    /// Add a signal pair for multipath analysis
    pub fn add_signal_pair(mut self, primary: &str, secondary: &str) -> Self {
        if let (Some(p), Some(s)) = (SignalCode::parse(primary), SignalCode::parse(secondary)) {
            self.signal_pairs.push((p, s));
        }
        self
    }
}

/// Statistics for multipath analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultipathStatistics {
    /// Number of estimates
    pub count: usize,
    /// Mean value (meters)
    pub mean: f64,
    /// Standard deviation (meters)
    pub std_dev: f64,
    /// RMS value (meters)
    pub rms: f64,
    /// Weighted RMS (elevation-weighted)
    pub weighted_rms: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Number of cycle slips detected
    pub cycle_slips: usize,
}

impl MultipathStatistics {
    /// Compute statistics from estimates
    pub fn compute(estimates: &[MultipathEstimate], use_weighting: bool) -> Self {
        if estimates.is_empty() {
            return Self::default();
        }

        let n = estimates.len();
        let values: Vec<f64> = estimates.iter().map(|e| e.mp_value).collect();
        
        // Basic statistics
        let sum: f64 = values.iter().sum();
        let mean = sum / n as f64;
        
        let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();
        
        let rms = (values.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
        
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Weighted RMS (elevation-dependent weighting)
        let weighted_rms = if use_weighting {
            let mut sum_weighted = 0.0;
            let mut sum_weights = 0.0;
            
            for est in estimates {
                let weight = crate::utils::elevation_weight(est.elevation);
                sum_weighted += weight * est.mp_value * est.mp_value;
                sum_weights += weight;
            }
            
            if sum_weights > 0.0 {
                (sum_weighted / sum_weights).sqrt()
            } else {
                rms
            }
        } else {
            rms
        };

        Self {
            count: n,
            mean,
            std_dev,
            rms,
            weighted_rms,
            min,
            max,
            cycle_slips: 0,
        }
    }
}

/// Multipath analyzer
pub struct MultipathAnalyzer {
    /// Configuration
    config: AnalysisConfig,
    /// Observation data
    obs_data: ObservationData,
    /// Satellite positions (precomputed)
    positions: HashMap<(Satellite, Epoch), SatellitePosition>,
    /// Receiver position
    receiver_pos: Option<Ecef>,
    /// GLONASS frequency channel map
    glonass_fcn: HashMap<u32, i8>,
}

impl MultipathAnalyzer {
    /// Create a new analyzer
    pub fn new(obs_data: ObservationData, config: AnalysisConfig) -> Self {
        // Get receiver position from header if available
        let receiver_pos = obs_data.header.approx_position.clone();
        let glonass_fcn = obs_data.header.glonass_slot_frq.clone();

        Self {
            config,
            obs_data,
            positions: HashMap::new(),
            receiver_pos,
            glonass_fcn,
        }
    }

    /// Set satellite positions
    pub fn with_positions(mut self, positions: HashMap<(Satellite, Epoch), SatellitePosition>) -> Self {
        self.positions = positions;
        self
    }

    /// Set receiver position
    pub fn with_receiver_position(mut self, pos: Ecef) -> Self {
        self.receiver_pos = Some(pos);
        self
    }

    /// Analyze multipath for all available signal pairs
    pub fn analyze(&self) -> Result<AnalysisResults> {
        let mut results = AnalysisResults::default();
        
        // Determine signal pairs to analyze
        let signal_pairs = if self.config.signal_pairs.is_empty() {
            self.detect_signal_pairs()
        } else {
            self.config.signal_pairs.clone()
        };

        // Analyze each system
        for system in &self.config.systems {
            let satellites = self.obs_data.satellites_for_system(*system);
            
            if satellites.is_empty() {
                continue;
            }

            // Find valid pairs for this system
            let valid_pairs: Vec<(SignalCode, SignalCode)> = signal_pairs
                .iter()
                .filter(|(p, s)| {
                    self.is_valid_pair_for_system(*system, p, s)
                })
                .cloned()
                .collect();

            for (primary, secondary) in valid_pairs {
                let estimates = self.compute_multipath_estimates(
                    *system,
                    &satellites,
                    &primary,
                    &secondary,
                );

                if !estimates.is_empty() {
                    let key = format!("{}_{}", system.to_char(), primary);
                    let stats = MultipathStatistics::compute(&estimates, self.config.elevation_weighting);
                    
                    results.estimates.insert(key.clone(), estimates);
                    results.statistics.insert(key, stats);
                }
            }
        }

        // Compute summary statistics
        results.compute_summary();

        Ok(results)
    }

    /// Detect available signal pairs for multipath analysis
    fn detect_signal_pairs(&self) -> Vec<(SignalCode, SignalCode)> {
        let mut pairs = Vec::new();

        for system in &self.config.systems {
            let codes = self.obs_data.signal_codes_for_system(*system);
            
            // Find code observations
            let code_obs: Vec<&SignalCode> = codes
                .iter()
                .filter(|c| c.is_code())
                .collect();

            // Find phase observations
            let phase_obs: Vec<&SignalCode> = codes
                .iter()
                .filter(|c| c.is_phase())
                .collect();

            // Match bands for multipath estimation
            for code in &code_obs {
                // Find phase on same band
                let phase1 = phase_obs
                    .iter()
                    .find(|p| p.band == code.band && p.attribute == code.attribute);

                // Find phase on different band
                let phase2 = phase_obs
                    .iter()
                    .find(|p| p.band != code.band);

                if let (Some(p1), Some(p2)) = (phase1, phase2) {
                    pairs.push(((*code).clone(), SignalCode::new(
                        ObservationType::Code,
                        (*p2).band,
                        (*p2).attribute,
                    )));
                }
            }
        }

        // Add common GPS pairs if detected
        if pairs.is_empty() {
            // Default GPS L1/L2 pair
            pairs.push((
                SignalCode::new(ObservationType::Code, 1, 'C'),
                SignalCode::new(ObservationType::Code, 2, 'W'),
            ));
        }

        pairs
    }

    /// Check if a signal pair is valid for a system
    fn is_valid_pair_for_system(
        &self,
        system: GnssSystem,
        primary: &SignalCode,
        secondary: &SignalCode,
    ) -> bool {
        let sys_char = system.to_char();
        let fcn = if system == GnssSystem::Glonass {
            Some(0i8) // Default FCN
        } else {
            None
        };

        let f1 = get_frequency(sys_char, primary.band, fcn);
        let f2 = get_frequency(sys_char, secondary.band, fcn);

        f1.is_some() && f2.is_some() && primary.band != secondary.band
    }

    /// Compute multipath estimates for a signal pair
    fn compute_multipath_estimates(
        &self,
        system: GnssSystem,
        satellites: &[Satellite],
        primary: &SignalCode,
        secondary: &SignalCode,
    ) -> Vec<MultipathEstimate> {
        let mut estimates = Vec::new();
        let sys_char = system.to_char();

        // Get frequencies
        let fcn = if system == GnssSystem::Glonass { Some(0i8) } else { None };
        let f1 = match get_frequency(sys_char, primary.band, fcn) {
            Some(f) => f,
            None => return estimates,
        };
        let f2 = match get_frequency(sys_char, secondary.band, fcn) {
            Some(f) => f,
            None => return estimates,
        };

        // Alpha factor (f1²/f2²)
        let alpha = alpha_factor(f1, f2);

        // Corresponding phase codes
        let phase1 = SignalCode::new(ObservationType::Phase, primary.band, primary.attribute);
        let phase2 = SignalCode::new(ObservationType::Phase, secondary.band, secondary.attribute);

        // SNR code
        let snr_code = SignalCode::new(ObservationType::Snr, primary.band, primary.attribute);

        for sat in satellites {
            if sat.system != system {
                continue;
            }

            // Get GLONASS FCN for this satellite
            let sat_fcn = if system == GnssSystem::Glonass {
                self.glonass_fcn.get(&sat.prn).copied()
            } else {
                None
            };

            // Recalculate frequencies for GLONASS with actual FCN
            let (f1_sat, f2_sat) = if system == GnssSystem::Glonass {
                let f1 = get_frequency(sys_char, primary.band, sat_fcn).unwrap_or(f1);
                let f2 = get_frequency(sys_char, secondary.band, sat_fcn).unwrap_or(f2);
                (f1, f2)
            } else {
                (f1, f2)
            };
            let alpha_sat = alpha_factor(f1_sat, f2_sat);

            // Previous epoch values for bias estimation
            let mut prev_mp: Option<f64> = None;

            for epoch_obs in &self.obs_data.epochs {
                // Get observations for this satellite
                let sat_obs = match epoch_obs.satellites.get(sat) {
                    Some(o) => o,
                    None => continue,
                };

                // Get required observations
                let c1 = match sat_obs.get(primary).map(|v| v.value) {
                    Some(v) => v,
                    None => continue,
                };
                let l1 = match sat_obs.get(&phase1).map(|v| v.value) {
                    Some(v) => v,
                    None => continue,
                };
                let l2 = match sat_obs.get(&phase2).map(|v| v.value) {
                    Some(v) => v,
                    None => continue,
                };

                // Convert phase from cycles to meters
                let lambda1 = SPEED_OF_LIGHT / f1_sat;
                let lambda2 = SPEED_OF_LIGHT / f2_sat;
                let phi1 = l1 * lambda1;
                let phi2 = l2 * lambda2;

                // Multipath linear combination:
                // MP1 = R1 - (1 + 2/(α-1)) * Φ1 + (2/(α-1)) * Φ2
                let mp = c1 
                    - (1.0 + 2.0 / (alpha_sat - 1.0)) * phi1 
                    + (2.0 / (alpha_sat - 1.0)) * phi2;

                // Get elevation/azimuth
                let (elevation, azimuth) = self.get_azel(sat, &epoch_obs.epoch);

                // Apply elevation cutoff
                if elevation < self.config.elevation_cutoff {
                    continue;
                }

                // Get SNR if available
                let snr = sat_obs.get(&snr_code).map(|v| v.value);

                // Remove bias (mean of first few estimates)
                let mp_debiased = if let Some(prev) = prev_mp {
                    // Simple recursive mean removal
                    mp - (mp + prev) / 2.0 + prev_mp.unwrap_or(0.0)
                } else {
                    0.0 // First estimate used as reference
                };
                prev_mp = Some(mp);

                estimates.push(MultipathEstimate {
                    satellite: *sat,
                    epoch: epoch_obs.epoch,
                    primary_code: primary.to_string(),
                    secondary_code: secondary.to_string(),
                    mp_value: mp_debiased,
                    elevation,
                    azimuth,
                    snr,
                });
            }
        }

        estimates
    }

    /// Get azimuth and elevation for satellite at epoch
    fn get_azel(&self, sat: &Satellite, epoch: &Epoch) -> (f64, f64) {
        // Try precomputed positions
        if let Some(pos) = self.positions.get(&(*sat, *epoch)) {
            if let Some(rec) = &self.receiver_pos {
                let azel = crate::utils::calculate_azel(rec, &pos.position);
                return (azel.elevation, azel.azimuth);
            }
        }

        // Default values if position not available
        (45.0, 0.0)
    }
}

/// Analysis results container
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisResults {
    /// Multipath estimates per signal
    pub estimates: HashMap<String, Vec<MultipathEstimate>>,
    /// Statistics per signal
    pub statistics: HashMap<String, MultipathStatistics>,
    /// Summary statistics
    pub summary: SummaryStatistics,
}

impl AnalysisResults {
    /// Compute summary statistics
    pub fn compute_summary(&mut self) {
        let total_estimates: usize = self.estimates.values().map(|e| e.len()).sum();
        
        let all_rms: Vec<f64> = self.statistics.values().map(|s| s.rms).collect();
        let avg_rms = if !all_rms.is_empty() {
            all_rms.iter().sum::<f64>() / all_rms.len() as f64
        } else {
            0.0
        };

        let total_cycle_slips: usize = self.statistics.values().map(|s| s.cycle_slips).sum();

        self.summary = SummaryStatistics {
            total_estimates,
            num_signals: self.statistics.len(),
            average_rms: avg_rms,
            total_cycle_slips,
        };
    }

    /// Export to CSV
    pub fn to_csv(&self, path: &str) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;
        
        // Header
        writeln!(file, "Signal,Count,Mean,StdDev,RMS,WeightedRMS,Min,Max,CycleSlips")?;

        // Data
        for (signal, stats) in &self.statistics {
            writeln!(
                file,
                "{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{}",
                signal,
                stats.count,
                stats.mean,
                stats.std_dev,
                stats.rms,
                stats.weighted_rms,
                stats.min,
                stats.max,
                stats.cycle_slips
            )?;
        }

        Ok(())
    }
}

/// Summary statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SummaryStatistics {
    /// Total number of estimates
    pub total_estimates: usize,
    /// Number of signals analyzed
    pub num_signals: usize,
    /// Average RMS across all signals
    pub average_rms: f64,
    /// Total cycle slips detected
    pub total_cycle_slips: usize,
}
