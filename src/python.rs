//! Python bindings for GeoVeil-MP multipath analysis library.
//!
//! This module provides Python access to the Rust GNSS analysis functionality
//! via PyO3.

#![allow(unused_imports)]
#![allow(dead_code)]

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::exceptions::{PyValueError, PyIOError};

#[cfg(feature = "python")]
use std::collections::HashMap;
#[cfg(feature = "python")]
use std::path::Path;

// Import from our crate
#[cfg(feature = "python")]
use crate::rinex::{GnssSystem, Satellite, SignalCode, ObservationType, ObservationData, RinexObsReader};
#[cfg(feature = "python")]
use crate::utils::{Ecef, Epoch, Geodetic, GpsTime};
#[cfg(feature = "python")]
use crate::navigation::NevilleInterpolator;

/// Python-exposed GNSS System enum
#[cfg(feature = "python")]
#[pyclass(name = "GnssSystem")]
#[derive(Clone)]
pub struct PyGnssSystem {
    inner: GnssSystem,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyGnssSystem {
    #[new]
    fn new(code: &str) -> PyResult<Self> {
        let c = code.chars().next().ok_or_else(|| {
            PyValueError::new_err("Empty system code")
        })?;
        let inner = GnssSystem::from_char(c).ok_or_else(|| {
            PyValueError::new_err(format!("Unknown system: {}", code))
        })?;
        Ok(Self { inner })
    }
    
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }
    
    #[getter]
    fn code(&self) -> String {
        self.inner.to_char().to_string()
    }
    
    fn __repr__(&self) -> String {
        format!("GnssSystem('{}')", self.inner.to_char())
    }
    
    fn __str__(&self) -> String {
        self.inner.name().to_string()
    }
}

/// Python-exposed Satellite
#[cfg(feature = "python")]
#[pyclass(name = "Satellite")]
#[derive(Clone)]
pub struct PySatellite {
    pub(crate) inner: Satellite,
}

#[cfg(feature = "python")]
#[pymethods]
impl PySatellite {
    #[new]
    fn new(id: &str) -> PyResult<Self> {
        let inner = Satellite::parse(id).ok_or_else(|| {
            PyValueError::new_err(format!("Invalid satellite ID: {}", id))
        })?;
        Ok(Self { inner })
    }
    
    #[getter]
    fn system(&self) -> PyGnssSystem {
        PyGnssSystem { inner: self.inner.system }
    }
    
    #[getter]
    fn prn(&self) -> u32 {
        self.inner.prn
    }
    
    #[getter]
    fn id(&self) -> String {
        self.inner.to_string()
    }
    
    fn __repr__(&self) -> String {
        format!("Satellite('{}')", self.inner)
    }
    
    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Python-exposed Epoch (time)
#[cfg(feature = "python")]
#[pyclass(name = "Epoch")]
#[derive(Clone)]
pub struct PyEpoch {
    pub(crate) inner: Epoch,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyEpoch {
    #[new]
    #[pyo3(signature = (year, month, day, hour=0, minute=0, second=0.0))]
    fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: f64) -> Self {
        Self {
            inner: Epoch::new(year, month, day, hour, minute, second)
        }
    }
    
    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        Epoch::parse(s)
            .map(|e| Self { inner: e })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
    
    #[getter]
    fn year(&self) -> i32 { self.inner.year }
    
    #[getter]
    fn month(&self) -> u32 { self.inner.month }
    
    #[getter]
    fn day(&self) -> u32 { self.inner.day }
    
    #[getter]
    fn hour(&self) -> u32 { self.inner.hour }
    
    #[getter]
    fn minute(&self) -> u32 { self.inner.minute }
    
    #[getter]
    fn second(&self) -> f64 { self.inner.second }
    
    fn to_iso(&self) -> String {
        self.inner.to_iso_string()
    }
    
    fn to_iso_string(&self) -> String {
        self.inner.to_iso_string()
    }
    
    fn to_gps_time(&self) -> (i32, f64) {
        let gps = self.inner.to_gps_time();
        (gps.week as i32, gps.tow)
    }
    
    fn julian_date(&self) -> f64 {
        self.inner.to_julian_date()
    }
    
