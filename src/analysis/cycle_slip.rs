//! Cycle slip detection algorithms.
//!
//! Implements cycle slip detection using:
//! - Geometry-free (ionospheric residual) combination, per phase signal code
//! - Code-phase combination
//! - Loss-of-lock indicator (LLI) flags on phase observables
//!
//! Thresholds operate in the delta domain and scale with the epoch spacing:
//! `|Δ| > base + rate·dt`, so the detector stays sensitive to single-cycle
//! slips at 30 s sampling while not over-triggering at 1 s.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::rinex::{
    GnssSystem, ObservationData, ObservationType,
    Satellite, SignalCode,
};
use crate::utils::{
    constants::{get_frequency, partner_bands, thresholds, SPEED_OF_LIGHT},
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
    /// Affected signal(s); the tested/affected phase code comes first
    pub signals: Vec<SignalCode>,
}

/// Cycle slip detection method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleSlipMethod {
    /// Geometry-free (ionospheric residual) combination
    IonosphericRate,
    /// Code-phase combination
    CodePhase,
    /// Loss of lock indicator (LLI)
    LLI,
    /// Doppler consistency
    Doppler,
}

impl CycleSlipMethod {
    /// Short string identifier for reports/APIs
    pub fn as_str(&self) -> &'static str {
        match self {
            CycleSlipMethod::IonosphericRate => "gf",
            CycleSlipMethod::CodePhase => "code_phase",
            CycleSlipMethod::LLI => "lli",
            CycleSlipMethod::Doppler => "doppler",
        }
    }
}

/// Delta-domain slip thresholds: a test triggers when `|Δ| > base + rate·dt`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SlipThresholds {
    /// Geometry-free base term (m)
    pub ion_delta_base: f64,
    /// Geometry-free rate term (m per second of epoch spacing)
    pub ion_delta_rate: f64,
    /// Code-phase base term (m)
    pub cp_delta_base: f64,
    /// Code-phase rate term (m per second of epoch spacing)
    pub cp_delta_rate: f64,
    /// Maximum epoch spacing for differential tests (s)
    pub max_dt: f64,
    /// Honor LLI flags on phase observables
    pub use_lli: bool,
}

impl Default for SlipThresholds {
    fn default() -> Self {
        Self {
            ion_delta_base: thresholds::ION_DELTA_BASE,
            ion_delta_rate: thresholds::ION_DELTA_RATE,
            cp_delta_base: thresholds::CP_DELTA_BASE,
            cp_delta_rate: thresholds::CP_DELTA_RATE,
            max_dt: thresholds::MAX_SLIP_DT,
            use_lli: true,
        }
    }
}

/// Summary of a detection run, indexed for downstream arc-splitting
#[derive(Debug, Clone, Default)]
pub struct SlipSummary {
    /// All detected slips, sorted by (epoch, satellite, first signal)
    pub slips: Vec<CycleSlip>,
    /// Slip counts per (system, phase signal code), e.g. (Gps, L2W) → 3
    pub per_signal: HashMap<(GnssSystem, SignalCode), usize>,
    /// Epochs (unix ms) with a slip per (satellite, phase signal code)
    pub epoch_index: HashMap<(Satellite, SignalCode), HashSet<i64>>,
}

impl SlipSummary {
    /// True if (sat, phase code) has a slip at the given epoch
    pub fn has_slip(&self, sat: &Satellite, code: &SignalCode, epoch_ms: i64) -> bool {
        self.epoch_index
            .get(&(*sat, code.clone()))
            .map(|s| s.contains(&epoch_ms))
            .unwrap_or(false)
    }
}

/// Cycle slip detector
pub struct CycleSlipDetector {
    thresholds: SlipThresholds,
    /// GLONASS frequency channel numbers by slot/PRN (from RINEX header)
    glonass_fcn: HashMap<u32, i8>,
    /// Previous epoch data for differencing
    prev_data: HashMap<Satellite, PrevEpochData>,
}

