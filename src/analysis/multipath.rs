//! Multipath analysis core functionality.
//!
//! Implements Anubis/TEQC-style code multipath estimation: for EVERY
//! pseudorange code observable present (C1C, C1X, C2W, ...), a dual-frequency
//! linear combination is formed with two carrier phases,
//!
//!   MP_k = P_k − (1 + 2/(α−1))·Φ_i + (2/(α−1))·Φ_j,   α = (f_i/f_j)²
//!
//! where Φ_i is the phase on the code's own band and Φ_j the phase on a
//! partner band. The phase-ambiguity/hardware bias is removed per
//! continuous arc (mean or moving average), with arcs reset at cycle slips
//! and data gaps. Statistics are reported per signal code in the style of
//! Hunegnaw & Teferle (2022): `GPSM1C`, `GLOM3X`, `BDSM2I`, ...

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::rinex::{
    GnssSystem, ObservationData, ObservationType,
    Satellite, SignalCode,
};
use crate::navigation::SatellitePosition;
use crate::utils::{
    constants::{alpha_factor, get_frequency, partner_bands, thresholds, SPEED_OF_LIGHT},
    Ecef, Epoch, Result,
};

use super::cycle_slip::{CycleSlip, CycleSlipDetector, SlipSummary, SlipThresholds};

/// Multipath estimate for a single observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipathEstimate {
    /// Satellite
    pub satellite: Satellite,
    /// Epoch
    pub epoch: Epoch,
    /// Code observable this MP belongs to (e.g., "C1C")
    pub signal: String,
    /// Primary signal code (kept for serde compatibility; equals `signal`)
    pub primary_code: String,
    /// Secondary signal code (partner-band code equivalent, e.g., "C2W")
    pub secondary_code: String,
    /// Phase observable on the code's own band (e.g., "L1C")
    #[serde(default)]
    pub phase1_code: String,
    /// Phase observable on the partner band (e.g., "L2W")
    #[serde(default)]
    pub phase2_code: String,
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
    /// Elevation cutoff angle (degrees). Applied only when real elevations
    /// are available at analysis time (via `with_positions`); estimates get
    /// a 45° placeholder otherwise so the cutoff does not drop everything.
    pub elevation_cutoff: f64,
    /// Systems to include
    pub systems: Vec<GnssSystem>,
    /// Legacy explicit signal pairs; the primary codes act as an include
    /// filter in the per-code engine
    pub signal_pairs: Vec<(SignalCode, SignalCode)>,
    /// Legacy ionospheric rate threshold (m/s); feeds slip.ion_delta_base
    pub ion_threshold: f64,
    /// Legacy code-phase threshold (m/s); feeds slip.cp_delta_base
    pub code_phase_threshold: f64,
    /// Include SNR analysis
    pub include_snr: bool,
    /// Weight by elevation
    pub elevation_weighting: bool,
    /// Moving-average bias window (s). None → whole-arc mean (Anubis style)
    #[serde(default = "default_bias_window")]
    pub bias_window_seconds: Option<f64>,
    /// Minimum arc duration to keep (s)
    #[serde(default = "default_min_arc")]
    pub min_arc_seconds: f64,
    /// Arc break when epoch gap exceeds this multiple of the interval
    #[serde(default = "default_arc_gap_factor")]
    pub arc_gap_factor: f64,
    /// MP jump magnitude that forces an arc break (m)
    #[serde(default = "default_mp_jump")]
    pub mp_jump_threshold: f64,
    /// Only analyze these code observables ("C1C" for all systems, or
    /// "GC1C" to restrict to one system). Empty → all codes.
    #[serde(default)]
    pub include_codes: Vec<String>,
    /// Skip these code observables (same format as include_codes)
    #[serde(default)]
    pub exclude_codes: Vec<String>,
    /// Uniformly decimate files with more epochs than this before analysis
    #[serde(default)]
    pub max_epochs: Option<usize>,
    /// Run cycle-slip detection and reset arcs at slips
    #[serde(default = "default_true")]
    pub detect_cycle_slips: bool,
    /// Delta-domain slip thresholds
    #[serde(default)]
    pub slip: SlipThresholds,
}