    fn day_of_year(&self) -> u32 {
        self.inner.day_of_year()
    }
    
    fn __repr__(&self) -> String {
        format!("Epoch({})", self.inner.to_iso_string())
    }
    
    fn __str__(&self) -> String {
        self.inner.to_iso_string()
    }
}

/// Python-exposed ECEF coordinates
#[cfg(feature = "python")]
#[pyclass(name = "Ecef")]
#[derive(Clone)]
pub struct PyEcef {
    pub(crate) inner: Ecef,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyEcef {
    #[new]
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { inner: Ecef::new(x, y, z) }
    }
    
    #[getter]
    fn x(&self) -> f64 { self.inner.x }
    
    #[getter]
    fn y(&self) -> f64 { self.inner.y }
    
    #[getter]
    fn z(&self) -> f64 { self.inner.z }
    
    fn to_geodetic(&self) -> PyGeodetic {
        PyGeodetic { inner: self.inner.to_geodetic() }
    }
    
    fn magnitude(&self) -> f64 {
        self.inner.magnitude()
    }
    
    fn distance(&self, other: &PyEcef) -> f64 {
        self.inner.distance(&other.inner)
    }
    
    fn __repr__(&self) -> String {
        format!("Ecef({:.3}, {:.3}, {:.3})", self.inner.x, self.inner.y, self.inner.z)
    }
}

/// Python-exposed Geodetic coordinates
#[cfg(feature = "python")]
#[pyclass(name = "Geodetic")]
#[derive(Clone)]
pub struct PyGeodetic {
    inner: Geodetic,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyGeodetic {
    #[new]
    fn new(lat: f64, lon: f64, height: f64) -> Self {
        Self { inner: Geodetic { lat, lon, height } }
    }
    
    #[getter]
    fn lat(&self) -> f64 { self.inner.lat }
    
    #[getter]
    fn lon(&self) -> f64 { self.inner.lon }
    
    #[getter]
    fn height(&self) -> f64 { self.inner.height }
    
    fn to_ecef(&self) -> PyEcef {
        PyEcef { inner: crate::utils::geodetic_to_ecef(&self.inner) }
    }
    
    fn __repr__(&self) -> String {
        format!("Geodetic({:.6}°, {:.6}°, {:.1}m)", 
                self.inner.lat, self.inner.lon, self.inner.height)
    }
}

/// Python-exposed RINEX observation data
#[cfg(feature = "python")]
#[pyclass(name = "RinexObsData")]
pub struct PyRinexObsData {
    pub(crate) inner: ObservationData,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyRinexObsData {
    #[getter]
    fn version(&self) -> String {
        format!("{}", self.inner.header.version)
    }
    
    #[getter]
    fn marker_name(&self) -> String {
        self.inner.header.marker_name.clone()
    }
    
    #[getter]
    fn receiver_type(&self) -> String {
        self.inner.header.receiver_type.clone()
    }
    
    #[getter]
    fn antenna_type(&self) -> String {
        self.inner.header.antenna_type.clone()
    }
    
    #[getter]
    fn approx_position(&self) -> Option<PyEcef> {
        self.inner.header.approx_position.clone().map(|p| PyEcef { inner: p })
    }
    
    #[getter]
    fn interval(&self) -> Option<f64> {
        self.inner.interval()
    }
    
    #[getter]
    fn num_epochs(&self) -> usize {
        self.inner.num_epochs()
    }
    
    #[getter]
    fn num_satellites(&self) -> usize {
        self.inner.satellites().len()
    }
    
    fn satellites(&self) -> Vec<PySatellite> {
        self.inner.satellites().into_iter()
            .map(|s| PySatellite { inner: s })
            .collect()
    }
    
    fn epochs(&self) -> Vec<PyEpoch> {
        self.inner.epochs.iter()
            .map(|e| PyEpoch { inner: e.epoch })
            .collect()
    }
    
    fn first_epoch(&self) -> Option<PyEpoch> {
        self.inner.epochs.first().map(|e| PyEpoch { inner: e.epoch })
    }
    
    fn last_epoch(&self) -> Option<PyEpoch> {
        self.inner.epochs.last().map(|e| PyEpoch { inner: e.epoch })
    }
    
    fn observation_types(&self, system: &str) -> Vec<String> {
        let sys = GnssSystem::from_char(system.chars().next().unwrap_or('G'))
            .unwrap_or(GnssSystem::Gps);
        self.inner.signal_codes_for_system(sys)
            .into_iter()
            .map(|c| c.to_string())
            .collect()
    }
    
    fn glonass_fcn(&self) -> HashMap<u32, i8> {
        self.inner.header.glonass_slot_frq.clone()
    }
    
    fn satellites_by_system(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for sat in self.inner.satellites() {
            let sys = sat.system.to_char().to_string();
            *counts.entry(sys).or_insert(0) += 1;
        }
        counts
    }
    
    fn __repr__(&self) -> String {
        format!("RinexObsData(version={}, marker='{}', epochs={}, sats={})",
                self.inner.header.version,
                self.inner.header.marker_name,
                self.inner.num_epochs(),
                self.inner.satellites().len())
    }
}

/// Python-exposed Multipath estimate
#[cfg(feature = "python")]
#[pyclass(name = "MultipathEstimate")]
#[derive(Clone)]
pub struct PyMultipathEstimate {
    #[pyo3(get)]
    pub satellite: String,
    #[pyo3(get)]
    pub system: String,
    #[pyo3(get)]
    pub epoch: String,
    #[pyo3(get)]
    pub mp_value: f64,
    #[pyo3(get, set)]
    pub elevation: f64,
    #[pyo3(get, set)]
    pub azimuth: f64,
    #[pyo3(get)]
    pub snr: Option<f64>,
    #[pyo3(get)]
    pub signal: String,
}

/// Python-exposed Multipath statistics
#[cfg(feature = "python")]
#[pyclass(name = "MultipathStats")]
#[derive(Clone)]
pub struct PyMultipathStats {
    #[pyo3(get)]
    pub signal: String,
    #[pyo3(get)]
    pub count: usize,
    #[pyo3(get)]
    pub rms: f64,
    #[pyo3(get)]
    pub mean: f64,
    #[pyo3(get)]
    pub std_dev: f64,
    #[pyo3(get)]
    pub min: f64,
    #[pyo3(get)]
    pub max: f64,
    #[pyo3(get)]
    pub weighted_rms: f64,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyMultipathStats {
    fn __repr__(&self) -> String {
        format!("MultipathStats(signal='{}', rms={:.4}, count={})",
                self.signal, self.rms, self.count)
    }
}

/// Python-exposed Cycle slip
#[cfg(feature = "python")]
#[pyclass(name = "CycleSlip")]
#[derive(Clone)]
pub struct PyCycleSlip {
    #[pyo3(get)]
    pub satellite: String,
    #[pyo3(get)]
    pub epoch: String,
    #[pyo3(get)]
    pub magnitude: f64,
    #[pyo3(get)]
    pub method: String,
}

/// Python-exposed Analysis results
#[cfg(feature = "python")]
#[pyclass(name = "AnalysisResults")]
pub struct PyAnalysisResults {
    #[pyo3(get)]
    pub estimates: Vec<PyMultipathEstimate>,
    #[pyo3(get)]
    pub statistics: Vec<PyMultipathStats>,
    #[pyo3(get)]
    pub cycle_slips: Vec<PyCycleSlip>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyAnalysisResults {
    fn total_estimates(&self) -> usize {
        self.estimates.len()
    }
    
    fn total_cycle_slips(&self) -> usize {
        self.cycle_slips.len()
    }
    
    /// Compute elevations and azimuths for all estimates using SP3 data
    /// Returns (computed_count, failed_count)
    fn compute_elevations(&mut self, sp3: &PySp3Data, receiver: &PyEcef) -> (usize, usize) {
        let interpolator = NevilleInterpolator::new();
        let mut computed = 0;
        let mut failed = 0;
        
        for est in &mut self.estimates {
            // Parse satellite
            let sat = match crate::rinex::Satellite::parse(&est.satellite) {
                Some(s) => s,
                None => {
                    failed += 1;
                    continue;
                }
            };
            
            // Parse epoch from ISO string
            let epoch = match Epoch::parse(&est.epoch) {
                Ok(e) => e,
                Err(_) => {
                    failed += 1;
                    continue;
                }
            };
            
            // Interpolate satellite position
            let sat_pos = match interpolator.interpolate(&sp3.inner, &sat, &epoch) {
                Some(p) => p.position,
                None => {
                    failed += 1;
                    continue;
                }
            };
            
            // Calculate azimuth and elevation
            let azel = crate::utils::calculate_azel(&receiver.inner, &sat_pos);
            
            // Only accept if elevation is positive (satellite above horizon)
            if azel.elevation > 0.0 {
                est.elevation = azel.elevation;
                est.azimuth = azel.azimuth;
                computed += 1;
            } else {
                failed += 1;
            }
        }
        
        (computed, failed)
    }
    
    fn __repr__(&self) -> String {
        format!("AnalysisResults(estimates={}, signals={}, cycle_slips={})",
                self.estimates.len(),
                self.statistics.len(),
                self.cycle_slips.len())
    }
}

/// Python-exposed Multipath Analyzer
#[cfg(feature = "python")]
#[pyclass(name = "MultipathAnalyzer")]
pub struct PyMultipathAnalyzer {
    obs_data: ObservationData,
    elevation_cutoff: f64,
    systems: Vec<GnssSystem>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyMultipathAnalyzer {
    #[new]
    #[pyo3(signature = (obs_data, elevation_cutoff=10.0, systems=None))]
    fn new(obs_data: &PyRinexObsData, elevation_cutoff: f64, systems: Option<Vec<String>>) -> Self {
        let systems = systems.map(|s| {
            s.iter()
                .filter_map(|c| GnssSystem::from_char(c.chars().next()?))
                .collect()
        }).unwrap_or_else(|| vec![
            GnssSystem::Gps,
            GnssSystem::Glonass,
            GnssSystem::Galileo,
            GnssSystem::Beidou,
        ]);
        
        Self {
            obs_data: obs_data.inner.clone(),
            elevation_cutoff,
            systems,
        }
    }
    
    fn analyze(&self) -> PyResult<PyAnalysisResults> {
        // Multipath analysis with bias removal per satellite arc
        // Key: (satellite, signal) -> Vec of (epoch_seconds, epoch_string, mp_value)
        let mut raw_mp: HashMap<(String, String), Vec<(f64, String, f64)>> = HashMap::new();
        
        // First pass: compute raw MP values with epoch timestamps for arc detection
        for epoch_obs in &self.obs_data.epochs {
            let epoch_seconds = epoch_obs.epoch.to_julian_date() * 86400.0; // For arc detection
            
            for (sat, obs) in &epoch_obs.satellites {
                if !self.systems.contains(&sat.system) {
                    continue;
                }
                
                let sys_char = sat.system.to_char();
                
                // Get code and phase observations
                let code_obs: Vec<_> = obs.iter()
                    .filter(|(k, _)| k.is_code())
                    .collect();
                let phase_obs: Vec<_> = obs.iter()
                    .filter(|(k, _)| k.is_phase())
                    .collect();
                
                // Need at least one code and two phases for MP
                if code_obs.is_empty() || phase_obs.len() < 2 {
                    continue;
                }
                
                // Try to find matching frequency pairs
                for (code_sig, code_val) in &code_obs {
                    let code_band = code_sig.band;
                    
                    // Find two phase observations on different bands
                    let mut phases_by_band: HashMap<u8, (&SignalCode, f64)> = HashMap::new();
                    for (p_sig, p_val) in &phase_obs {
                        phases_by_band.insert(p_sig.band, (*p_sig, p_val.value));
                    }
                    
                    // Get phases for bands with sufficiently different frequencies
                    // Avoid BeiDou (1,2) pair as B1C and B1I are too close (1575 vs 1561 MHz)
                    // Prefer wider frequency separations for better MP estimates
                    let band_pairs: &[(u8, u8)] = if sys_char == 'C' {
                        // BeiDou: Use B1C-B2a (1,5), B1C-B3I (1,6), B1C-B2I (1,7), etc.
                        &[(1, 5), (1, 6), (1, 7), (2, 5), (2, 7), (5, 6), (5, 7), (6, 7)]
                    } else {
                        // GPS, Galileo, GLONASS: standard pairs
                        &[(1, 2), (1, 5), (2, 5), (1, 7), (5, 7), (1, 6), (6, 7), (2, 7)]
                    };
                    
                    for (b1, b2) in band_pairs {
                        // CRITICAL: Code must be on the same band as the first phase (b1)
                        // MP_i = C_i - (1 + 2/(α-1))·L_i + (2/(α-1))·L_j
                        // where i is the code/first-phase band, j is the second phase band
                        if code_band != *b1 {
                            continue;  // Skip - code band doesn't match first phase band
                        }
                        
                        if let (Some((_, l1_cycles)), Some((_, l2_cycles))) = 
                            (phases_by_band.get(b1), phases_by_band.get(b2)) 
                        {
                            // Get GLONASS FCN if applicable
                            let fcn = if sat.system == GnssSystem::Glonass {
                                self.obs_data.header.glonass_slot_frq.get(&sat.prn).copied()
                            } else {
                                None
                            };
                            
                            // Get frequencies (with FCN for GLONASS)
                            let f1 = match crate::utils::constants::get_frequency(sys_char, *b1, fcn) {
                                Some(f) => f,
                                None => continue,
                            };
                            let f2 = match crate::utils::constants::get_frequency(sys_char, *b2, fcn) {
                                Some(f) => f,
                                None => continue,
                            };
                            
                            // Compute wavelengths
                            let lambda1 = crate::utils::constants::SPEED_OF_LIGHT / f1;
                            let lambda2 = crate::utils::constants::SPEED_OF_LIGHT / f2;
                            
                            // Alpha factor (f1/f2)^2
                            let alpha = (f1 / f2).powi(2);
                            
                            // Skip if frequencies are too close (ill-conditioned)
                            // alpha should be > 1.1 or < 0.9 for good MP estimates
                            if (alpha - 1.0).abs() < 0.1 {
                                continue;  // frequencies too close, try next pair
                            }
                            
                            // Convert phase to meters
                            let l1 = l1_cycles * lambda1;
                            let l2 = l2_cycles * lambda2;
                            let c1 = code_val.value;
                            
                            // MP = C1 - (1 + 2/(α-1))·L1 + (2/(α-1))·L2
                            let mp = c1 - (1.0 + 2.0/(alpha - 1.0)) * l1 + (2.0/(alpha - 1.0)) * l2;
                            
                            if mp.is_finite() && mp.abs() < 1000.0 {
                                let key = (sat.to_string(), code_sig.to_string());
                                raw_mp.entry(key)
                                    .or_default()
                                    .push((epoch_seconds, epoch_obs.epoch.to_iso_string(), mp));
                            }
                            
                            break; // Use first valid band pair
                        }
                    }
                }
            }
        }
        
        // Second pass: detect arcs and remove bias per arc
        // Arc boundary: gap > 60 seconds OR large MP jump (cycle slip indicator)
        let arc_gap_threshold = 60.0; // seconds
        let arc_jump_threshold = 10.0; // meters - MP jump indicating cycle slip
        let mut estimates = Vec::new();
        let mut stats_map: HashMap<String, Vec<f64>> = HashMap::new();
        
        for ((sat_id, signal), mut values) in raw_mp {
            if values.len() < 2 {
                continue;
            }
            
            // Sort by epoch time
            values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            
            // Get system char
            let sys_char = sat_id.chars().next().unwrap_or('G');
            
            // Split into arcs based on time gaps OR MP jumps
            let mut arcs: Vec<Vec<(f64, String, f64)>> = Vec::new();
            let mut current_arc: Vec<(f64, String, f64)> = Vec::new();
            
            for (i, val) in values.iter().enumerate() {
                if i == 0 {
                    current_arc.push(val.clone());
                } else {
                    let time_gap = val.0 - values[i-1].0;
                    let mp_jump = (val.2 - values[i-1].2).abs();
                    
                    // Start new arc if time gap too large OR MP value jumped (cycle slip)
                    if time_gap > arc_gap_threshold || mp_jump > arc_jump_threshold {
                        // Save current arc if it has enough points
                        if current_arc.len() >= 10 {  // Require at least 10 epochs per arc
                            arcs.push(current_arc);
                        }
                        current_arc = Vec::new();
                    }
                    current_arc.push(val.clone());
                }
            }
            // Don't forget the last arc
            if current_arc.len() >= 10 {
                arcs.push(current_arc);
            }
            
            // Process each arc: compute mean and remove bias
            for arc in arcs {
                // Compute mean (bias) for this arc
                let mean_bias: f64 = arc.iter().map(|(_, _, mp)| mp).sum::<f64>() / arc.len() as f64;
                
                // Store debiased values
                for (_, epoch, mp) in arc {
                    let mp_debiased = mp - mean_bias;
                    
                    estimates.push(PyMultipathEstimate {
                        satellite: sat_id.clone(),
                        system: sys_char.to_string(),
                        epoch: epoch.clone(),
                        mp_value: mp_debiased,
                        elevation: 45.0, // Placeholder - will be updated by compute_elevations
                        azimuth: 0.0,
                        snr: None,
                        signal: signal.clone(),
                    });
                    
                    stats_map.entry(format!("{}_{}", sys_char, signal))
                        .or_default()
                        .push(mp_debiased);
                }
            }
        }
        
        // Compute statistics from debiased values
        let mut statistics = Vec::new();
        for (signal, values) in stats_map {
            if values.is_empty() {
                continue;
            }
            
            let n = values.len() as f64;
            let mean = values.iter().sum::<f64>() / n;
            let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
            let std_dev = variance.sqrt();
            let rms = (values.iter().map(|x| x.powi(2)).sum::<f64>() / n).sqrt();
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            
            statistics.push(PyMultipathStats {
                signal,
                count: values.len(),
                rms,
                mean,
                std_dev,
                min,
                max,
                weighted_rms: rms,
            });
        }
        
        // Sort statistics by signal name
        statistics.sort_by(|a, b| a.signal.cmp(&b.signal));
        
        Ok(PyAnalysisResults {
            estimates,
            statistics,
            cycle_slips: Vec::new(),
        })
    }
}

/// Read RINEX observation file
#[cfg(feature = "python")]
#[pyfunction]
fn read_rinex_obs(path: &str) -> PyResult<PyRinexObsData> {
    let reader = RinexObsReader::new();
    let data = reader.read(path)
        .map_err(|e| PyIOError::new_err(format!("Failed to read RINEX: {}", e)))?;
    
    Ok(PyRinexObsData { inner: data })
}

/// Read RINEX observation from bytes
#[cfg(feature = "python")]
#[pyfunction]
fn read_rinex_obs_bytes(data: &[u8], filename: &str) -> PyResult<PyRinexObsData> {
    // Write to temp file and read
    use std::io::Write;
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(filename);
    
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| PyIOError::new_err(format!("Failed to create temp file: {}", e)))?;
    file.write_all(data)
        .map_err(|e| PyIOError::new_err(format!("Failed to write temp file: {}", e)))?;
    drop(file);
    