/// Data from previous epoch for differencing
struct PrevEpochData {
    epoch: Epoch,
    /// Geometry-free value per tested phase code: code → (reference code, GF meters)
    gf: HashMap<SignalCode, (SignalCode, f64)>,
    /// Code-phase difference per code observable (m)
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
            thresholds: SlipThresholds::default(),
            glonass_fcn: HashMap::new(),
            prev_data: HashMap::new(),
        }
    }

    /// Create with custom legacy rate thresholds (m/s), mapped onto the
    /// delta-domain base terms. Prefer `with_config` for new code.
    pub fn with_thresholds(ion_threshold: f64, code_phase_threshold: f64) -> Self {
        let mut t = SlipThresholds::default();
        if ion_threshold > 0.0 {
            t.ion_delta_base = ion_threshold;
        }
        if code_phase_threshold > 0.0 {
            t.cp_delta_base = code_phase_threshold;
        }
        Self {
            thresholds: t,
            glonass_fcn: HashMap::new(),
            prev_data: HashMap::new(),
        }
    }

    /// Create with full configuration
    pub fn with_config(thresholds: SlipThresholds, glonass_fcn: HashMap<u32, i8>) -> Self {
        Self {
            thresholds,
            glonass_fcn,
            prev_data: HashMap::new(),
        }
    }

    fn fcn_for(&self, sat: &Satellite) -> Option<i8> {
        if sat.system == GnssSystem::Glonass {
            Some(self.glonass_fcn.get(&sat.prn).copied().unwrap_or(0))
        } else {
            None
        }
    }

    /// Detect cycle slips in observation data
    pub fn detect(&mut self, obs_data: &ObservationData) -> Vec<CycleSlip> {
        let mut slips = Vec::new();

        // Reset state
        self.prev_data.clear();

        // Pull GLONASS FCN map from the header if not externally provided
        if self.glonass_fcn.is_empty() {
            self.glonass_fcn = obs_data.header.glonass_slot_frq.clone();
        }

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

        slips.sort_by(|a, b| {
            a.epoch
                .to_unix_ms()
                .cmp(&b.epoch.to_unix_ms())
                .then_with(|| a.satellite.to_string().cmp(&b.satellite.to_string()))
                .then_with(|| {
                    let sa = a.signals.first().map(|s| s.to_string()).unwrap_or_default();
                    let sb = b.signals.first().map(|s| s.to_string()).unwrap_or_default();
                    sa.cmp(&sb)
                })
        });
        slips
    }

    /// Run detection and build the per-signal summary/index
    pub fn detect_summary(&mut self, obs_data: &ObservationData) -> SlipSummary {
        let slips = self.detect(obs_data);
        let mut per_signal: HashMap<(GnssSystem, SignalCode), usize> = HashMap::new();
        let mut epoch_index: HashMap<(Satellite, SignalCode), HashSet<i64>> = HashMap::new();

        for slip in &slips {
            let epoch_ms = slip.epoch.to_unix_ms();
            let mut counted = false;
            for sig in &slip.signals {
                if !sig.is_phase() {
                    continue;
                }
                epoch_index
                    .entry((slip.satellite, sig.clone()))
                    .or_default()
                    .insert(epoch_ms);
                if !counted {
                    // Attribute the slip to the first (tested) phase signal only
                    *per_signal
                        .entry((slip.satellite.system, sig.clone()))
                        .or_insert(0) += 1;
                    counted = true;
                }
            }
        }

        SlipSummary { slips, per_signal, epoch_index }
    }

    /// Detect cycle slips for a single satellite at one epoch
    fn detect_for_satellite(
        &self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
        epoch: &Epoch,
    ) -> Vec<CycleSlip> {
        let mut slips = Vec::new();

        // Check LLI flags — defined for phase observables only
        if self.thresholds.use_lli {
            for (code, value) in obs {
                if !code.is_phase() {
                    continue;
                }
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
        }

        // Get previous epoch data
        let prev = match self.prev_data.get(sat) {
            Some(p) => p,
            None => return slips,
        };

        // Time difference
        let dt = epoch.diff(&prev.epoch);
        if dt.abs() < 0.001 || dt.abs() > self.thresholds.max_dt {
            // Same epoch, or a gap long enough that the arc breaks anyway
            return slips;
        }

        // Geometry-free check per phase signal code
        slips.extend(self.check_geometry_free(sat, obs, epoch, prev, dt));

        // Code-phase combination check
        slips.extend(self.check_code_phase(sat, obs, epoch, prev, dt));

        slips
    }

    /// Pick the reference phase for a tested phase code: the alphabetically
    /// first phase on the highest-priority partner band ≠ the tested band
    /// with adequate frequency separation.
    fn reference_phase(
        &self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
        tested: &SignalCode,
    ) -> Option<SignalCode> {
        let sys_char = sat.system.to_char();
        let fcn = self.fcn_for(sat);
        let f_tested = get_frequency(sys_char, tested.band, fcn)?;

        for &band in partner_bands(sys_char) {
            if band == tested.band {
                continue;
            }
            let f_ref = match get_frequency(sys_char, band, fcn) {
                Some(f) => f,
                None => continue,
            };
            let alpha = (f_tested * f_tested) / (f_ref * f_ref);
            if (alpha - 1.0).abs() < thresholds::MIN_ALPHA_SEPARATION {
                continue;
            }
            // Alphabetically first phase attribute on this band
            let mut candidates: Vec<&SignalCode> = obs
                .keys()
                .filter(|c| c.is_phase() && c.band == band)
                .collect();
            candidates.sort_by_key(|c| c.attribute);
            if let Some(code) = candidates.first() {
                return Some((*code).clone());
            }
        }
        None
    }

    /// Geometry-free (ionospheric residual) value in meters for a phase pair
    fn gf_value(
        &self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
        tested: &SignalCode,
        reference: &SignalCode,
    ) -> Option<f64> {
        let sys_char = sat.system.to_char();
        let fcn = self.fcn_for(sat);
        let f1 = get_frequency(sys_char, tested.band, fcn)?;
        let f2 = get_frequency(sys_char, reference.band, fcn)?;
        let l1 = obs.get(tested)?.value;
        let l2 = obs.get(reference)?.value;
        let phi1 = l1 * SPEED_OF_LIGHT / f1;
        let phi2 = l2 * SPEED_OF_LIGHT / f2;
        let alpha = (f1 * f1) / (f2 * f2);
        Some((phi1 - phi2) / (alpha - 1.0))
    }

    /// Check geometry-free combination per phase signal code
    fn check_geometry_free(
        &self,
        sat: &Satellite,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
        epoch: &Epoch,
        prev: &PrevEpochData,
        dt: f64,
    ) -> Vec<CycleSlip> {
        let mut slips = Vec::new();
        let threshold =
            self.thresholds.ion_delta_base + self.thresholds.ion_delta_rate * dt.abs();

        let mut phase_codes: Vec<&SignalCode> =
            obs.keys().filter(|c| c.is_phase()).collect();
        phase_codes.sort_by_key(|c| (c.band, c.attribute));

        for code in phase_codes {
            let reference = match self.reference_phase(sat, obs, code) {
                Some(r) => r,
                None => continue,
            };
            let gf = match self.gf_value(sat, obs, code, &reference) {
                Some(v) => v,
                None => continue,
            };
            // Only difference against the previous epoch when the same
            // reference was used — otherwise ΔGF is meaningless
            if let Some((prev_ref, prev_gf)) = prev.gf.get(code) {
                if *prev_ref != reference {
                    continue;
                }
                let delta = gf - prev_gf;
                if delta.abs() > threshold {
                    slips.push(CycleSlip {
                        satellite: *sat,
                        epoch: *epoch,
                        method: CycleSlipMethod::IonosphericRate,
                        magnitude: delta.abs(),
                        threshold,
                        signals: vec![code.clone(), reference],
                    });
                }
            }
        }

        slips
    }

    /// Check code-phase combination (delta domain)
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
        let fcn = self.fcn_for(sat);
        let threshold =
            self.thresholds.cp_delta_base + self.thresholds.cp_delta_rate * dt.abs();

        let mut code_obs: Vec<(&SignalCode, &crate::rinex::ObservationValue)> =
            obs.iter().filter(|(c, _)| c.is_code()).collect();
        code_obs.sort_by_key(|(c, _)| (c.band, c.attribute));

        for (code, value) in code_obs {
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
                let delta = code_phase_diff - prev_cp;

                if delta.abs() > threshold {
                    slips.push(CycleSlip {
                        satellite: *sat,
                        epoch: *epoch,
                        method: CycleSlipMethod::CodePhase,
                        magnitude: delta.abs(),
                        threshold,
                        signals: vec![phase_code, code.clone()],
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
        let fcn = self.fcn_for(sat);

        // Geometry-free per phase code
        let mut gf = HashMap::new();
        for code in obs.keys().filter(|c| c.is_phase()) {
            if let Some(reference) = self.reference_phase(sat, obs, code) {
                if let Some(v) = self.gf_value(sat, obs, code, &reference) {
                    gf.insert(code.clone(), (reference, v));
                }
            }
        }

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
            gf,
            code_phase,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rinex::{EpochObservations, Header, ObservationValue};
    use crate::utils::constants::frequencies;

    fn make_epoch(sec_of_day: f64) -> Epoch {
        let h = (sec_of_day / 3600.0) as u32;
        let m = ((sec_of_day % 3600.0) / 60.0) as u32;
        let s = sec_of_day % 60.0;
        Epoch::new(2026, 4, 8, h, m, s)
    }

    /// Build GPS obs data at a given interval with L1C/L2W phases (cycles)
    /// and C1C code; injects a `slip_cycles` jump on L1C at epoch `slip_at`.
    fn synthetic_gps(interval: f64, n: usize, slip_at: Option<usize>, slip_cycles: f64) -> ObservationData {
        let sat = Satellite::parse("G01").unwrap();
        let l1c = SignalCode::parse("L1C").unwrap();
        let l2w = SignalCode::parse("L2W").unwrap();
        let c1c = SignalCode::parse("C1C").unwrap();
        let lambda1 = SPEED_OF_LIGHT / frequencies::gps::L1;
        let lambda2 = SPEED_OF_LIGHT / frequencies::gps::L2;

        let mut data = ObservationData::new(Header::default());
        let range0 = 22_000_000.0_f64;
        for i in 0..n {
            let t = i as f64 * interval;
            // Smooth geometry change + tiny iono drift
            let range = range0 + 50.0 * t;
            let iono_l1 = 2.0 + 0.0005 * t;
            let mut l1_cycles = (range + iono_l1) / lambda1;
            let l2_cycles = (range + iono_l1 * (frequencies::gps::ALPHA_L1_L2)) / lambda2;
            if let Some(k) = slip_at {
                if i >= k {
                    l1_cycles += slip_cycles;
                }
            }
            let mut eo = EpochObservations::new(make_epoch(t));
            let mut obs = HashMap::new();
            obs.insert(l1c.clone(), ObservationValue::new(l1_cycles));
            obs.insert(l2w.clone(), ObservationValue::new(l2_cycles));
            obs.insert(c1c.clone(), ObservationValue::new(range + iono_l1));
            eo.satellites.insert(sat, obs);
            data.epochs.push(eo);
        }
        data
    }

    #[test]
    fn test_detector_creation() {
        let detector = CycleSlipDetector::new();
        assert!((detector.thresholds.ion_delta_base - thresholds::ION_DELTA_BASE).abs() < 1e-9);
    }

    #[test]
    fn test_custom_thresholds_legacy_mapping() {
        let detector = CycleSlipDetector::with_thresholds(0.1, 10.0);
        assert!((detector.thresholds.ion_delta_base - 0.1).abs() < 1e-9);
        assert!((detector.thresholds.cp_delta_base - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_single_cycle_slip_detected_at_30s() {
        // A 1-cycle L1 slip has a GF signature of λ1/(α−1) ≈ 0.294 m,
        // above the 30 s threshold 0.10 + 0.003·30 = 0.19 m.
        let data = synthetic_gps(30.0, 20, Some(10), 1.0);
        let mut det = CycleSlipDetector::new();
        let summary = det.detect_summary(&data);
        let l1c = SignalCode::parse("L1C").unwrap();
        assert!(
            summary.per_signal.get(&(GnssSystem::Gps, l1c.clone())).copied().unwrap_or(0) >= 1,
            "1-cycle L1 slip must be detected at 30 s sampling: {:?}",
            summary.per_signal
        );
        // And the epoch index must flag the slip epoch for (G01, L1C)
        let sat = Satellite::parse("G01").unwrap();
        let slip_ms = data.epochs[10].epoch.to_unix_ms();
        assert!(summary.has_slip(&sat, &l1c, slip_ms));
    }

    #[test]
    fn test_no_false_positives_clean_data() {
        for &dt in &[1.0, 30.0] {
            let data = synthetic_gps(dt, 40, None, 0.0);
            let mut det = CycleSlipDetector::new();
            let summary = det.detect_summary(&data);
            assert!(
                summary.slips.is_empty(),
                "clean data at {dt}s produced slips: {:?}",
                summary.slips
            );
        }
    }

    #[test]
    fn test_legacy_rate_logic_would_miss_30s_slip() {
        // Documents the 0.1.x defect: rate = 0.294 m / 30 s ≈ 0.0098 m/s
        // was far below the 0.0667 m/s rate threshold.
        let gf_signature = (SPEED_OF_LIGHT / frequencies::gps::L1)
            / (frequencies::gps::ALPHA_L1_L2 - 1.0);
        let rate_at_30s = gf_signature / 30.0;
        assert!(rate_at_30s < thresholds::ION_RATE_THRESHOLD);
        // New delta-domain threshold at 30 s catches it
        let delta_threshold = thresholds::ION_DELTA_BASE + thresholds::ION_DELTA_RATE * 30.0;
        assert!(gf_signature > delta_threshold);
    }

    #[test]
    fn test_lli_only_on_phase() {
        let sat = Satellite::parse("G01").unwrap();
        let mut eo = EpochObservations::new(make_epoch(0.0));
        let mut obs = HashMap::new();
        // LLI bit set on an SNR observable must NOT fire
        obs.insert(
            SignalCode::parse("S1C").unwrap(),
            ObservationValue::with_flags(45.0, Some(1), None),
        );
        eo.satellites.insert(sat, obs);
        let mut data = ObservationData::new(Header::default());
        data.epochs.push(eo);

        let mut det = CycleSlipDetector::new();
        assert!(det.detect(&data).is_empty());
    }
}
