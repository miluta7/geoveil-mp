//! Cycle slip detection algorithms.
//!
//! Implements cycle slip detection using:
//! - Ionospheric residual (rate of change)
//! - Code-phase combination

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::rinex::{
    EpochObservations, GnssSystem, ObservationData, ObservationType, 
    Satellite, SignalCode,
};
use crate::utils::{
    constants::{get_frequency, thresholds, SPEED_OF_LIGHT},
    Epoch,
};

/// Detected cycle slip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleSlip {
    /// Satellite
    pub satellite: Satellite,
    /// Epoch where slip was detected
    pub epoch: Epoch,
    /// Detection method
    pub method: CycleSlipMethod,
    /// Magnitude of the indicator
    pub magnitude: f64,
    /// Threshold that was exceeded
    pub threshold: f64,
    /// Affected signal(s)
    pub signals: Vec<SignalCode>,
}

/// Cycle slip detection method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleSlipMethod {
    /// Ionospheric residual rate
    IonosphericRate,
    /// Code-phase combination
    CodePhase,
    /// Loss of lock indicator (LLI)
    LLI,
    /// Doppler consistency
    Doppler,
}

/// Cycle slip detector
pub struct CycleSlipDetector {
    /// Ionospheric rate threshold (m/s)
    ion_threshold: f64,
    /// Code-phase threshold (m/s)
    code_phase_threshold: f64,
    /// Previous epoch data for differencing
    prev_data: HashMap<Satellite, PrevEpochData>,
}

/// Data from previous epoch for differencing
struct PrevEpochData {
    epoch: Epoch,
    ion_combo: Option<f64>,
    code_phase: HashMap<SignalCode, f64>,
}