fn default_bias_window() -> Option<f64> {
    Some(thresholds::BIAS_WINDOW_SECONDS)
}
fn default_min_arc() -> f64 {
    thresholds::MIN_ARC_SECONDS
}
fn default_arc_gap_factor() -> f64 {
    thresholds::ARC_GAP_FACTOR
}
fn default_mp_jump() -> f64 {
    thresholds::MP_JUMP_THRESHOLD
}
fn default_true() -> bool {
    true
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
            bias_window_seconds: default_bias_window(),
            min_arc_seconds: default_min_arc(),
            arc_gap_factor: default_arc_gap_factor(),
            mp_jump_threshold: default_mp_jump(),
            include_codes: Vec::new(),
            exclude_codes: Vec::new(),
            max_epochs: None,
            detect_cycle_slips: true,
            slip: SlipThresholds::default(),
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

    /// Add a signal pair for multipath analysis (legacy; acts as include filter)
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
    /// Number of cycle slips that broke arcs on this signal's phase pair
    pub cycle_slips: usize,
    /// System identifier (e.g., "G")
    #[serde(default)]
    pub system: String,
    /// Code observable (e.g., "C1C")
    #[serde(default)]
    pub code: String,
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
            system: estimates
                .first()
                .map(|e| e.satellite.system.to_char().to_string())
                .unwrap_or_default(),
            code: estimates.first().map(|e| e.signal.clone()).unwrap_or_default(),
        }
    }
}

/// The chosen combination for one (satellite, code observable)
struct ComboSpec {
    satellite: Satellite,
    code: SignalCode,
    phase1: SignalCode,
    phase2: SignalCode,
    snr_code: Option<SignalCode>,
    lambda1: f64,
    lambda2: f64,
    alpha: f64,
}

/// Raw (biased) MP series for one combo before arc splitting
struct RawSeries {
    spec_idx: usize,
    t_ms: Vec<i64>,
    epochs: Vec<Epoch>,
    mp: Vec<f64>,
    snr: Vec<Option<f64>>,
}

/// Multipath analyzer
pub struct MultipathAnalyzer {
    /// Configuration
    config: AnalysisConfig,
    /// Observation data (shared, not cloned)
    obs_data: Arc<ObservationData>,
    /// Satellite positions (precomputed)
    positions: HashMap<(Satellite, Epoch), SatellitePosition>,
    /// Receiver position
    receiver_pos: Option<Ecef>,
    /// GLONASS frequency channel map
    glonass_fcn: HashMap<u32, i8>,
}