    let reader = RinexObsReader::new();
    let obs_data = reader.read(temp_path.to_str().unwrap())
        .map_err(|e| PyIOError::new_err(format!("Failed to parse RINEX: {}", e)))?;
    
    // Clean up
    let _ = std::fs::remove_file(&temp_path);
    
    Ok(PyRinexObsData { inner: obs_data })
}

/// Get frequency for a GNSS signal
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (system, band, fcn=None))]
fn get_frequency(system: &str, band: u8, fcn: Option<i8>) -> Option<f64> {
    let sys = system.chars().next()?;
    crate::utils::constants::get_frequency(sys, band, fcn)
}

/// Get wavelength for a GNSS signal
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (system, band, fcn=None))]
fn get_wavelength(system: &str, band: u8, fcn: Option<i8>) -> Option<f64> {
    let freq = get_frequency(system, band, fcn)?;
    Some(crate::utils::constants::SPEED_OF_LIGHT / freq)
}

/// Calculate azimuth and elevation
#[cfg(feature = "python")]
#[pyfunction]
fn calculate_azel(receiver: &PyEcef, satellite: &PyEcef) -> (f64, f64) {
    let azel = crate::utils::calculate_azel(&receiver.inner, &satellite.inner);
    (azel.azimuth, azel.elevation)
}

/// Get library version
#[cfg(feature = "python")]
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ============ SP3 Support ============

#[cfg(feature = "python")]
use crate::navigation::{Sp3Data, Sp3Reader};

/// Python-exposed SP3 data
#[cfg(feature = "python")]
#[pyclass(name = "Sp3Data")]
pub struct PySp3Data {
    inner: Sp3Data,
}

#[cfg(feature = "python")]
#[pymethods]
impl PySp3Data {
    #[getter]
    fn num_satellites(&self) -> usize {
        self.inner.satellites.len()
    }
    