impl Default for CycleSlipDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl CycleSlipDetector {
    /// Create a new detector with default thresholds
    pub fn new() -> Self {
        Self {
            ion_threshold: thresholds::ION_RATE_THRESHOLD,
            code_phase_threshold: thresholds::CODE_PHASE_THRESHOLD,
            prev_data: HashMap::new(),
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(ion_threshold: f64, code_phase_threshold: f64) -> Self {
        Self {
            ion_threshold: if ion_threshold > 0.0 { ion_threshold } else { thresholds::ION_RATE_THRESHOLD },
            code_phase_threshold: if code_phase_threshold > 0.0 { code_phase_threshold } else { thresholds::CODE_PHASE_THRESHOLD },
            prev_data: HashMap::new(),
        }
    }

    /// Detect cycle slips in observation data
    pub fn detect(&mut self, obs_data: &ObservationData) -> Vec<CycleSlip> {
        let mut slips = Vec::new();
        
        // Reset state
        self.prev_data.clear();

        for epoch_obs in &obs_data.epochs {
            for (sat, sat_obs) in &epoch_obs.satellites {
                let epoch_slips = self.detect_for_satellite(
                    sat,
                    sat_obs,
                    &epoch_obs.epoch,
                );
                slips.extend(epoch_slips);

                // Update previous epoch data
                self.update_prev_data(sat, sat_obs, &epoch_obs.epoch);
            }
        }

        slips
    }

    /// Detect cycle slips for a single satellite at one epoch
    fn detect_for_satellite(
        &self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
        epoch: &Epoch,
    ) -> Vec<CycleSlip> {
        let mut slips = Vec::new();

        // Check LLI flags
        for (code, value) in obs {
            if value.has_cycle_slip() || value.has_loss_of_lock() {
                slips.push(CycleSlip {
                    satellite: *sat,
                    epoch: *epoch,
                    method: CycleSlipMethod::LLI,
                    magnitude: 1.0,
                    threshold: 0.0,
                    signals: vec![code.clone()],
                });
            }
        }

        // Get previous epoch data
        let prev = match self.prev_data.get(sat) {
            Some(p) => p,
            None => return slips,
        };

        // Time difference
        let dt = epoch.diff(&prev.epoch);
        if dt.abs() < 0.001 || dt.abs() > 300.0 {
            // Skip if same epoch or gap > 5 minutes
            return slips;
        }

        // Ionospheric rate check
        if let Some(slip) = self.check_ionospheric_rate(sat, obs, epoch, prev, dt) {
            slips.push(slip);
        }

        // Code-phase combination check
        slips.extend(self.check_code_phase(sat, obs, epoch, prev, dt));

        slips
    }

    /// Check ionospheric residual rate
    fn check_ionospheric_rate(
        &self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
        epoch: &Epoch,
        prev: &PrevEpochData,
        dt: f64,
    ) -> Option<CycleSlip> {
        let prev_ion = prev.ion_combo?;

        // Get phase observations on two frequencies
        let sys_char = sat.system.to_char();
        let fcn = if sat.system == GnssSystem::Glonass { Some(0i8) } else { None };

        // Find L1 and L2 (or equivalent) phase observations
        let mut l1_value: Option<f64> = None;
        let mut l2_value: Option<f64> = None;
        let mut l1_code: Option<SignalCode> = None;
        let mut l2_code: Option<SignalCode> = None;

        for (code, value) in obs {
            if code.is_phase() {
                if code.band == 1 {
                    l1_value = Some(value.value);
                    l1_code = Some(code.clone());
                } else if code.band == 2 || code.band == 5 {
                    l2_value = Some(value.value);
                    l2_code = Some(code.clone());
                }
            }
        }

        let (l1, l2) = match (l1_value, l2_value) {
            (Some(l1), Some(l2)) => (l1, l2),
            _ => return None,
        };

        // Get frequencies
        let f1 = get_frequency(sys_char, 1, fcn)?;
        let f2 = get_frequency(sys_char, l2_code.as_ref()?.band, fcn)?;

        // Convert to meters
        let lambda1 = SPEED_OF_LIGHT / f1;
        let lambda2 = SPEED_OF_LIGHT / f2;
        let phi1 = l1 * lambda1;
        let phi2 = l2 * lambda2;

        // Ionospheric combination: I = (Φ1 - Φ2) / (α - 1)
        let alpha = (f1 * f1) / (f2 * f2);
        let ion_combo = (phi1 - phi2) / (alpha - 1.0);

        // Rate of ionospheric delay change
        let ion_rate = (ion_combo - prev_ion) / dt;

        if ion_rate.abs() > self.ion_threshold {
            Some(CycleSlip {
                satellite: *sat,
                epoch: *epoch,
                method: CycleSlipMethod::IonosphericRate,
                magnitude: ion_rate.abs(),
                threshold: self.ion_threshold,
                signals: vec![l1_code?, l2_code?],
            })
        } else {
            None
        }
    }

    /// Check code-phase combination
    fn check_code_phase(
        &self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
        epoch: &Epoch,
        prev: &PrevEpochData,
        dt: f64,
    ) -> Vec<CycleSlip> {
        let mut slips = Vec::new();
        let sys_char = sat.system.to_char();
        let fcn = if sat.system == GnssSystem::Glonass { Some(0i8) } else { None };

        // Check each code observation
        for (code, value) in obs {
            if !code.is_code() {
                continue;
            }

            // Find corresponding phase
            let phase_code = SignalCode::new(ObservationType::Phase, code.band, code.attribute);
            let phase_value = match obs.get(&phase_code).map(|v| v.value) {
                Some(v) => v,
                None => continue,
            };

            // Get frequency
            let freq = match get_frequency(sys_char, code.band, fcn) {
                Some(f) => f,
                None => continue,
            };

            let lambda = SPEED_OF_LIGHT / freq;
            let phase_meters = phase_value * lambda;

            // Code-phase difference: dΦR = Φ - R
            let code_phase_diff = phase_meters - value.value;

            // Check against previous epoch
            if let Some(prev_cp) = prev.code_phase.get(code) {
                let cp_rate = (code_phase_diff - prev_cp) / dt;

                if cp_rate.abs() > self.code_phase_threshold {
                    slips.push(CycleSlip {
                        satellite: *sat,
                        epoch: *epoch,
                        method: CycleSlipMethod::CodePhase,
                        magnitude: cp_rate.abs(),
                        threshold: self.code_phase_threshold,
                        signals: vec![code.clone(), phase_code],
                    });
                }
            }
        }

        slips
    }

    /// Update previous epoch data
    fn update_prev_data(
        &mut self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
        epoch: &Epoch,
    ) {
        let sys_char = sat.system.to_char();
        let fcn = if sat.system == GnssSystem::Glonass { Some(0i8) } else { None };

        // Compute ionospheric combination
        let ion_combo = self.compute_ion_combo(sat, obs);

        // Compute code-phase differences
        let mut code_phase = HashMap::new();
        for (code, value) in obs {
            if !code.is_code() {
                continue;
            }

            let phase_code = SignalCode::new(ObservationType::Phase, code.band, code.attribute);
            if let Some(phase_value) = obs.get(&phase_code).map(|v| v.value) {
                if let Some(freq) = get_frequency(sys_char, code.band, fcn) {
                    let lambda = SPEED_OF_LIGHT / freq;
                    let cp_diff = phase_value * lambda - value.value;
                    code_phase.insert(code.clone(), cp_diff);
                }
            }
        }

        self.prev_data.insert(*sat, PrevEpochData {
            epoch: *epoch,
            ion_combo,
            code_phase,
        });
    }

    /// Compute ionospheric combination
    fn compute_ion_combo(
        &self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
    ) -> Option<f64> {
        let sys_char = sat.system.to_char();
        let fcn = if sat.system == GnssSystem::Glonass { Some(0i8) } else { None };

        // Find L1 and L2 phase
        let mut l1: Option<f64> = None;
        let mut l2: Option<f64> = None;
        let mut band2: u8 = 2;

        for (code, value) in obs {
            if code.is_phase() {
                if code.band == 1 {
                    l1 = Some(value.value);
                } else if code.band == 2 || code.band == 5 {
                    l2 = Some(value.value);
                    band2 = code.band;
                }
            }
        }

        let (l1_val, l2_val) = (l1?, l2?);
        let f1 = get_frequency(sys_char, 1, fcn)?;
        let f2 = get_frequency(sys_char, band2, fcn)?;

        let lambda1 = SPEED_OF_LIGHT / f1;
        let lambda2 = SPEED_OF_LIGHT / f2;
        let phi1 = l1_val * lambda1;
        let phi2 = l2_val * lambda2;

        let alpha = (f1 * f1) / (f2 * f2);
        Some((phi1 - phi2) / (alpha - 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let detector = CycleSlipDetector::new();
        assert!((detector.ion_threshold - thresholds::ION_RATE_THRESHOLD).abs() < 1e-6);
    }

    #[test]
    fn test_custom_thresholds() {
        let detector = CycleSlipDetector::with_thresholds(0.1, 10.0);
        assert!((detector.ion_threshold - 0.1).abs() < 1e-6);
        assert!((detector.code_phase_threshold - 10.0).abs() < 1e-6);
    }
}