impl MultipathAnalyzer {
    /// Create a new analyzer. Accepts owned data or an `Arc` (no deep copy).
    pub fn new(obs_data: impl Into<Arc<ObservationData>>, config: AnalysisConfig) -> Self {
        let obs_data = obs_data.into();
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

    /// Does a code observable pass the include/exclude filters?
    fn code_allowed(&self, sys_char: char, code: &SignalCode, include: &[String], exclude: &[String]) -> bool {
        let plain = code.to_string();
        let scoped = format!("{}{}", sys_char, plain);
        let matches = |list: &[String]| {
            list.iter().any(|e| {
                let e = e.to_ascii_uppercase();
                e == plain || e == scoped
            })
        };
        if matches(exclude) {
            return false;
        }
        if include.is_empty() {
            return true;
        }
        matches(include)
    }

    /// Signal display name, e.g. GPSM1C
    fn signal_name(system: GnssSystem, code: &SignalCode) -> String {
        format!("{}M{}{}", system.abbrev(), code.band, code.attribute)
    }

    /// Choose the phase pair for a (satellite, code): Φ1 on the code's own
    /// band (same attribute → highest count → alphabetical), Φ2 on the first
    /// partner band with data and adequate frequency separation.
    fn select_phases(
        &self,
        sat: &Satellite,
        code: &SignalCode,
        counts: &HashMap<(Satellite, SignalCode), usize>,
        codes_of_sat: &[SignalCode],
    ) -> Option<(SignalCode, SignalCode, f64, f64)> {
        let sys_char = sat.system.to_char();
        let fcn = if sat.system == GnssSystem::Glonass {
            Some(self.glonass_fcn.get(&sat.prn).copied().unwrap_or(0))
        } else {
            None
        };

        let f1 = get_frequency(sys_char, code.band, fcn)?;

        // Φ1 candidates on own band
        let mut own: Vec<&SignalCode> = codes_of_sat
            .iter()
            .filter(|c| c.is_phase() && c.band == code.band)
            .collect();
        if own.is_empty() {
            return None;
        }
        own.sort_by_key(|c| {
            let same_attr = if c.attribute == code.attribute { 0u8 } else { 1u8 };
            let count = counts.get(&(*sat, (*c).clone())).copied().unwrap_or(0);
            (same_attr, usize::MAX - count, c.attribute)
        });
        let phase1 = own[0].clone();

        // Φ2 from partner priority
        for &band in partner_bands(sys_char) {
            if band == code.band {
                continue;
            }
            let f2 = match get_frequency(sys_char, band, fcn) {
                Some(f) => f,
                None => continue,
            };
            let alpha = alpha_factor(f1, f2);
            if (alpha - 1.0).abs() < thresholds::MIN_ALPHA_SEPARATION {
                continue;
            }
            let mut cands: Vec<&SignalCode> = codes_of_sat
                .iter()
                .filter(|c| c.is_phase() && c.band == band)
                .collect();
            if cands.is_empty() {
                continue;
            }
            cands.sort_by_key(|c| {
                let count = counts.get(&(*sat, (*c).clone())).copied().unwrap_or(0);
                (usize::MAX - count, c.attribute)
            });
            return Some((phase1, cands[0].clone(), f1, f2));
        }
        None
    }

    /// Analyze multipath for all code observables
    pub fn analyze(&self) -> Result<AnalysisResults> {
        // Optional decimation for oversized files
        let decimated_storage;
        let data: &ObservationData = match self.config.max_epochs {
            Some(m) if m > 0 && self.obs_data.epochs.len() > m => {
                let stride = self.obs_data.epochs.len().div_ceil(m);
                let mut d = ObservationData::new(self.obs_data.header.clone());
                d.epochs = self
                    .obs_data
                    .epochs
                    .iter()
                    .step_by(stride)
                    .cloned()
                    .collect();
                decimated_storage = d;
                &decimated_storage
            }
            _ => self.obs_data.as_ref(),
        };

        let interval = data.sampling_interval().unwrap_or(30.0).max(0.001);
        let gap_break = (self.config.arc_gap_factor * interval).max(interval + 0.001);

        // Cycle-slip detection feeds arc splitting and per-signal counts
        let slip_summary = if self.config.detect_cycle_slips {
            let mut detector =
                CycleSlipDetector::with_config(self.config.slip, self.glonass_fcn.clone());
            detector.detect_summary(data)
        } else {
            SlipSummary::default()
        };

        // Legacy signal_pairs act as an include filter
        let mut include = self.config.include_codes.clone();
        for (p, _) in &self.config.signal_pairs {
            include.push(p.to_string());
        }
        let exclude = self.config.exclude_codes.clone();

        // Pre-scan: per-satellite code inventory and observation counts
        let counts = data.observed_code_counts();
        let mut codes_by_sat: HashMap<Satellite, Vec<SignalCode>> = HashMap::new();
        for (sat, code) in counts.keys() {
            codes_by_sat.entry(*sat).or_default().push(code.clone());
        }

        // Build combo registry
        let mut specs: Vec<ComboSpec> = Vec::new();
        let mut sats: Vec<&Satellite> = codes_by_sat.keys().collect();
        sats.sort_by_key(|s| (s.system.to_char(), s.prn));

        for sat in sats {
            if !self.config.systems.contains(&sat.system) {
                continue;
            }
            let sys_char = sat.system.to_char();
            let codes_of_sat = &codes_by_sat[sat];

            let mut code_obs: Vec<&SignalCode> =
                codes_of_sat.iter().filter(|c| c.is_code()).collect();
            code_obs.sort_by_key(|c| (c.band, c.attribute));

            for code in code_obs {
                if !self.code_allowed(sys_char, code, &include, &exclude) {
                    continue;
                }
                let (phase1, phase2, f1, f2) =
                    match self.select_phases(sat, code, &counts, codes_of_sat) {
                        Some(sel) => sel,
                        None => continue,
                    };

                // SNR: matching attribute on own band, else any S on the band
                let snr_code = if self.config.include_snr {
                    let exact = SignalCode::new(ObservationType::Snr, code.band, code.attribute);
                    if counts.contains_key(&(*sat, exact.clone())) {
                        Some(exact)
                    } else {
                        let mut s: Vec<&SignalCode> = codes_of_sat
                            .iter()
                            .filter(|c| c.is_snr() && c.band == code.band)
                            .collect();
                        s.sort_by_key(|c| c.attribute);
                        s.first().map(|c| (*c).clone())
                    }
                } else {
                    None
                };

                specs.push(ComboSpec {
                    satellite: *sat,
                    code: code.clone(),
                    phase1,
                    phase2,
                    snr_code,
                    lambda1: SPEED_OF_LIGHT / f1,
                    lambda2: SPEED_OF_LIGHT / f2,
                    alpha: alpha_factor(f1, f2),
                });
            }
        }

        // Single epoch-major pass building all raw series
        let mut spec_lookup: HashMap<(Satellite, SignalCode), usize> = HashMap::new();
        for (i, s) in specs.iter().enumerate() {
            spec_lookup.insert((s.satellite, s.code.clone()), i);
        }
        let mut series: Vec<RawSeries> = specs
            .iter()
            .enumerate()
            .map(|(i, _)| RawSeries {
                spec_idx: i,
                t_ms: Vec::new(),
                epochs: Vec::new(),
                mp: Vec::new(),
                snr: Vec::new(),
            })
            .collect();

        for epoch_obs in &data.epochs {
            let t_ms = epoch_obs.epoch.to_unix_ms();
            for (sat, sat_obs) in &epoch_obs.satellites {
                for (code, value) in sat_obs.iter().filter(|(c, _)| c.is_code()) {
                    let idx = match spec_lookup.get(&(*sat, code.clone())) {
                        Some(i) => *i,
                        None => continue,
                    };
                    let spec = &specs[idx];
                    let l1 = match sat_obs.get(&spec.phase1) {
                        Some(v) => v.value,
                        None => continue,
                    };
                    let l2 = match sat_obs.get(&spec.phase2) {
                        Some(v) => v.value,
                        None => continue,
                    };

                    let phi1 = l1 * spec.lambda1;
                    let phi2 = l2 * spec.lambda2;
                    let a = spec.alpha;
                    // MP = P − (1 + 2/(α−1))·Φ1 + (2/(α−1))·Φ2
                    let mp = value.value - (1.0 + 2.0 / (a - 1.0)) * phi1 + (2.0 / (a - 1.0)) * phi2;
                    if !mp.is_finite() || mp.abs() > 1.0e7 {
                        continue;
                    }

                    let snr = spec
                        .snr_code
                        .as_ref()
                        .and_then(|c| sat_obs.get(c))
                        .map(|v| v.value);

                    let s = &mut series[idx];
                    s.t_ms.push(t_ms);
                    s.epochs.push(epoch_obs.epoch);
                    s.mp.push(mp);
                    s.snr.push(snr);
                }
            }
        }

        // Arc splitting + debiasing + estimate emission
        let min_epochs = 10usize;
        let mut results = AnalysisResults::default();
        let mut slip_breaks_per_signal: HashMap<String, usize> = HashMap::new();

        for s in &series {
            if s.t_ms.is_empty() {
                continue;
            }
            let spec = &specs[s.spec_idx];
            let name = Self::signal_name(spec.satellite.system, &spec.code);

            // Arc boundaries
            let mut arcs: Vec<(usize, usize)> = Vec::new(); // [start, end)
            let mut start = 0usize;
            for i in 1..s.t_ms.len() {
                let dt = (s.t_ms[i] - s.t_ms[i - 1]) as f64 / 1000.0;
                let slip_here = slip_summary.has_slip(&spec.satellite, &spec.phase1, s.t_ms[i])
                    || slip_summary.has_slip(&spec.satellite, &spec.phase2, s.t_ms[i]);
                let jump = (s.mp[i] - s.mp[i - 1]).abs() > self.config.mp_jump_threshold;
                if dt > gap_break || slip_here || jump {
                    if slip_here {
                        *slip_breaks_per_signal.entry(name.clone()).or_insert(0) += 1;
                    }
                    arcs.push((start, i));
                    start = i;
                }
            }
            arcs.push((start, s.t_ms.len()));

            let sys_char = spec.satellite.system.to_char().to_string();
            let estimates = results.estimates.entry(name.clone()).or_default();

            for (a0, a1) in arcs {
                let n = a1 - a0;
                if n < min_epochs {
                    continue;
                }
                let duration = (s.t_ms[a1 - 1] - s.t_ms[a0]) as f64 / 1000.0;
                if duration < self.config.min_arc_seconds {
                    continue;
                }

                let debiased = debias_arc(
                    &s.t_ms[a0..a1],
                    &s.mp[a0..a1],
                    self.config.bias_window_seconds,
                );

                for (k, mp) in debiased.into_iter().enumerate() {
                    let i = a0 + k;
                    let (elevation, azimuth) = self.get_azel(&spec.satellite, &s.epochs[i]);
                    if elevation < self.config.elevation_cutoff {
                        continue;
                    }
                    estimates.push(MultipathEstimate {
                        satellite: spec.satellite,
                        epoch: s.epochs[i],
                        signal: spec.code.to_string(),
                        primary_code: spec.code.to_string(),
                        secondary_code: format!("C{}{}", spec.phase2.band, spec.phase2.attribute),
                        phase1_code: spec.phase1.to_string(),
                        phase2_code: spec.phase2.to_string(),
                        mp_value: mp,
                        elevation,
                        azimuth,
                        snr: s.snr[i],
                    });
                }
            }

            if estimates.is_empty() {
                results.estimates.remove(&name);
            } else {
                // Keep map entry; statistics computed below
                let _ = sys_char;
            }
        }

        // Per-signal statistics
        for (name, ests) in &results.estimates {
            let mut stats = MultipathStatistics::compute(ests, self.config.elevation_weighting);
            stats.cycle_slips = slip_breaks_per_signal.get(name).copied().unwrap_or(0);
            results.statistics.insert(name.clone(), stats);
        }

        // Cycle-slip outputs
        results.cycle_slip_counts = slip_summary
            .per_signal
            .iter()
            .map(|((sys, code), n)| (format!("{}{}", sys.abbrev(), code), *n))
            .collect();
        results.cycle_slips = slip_summary.slips;

        // Compute summary statistics
        results.compute_summary();

        Ok(results)
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

        // Placeholder when position unknown (real elevations can be attached
        // afterwards, e.g. from SP3); 45° passes typical cutoffs
        (45.0, 0.0)
    }
}

/// Remove the phase-ambiguity bias from one arc: whole-arc mean when the arc
/// fits within the window (or window is None), otherwise a centered
/// moving-average of the given duration, computed in O(n) with two pointers.
fn debias_arc(t_ms: &[i64], mp: &[f64], window_seconds: Option<f64>) -> Vec<f64> {
    let n = mp.len();
    if n == 0 {
        return Vec::new();
    }
    let duration = (t_ms[n - 1] - t_ms[0]) as f64 / 1000.0;

    let window = match window_seconds {
        Some(w) if w > 0.0 && duration > w => w,
        _ => {
            let mean = mp.iter().sum::<f64>() / n as f64;
            return mp.iter().map(|v| v - mean).collect();
        }
    };

    let half_ms = (window * 500.0) as i64; // window/2 in ms
    let mut prefix = Vec::with_capacity(n + 1);
    prefix.push(0.0);
    for v in mp {
        prefix.push(prefix.last().unwrap() + v);
    }

    let mut out = Vec::with_capacity(n);
    let mut lo = 0usize;
    let mut hi = 0usize;
    for i in 0..n {
        let t = t_ms[i];
        while lo < n && t_ms[lo] < t - half_ms {
            lo += 1;
        }
        if hi < i + 1 {
            hi = i + 1;
        }
        while hi < n && t_ms[hi] <= t + half_ms {
            hi += 1;
        }
        let count = (hi - lo) as f64;
        let local_mean = (prefix[hi] - prefix[lo]) / count;
        out.push(mp[i] - local_mean);
    }
    out
}

/// Analysis results container
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisResults {
    /// Multipath estimates per signal (key: e.g. "GPSM1C")
    pub estimates: HashMap<String, Vec<MultipathEstimate>>,
    /// Statistics per signal
    pub statistics: HashMap<String, MultipathStatistics>,
    /// Summary statistics
    pub summary: SummaryStatistics,
    /// All detected cycle slips
    #[serde(default)]
    pub cycle_slips: Vec<CycleSlip>,
    /// Slip counts per phase signal (key: e.g. "GPSL2W")
    #[serde(default)]
    pub cycle_slip_counts: HashMap<String, usize>,
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

        self.summary = SummaryStatistics {
            total_estimates,
            num_signals: self.statistics.len(),
            average_rms: avg_rms,
            total_cycle_slips: self.cycle_slips.len(),
        };
    }