    #[getter]
    fn num_epochs(&self) -> usize {
        self.inner.num_epochs
    }
    
    #[getter]
    fn interval(&self) -> f64 {
        self.inner.interval
    }
    
    fn satellites(&self) -> Vec<String> {
        self.inner.satellites.iter().map(|s| s.to_string()).collect()
    }
    
    fn get_position(&self, satellite: &str, epoch: &PyEpoch) -> Option<PyEcef> {
        let sat = crate::rinex::Satellite::parse(satellite)?;
        let interpolator = NevilleInterpolator::new();
        let pos = interpolator.interpolate(&self.inner, &sat, &epoch.inner)?;
        Some(PyEcef { inner: pos.position })
    }
    
    fn __repr__(&self) -> String {
        format!("Sp3Data(satellites={}, epochs={}, interval={}s)",
                self.inner.satellites.len(),
                self.inner.num_epochs,
                self.inner.interval)
    }
}

/// Read SP3 file
#[cfg(feature = "python")]
#[pyfunction]
fn read_sp3(path: &str) -> PyResult<PySp3Data> {
    let data = Sp3Reader::read(path)
        .map_err(|e| PyIOError::new_err(format!("Failed to read SP3: {}", e)))?;
    Ok(PySp3Data { inner: data })
}

/// Compute satellite elevation from SP3 and receiver position
#[cfg(feature = "python")]
#[pyfunction]
fn compute_elevation(sp3: &PySp3Data, receiver: &PyEcef, satellite: &str, epoch: &PyEpoch) -> Option<f64> {
    let sat = crate::rinex::Satellite::parse(satellite)?;
    let interpolator = NevilleInterpolator::new();
    let sat_pos = interpolator.interpolate(&sp3.inner, &sat, &epoch.inner)?;
    let azel = crate::utils::calculate_azel(&receiver.inner, &sat_pos.position);
    Some(azel.elevation)
}

/// Python module definition
#[cfg(feature = "python")]
#[pymodule]
fn geoveil_mp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PyGnssSystem>()?;
    m.add_class::<PySatellite>()?;
    m.add_class::<PyEpoch>()?;
    m.add_class::<PyEcef>()?;
    m.add_class::<PyGeodetic>()?;
    m.add_class::<PyRinexObsData>()?;
    m.add_class::<PyMultipathEstimate>()?;
    m.add_class::<PyMultipathStats>()?;
    m.add_class::<PyCycleSlip>()?;
    m.add_class::<PyAnalysisResults>()?;
    m.add_class::<PyMultipathAnalyzer>()?;
    m.add_class::<PySp3Data>()?;
    
    // Functions
    m.add_function(wrap_pyfunction!(read_rinex_obs, m)?)?;
    m.add_function(wrap_pyfunction!(read_rinex_obs_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(read_sp3, m)?)?;
    m.add_function(wrap_pyfunction!(get_frequency, m)?)?;
    m.add_function(wrap_pyfunction!(get_wavelength, m)?)?;
    m.add_function(wrap_pyfunction!(calculate_azel, m)?)?;
    m.add_function(wrap_pyfunction!(compute_elevation, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    
    // Constants
    m.add("SPEED_OF_LIGHT", crate::utils::constants::SPEED_OF_LIGHT)?;
    m.add("GM_WGS84", crate::utils::constants::GM_WGS84)?;
    m.add("EARTH_RADIUS", crate::utils::constants::EARTH_RADIUS_WGS84)?;
    
    Ok(())
}