    /// Recompute per-signal statistics (e.g. after elevations were attached)
    pub fn recompute_statistics(&mut self, elevation_weighting: bool) {
        let slip_counts: HashMap<String, usize> = self
            .statistics
            .iter()
            .map(|(k, v)| (k.clone(), v.cycle_slips))
            .collect();
        for (name, ests) in &self.estimates {
            let mut stats = MultipathStatistics::compute(ests, elevation_weighting);
            stats.cycle_slips = slip_counts.get(name).copied().unwrap_or(0);
            self.statistics.insert(name.clone(), stats);
        }
        self.compute_summary();
    }

    /// Export to CSV
    pub fn to_csv(&self, path: &str) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        // Header
        writeln!(file, "Signal,Count,Mean,StdDev,RMS,WeightedRMS,Min,Max,CycleSlips")?;

        // Data (sorted for determinism)
        let mut keys: Vec<&String> = self.statistics.keys().collect();
        keys.sort();
        for signal in keys {
            let stats = &self.statistics[signal];
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

    /// GPS L1/L2 data with a known sinusoidal MP signal on C1C plus a large
    /// constant ambiguity bias; MP2-relevant codes included.
    fn synthetic_obs(interval: f64, n: usize, mp_amp: f64) -> ObservationData {
        let sat = Satellite::parse("G07").unwrap();
        let c1c = SignalCode::parse("C1C").unwrap();
        let c2w = SignalCode::parse("C2W").unwrap();
        let l1c = SignalCode::parse("L1C").unwrap();
        let l2w = SignalCode::parse("L2W").unwrap();
        let s1c = SignalCode::parse("S1C").unwrap();
        let lambda1 = SPEED_OF_LIGHT / frequencies::gps::L1;
        let lambda2 = SPEED_OF_LIGHT / frequencies::gps::L2;

        let mut data = ObservationData::new(Header::default());
        for i in 0..n {
            let t = i as f64 * interval;
            let range = 21_000_000.0 + 30.0 * t;
            // Injected code multipath oscillation on C1C only
            let mp_signal = mp_amp * (2.0 * std::f64::consts::PI * t / 300.0).sin();
            // Ambiguities (constant per arc): large so debiasing is provable
            let amb1 = 1234.5 * lambda1;
            let amb2 = -987.25 * lambda2;
            let l1_cycles = (range + amb1) / lambda1;
            let l2_cycles = (range + amb2) / lambda2;

            let mut eo = EpochObservations::new(make_epoch(t));
            let mut obs = HashMap::new();
            obs.insert(c1c.clone(), ObservationValue::new(range + mp_signal));
            obs.insert(c2w.clone(), ObservationValue::new(range));
            obs.insert(l1c.clone(), ObservationValue::new(l1_cycles));
            obs.insert(l2w.clone(), ObservationValue::new(l2_cycles));
            obs.insert(s1c.clone(), ObservationValue::new(45.0));
            eo.satellites.insert(sat, obs);
            data.epochs.push(eo);
        }
        data
    }

    fn base_config() -> AnalysisConfig {
        AnalysisConfig {
            systems: vec![GnssSystem::Gps],
            min_arc_seconds: 0.0,
            detect_cycle_slips: false,
            ..Default::default()
        }
    }

    #[test]
    fn test_mp_formula_identity() {
        // The generalized MP with code band k=2, partner j=1 must equal the
        // textbook MP2 = P2 − (2α/(α−1))Φ1 + (2α/(α−1) − 1)Φ2, α=(f1/f2)².
        let f1 = frequencies::gps::L1;
        let f2 = frequencies::gps::L2;
        let (p2, phi1, phi2) = (22.0e6_f64, 22.0e6 - 3.0, 22.0e6 - 4.7);

        // Engine form with "own band" = 2: a = (f2/f1)²
        let a = alpha_factor(f2, f1);
        let engine = p2 - (1.0 + 2.0 / (a - 1.0)) * phi2 + (2.0 / (a - 1.0)) * phi1;

        // Textbook form with α = (f1/f2)²
        let alpha = alpha_factor(f1, f2);
        let textbook = p2 - (2.0 * alpha / (alpha - 1.0)) * phi1
            + (2.0 * alpha / (alpha - 1.0) - 1.0) * phi2;

        assert!(
            (engine - textbook).abs() < 1e-9,
            "engine={engine}, textbook={textbook}"
        );
    }

    #[test]
    fn test_debias_recovers_injected_mp() {
        let amp = 0.4;
        let data = synthetic_obs(30.0, 120, amp);
        let analyzer = MultipathAnalyzer::new(data, base_config());
        let results = analyzer.analyze().unwrap();

        let ests = results
            .estimates
            .get("GPSM1C")
            .expect("GPSM1C must be produced");
        // Debiased values must be bias-free (mean ≈ 0) and reflect the
        // injected oscillation amplitude
        let mean: f64 = ests.iter().map(|e| e.mp_value).sum::<f64>() / ests.len() as f64;
        assert!(mean.abs() < 0.05, "residual bias too large: {mean}");
        let max = ests.iter().map(|e| e.mp_value.abs()).fold(0.0, f64::max);
        assert!((max - amp).abs() < 0.1, "amplitude {max} vs injected {amp}");
    }

    #[test]
    fn test_per_code_coverage_includes_m2w() {
        // 0.1.1 produced nothing for C2W on an L1/L2-only file
        let data = synthetic_obs(30.0, 60, 0.2);
        let analyzer = MultipathAnalyzer::new(data, base_config());
        let results = analyzer.analyze().unwrap();
        assert!(results.estimates.contains_key("GPSM1C"));
        assert!(
            results.estimates.contains_key("GPSM2W"),
            "per-code engine must produce GPSM2W; got {:?}",
            results.estimates.keys().collect::<Vec<_>>()
        );
        let stats = &results.statistics["GPSM2W"];
        assert_eq!(stats.system, "G");
        assert_eq!(stats.code, "C2W");
    }

    #[test]
    fn test_arc_split_on_gap() {
        // 40 epochs, a 10-interval hole in the middle → two arcs; with
        // min_arc_seconds=0 both survive, and no estimate bridges the gap
        let mut data = synthetic_obs(30.0, 40, 0.0);
        data.epochs.drain(18..28);
        let analyzer = MultipathAnalyzer::new(data, base_config());
        let results = analyzer.analyze().unwrap();
        let ests = &results.estimates["GPSM1C"];
        assert_eq!(ests.len(), 30);
        // Values on each side debias independently → both means ≈ 0
        let (a, b): (Vec<f64>, Vec<f64>) = (
            ests.iter().take(18).map(|e| e.mp_value).collect(),
            ests.iter().skip(18).map(|e| e.mp_value).collect(),
        );
        assert!(a.iter().sum::<f64>().abs() / 18.0 < 1e-6);
        assert!(b.iter().sum::<f64>().abs() / 12.0 < 1e-6);
    }

    #[test]
    fn test_min_arc_seconds_drops_short_arcs() {
        let data = synthetic_obs(30.0, 8, 0.0); // 210 s < 300 s
        let mut cfg = base_config();
        cfg.min_arc_seconds = 300.0;
        let analyzer = MultipathAnalyzer::new(data, cfg);
        let results = analyzer.analyze().unwrap();
        assert!(results.estimates.is_empty());
    }

    #[test]
    fn test_moving_average_removes_drift() {
        // Add a strong linear drift (simulating iono leakage on long arcs):
        // with whole-arc mean the ends stay biased; the moving average
        // (window 600 s) tracks it out
        let mut data = synthetic_obs(30.0, 240, 0.0);
        for (i, eo) in data.epochs.iter_mut().enumerate() {
            let sat = Satellite::parse("G07").unwrap();
            let c1c = SignalCode::parse("C1C").unwrap();
            if let Some(obs) = eo.satellites.get_mut(&sat) {
                if let Some(v) = obs.get_mut(&c1c) {
                    v.value += 0.002 * (i as f64 * 30.0); // 2 mm/s drift
                }
            }
        }
        let mut cfg = base_config();
        cfg.bias_window_seconds = Some(600.0);
        let analyzer = MultipathAnalyzer::new(data, cfg);
        let results = analyzer.analyze().unwrap();
        let stats = &results.statistics["GPSM1C"];
        assert!(
            stats.rms < 0.15,
            "moving average should absorb the drift, rms={}",
            stats.rms
        );
    }

    #[test]
    fn test_include_exclude_filters() {
        let data = synthetic_obs(30.0, 60, 0.1);
        let mut cfg = base_config();
        cfg.include_codes = vec!["C1C".to_string()];
        let analyzer = MultipathAnalyzer::new(data.clone(), cfg);
        let results = analyzer.analyze().unwrap();
        assert!(results.estimates.contains_key("GPSM1C"));
        assert!(!results.estimates.contains_key("GPSM2W"));

        let mut cfg2 = base_config();
        cfg2.exclude_codes = vec!["GC1C".to_string()];
        let analyzer2 = MultipathAnalyzer::new(data, cfg2);
        let results2 = analyzer2.analyze().unwrap();
        assert!(!results2.estimates.contains_key("GPSM1C"));
        assert!(results2.estimates.contains_key("GPSM2W"));
    }

    #[test]
    fn test_max_epochs_decimation() {
        let data = synthetic_obs(1.0, 1000, 0.0);
        let mut cfg = base_config();
        cfg.max_epochs = Some(100);
        let analyzer = MultipathAnalyzer::new(data, cfg);
        let results = analyzer.analyze().unwrap();
        let n = results.estimates["GPSM1C"].len();
        assert!(n <= 100, "decimated to {n}");
        assert!(n >= 50);
    }

    #[test]
    fn test_signal_names() {
        assert_eq!(
            MultipathAnalyzer::signal_name(GnssSystem::Gps, &SignalCode::parse("C1C").unwrap()),
            "GPSM1C"
        );
        assert_eq!(
            MultipathAnalyzer::signal_name(GnssSystem::Glonass, &SignalCode::parse("C3X").unwrap()),
            "GLOM3X"
        );
        assert_eq!(
            MultipathAnalyzer::signal_name(GnssSystem::Beidou, &SignalCode::parse("C2I").unwrap()),
            "BDSM2I"
        );
    }

    #[test]
    fn test_slip_resets_arc() {
        // Inject a 5-cycle L1 slip mid-file; with slip detection on, the arc
        // must split (the slip would otherwise smear a λ-scale MP jump)
        let mut data = synthetic_obs(30.0, 120, 0.0);
        let sat = Satellite::parse("G07").unwrap();
        let l1c = SignalCode::parse("L1C").unwrap();
        for eo in data.epochs.iter_mut().skip(60) {
            if let Some(obs) = eo.satellites.get_mut(&sat) {
                if let Some(v) = obs.get_mut(&l1c) {
                    v.value += 5.0;
                }
            }
        }
        let mut cfg = base_config();
        cfg.detect_cycle_slips = true;
        let analyzer = MultipathAnalyzer::new(data, cfg);
        let results = analyzer.analyze().unwrap();
        assert!(
            !results.cycle_slips.is_empty(),
            "5-cycle slip must be detected"
        );
        assert!(results.cycle_slip_counts.keys().any(|k| k.starts_with("GPSL1")));
        // Debiasing per split arc keeps the RMS small despite the jump
        let stats = &results.statistics["GPSM1C"];
        assert!(stats.rms < 0.5, "rms={} (arc not reset?)", stats.rms);
        assert!(stats.cycle_slips >= 1);
    }
}
