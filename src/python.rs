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
#[cfg(feature = "python")]
use std::sync::Arc;

#[cfg(feature = "python")]
use crate::analysis::{AnalysisConfig, MultipathAnalyzer, SlipThresholds};

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
    pub(crate) inner: Arc<ObservationData>,
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

    /// S-codes with data for a satellite (e.g. ["S1C", "S2W"])
    fn snr_codes(&self, satellite: &str) -> PyResult<Vec<String>> {
        let sat = Satellite::parse(satellite)
            .ok_or_else(|| PyValueError::new_err(format!("Invalid satellite id '{satellite}'")))?;
        let mut codes: Vec<String> = self
            .inner
            .epochs
            .iter()
            .filter_map(|e| e.satellites.get(&sat))
            .flat_map(|obs| obs.keys().filter(|c| c.is_snr()).map(|c| c.to_string()))
            .collect();
        codes.sort();
        codes.dedup();
        Ok(codes)
    }

    /// SNR time series for one satellite: one SnrSeries per S-code, or only
    /// the requested code. Times are unix seconds in the file's time scale.
    #[pyo3(signature = (satellite, code=None))]
    fn get_snr_series(&self, satellite: &str, code: Option<String>) -> PyResult<Vec<PySnrSeries>> {
        let sat = Satellite::parse(satellite)
            .ok_or_else(|| PyValueError::new_err(format!("Invalid satellite id '{satellite}'")))?;
        let wanted: Option<SignalCode> = match code {
            Some(c) => Some(
                SignalCode::parse(&c.to_ascii_uppercase())
                    .filter(|sc| sc.is_snr())
                    .ok_or_else(|| PyValueError::new_err(format!("'{c}' is not an S-code")))?,
            ),
            None => None,
        };

        let mut series: HashMap<SignalCode, (Vec<f64>, Vec<f64>)> = HashMap::new();
        for epoch_obs in &self.inner.epochs {
            let obs = match epoch_obs.satellites.get(&sat) {
                Some(o) => o,
                None => continue,
            };
            let t = epoch_obs.epoch.to_unix_seconds();
            for (c, v) in obs.iter().filter(|(c, _)| c.is_snr()) {
                if let Some(w) = &wanted {
                    if c != w {
                        continue;
                    }
                }
                if v.value <= 0.0 {
                    continue;
                }
                let entry = series.entry(c.clone()).or_default();
                entry.0.push(t);
                entry.1.push(v.value);
            }
        }

        let sys = sat.system.to_char().to_string();
        let mut out: Vec<PySnrSeries> = series
            .into_iter()
            .map(|(c, (times, values))| PySnrSeries {
                satellite: satellite.to_string(),
                system: sys.clone(),
                code: c.to_string(),
                times,
                values,
            })
            .collect();
        out.sort_by(|a, b| a.code.cmp(&b.code));
        Ok(out)
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
    #[pyo3(get)]
    pub system: String,
    #[pyo3(get)]
    pub code: String,
    #[pyo3(get)]
    pub cycle_slips: usize,
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
    #[pyo3(get)]
    pub signal: String,
    #[pyo3(get)]
    pub system: String,
    #[pyo3(get)]
    pub threshold: f64,
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
    #[pyo3(get)]
    pub cycle_slip_counts: HashMap<String, usize>,
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
    fn compute_elevations(&mut self, py: Python<'_>, sp3: &PySp3Data, receiver: &PyEcef) -> (usize, usize) {
        let receiver_pos = receiver.inner.clone();
        let sp3_data = &sp3.inner;
        let estimates = &mut self.estimates;

        // Pure Rust interpolation over (possibly) hundreds of thousands of
        // estimates — release the GIL
        let (computed, failed) = py.allow_threads(move || {
            let interpolator = NevilleInterpolator::new();
            let mut computed = 0;
            let mut failed = 0;

            for est in estimates.iter_mut() {
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
                let sat_pos = match interpolator.interpolate(sp3_data, &sat, &epoch) {
                    Some(p) => p.position,
                    None => {
                        failed += 1;
                        continue;
                    }
                };

                // Calculate azimuth and elevation
                let azel = crate::utils::calculate_azel(&receiver_pos, &sat_pos);

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
        });

        // Recompute elevation-weighted RMS now that real elevations exist
        if computed > 0 {
            let mut acc: HashMap<String, (f64, f64)> = HashMap::new(); // key -> (Σw·mp², Σw)
            for est in &self.estimates {
                let abbrev = crate::rinex::GnssSystem::from_char(
                    est.system.chars().next().unwrap_or('G'),
                )
                .map(|s| s.abbrev())
                .unwrap_or("GPS");
                let key = format!("{}M{}", abbrev, &est.signal[1..]);
                let w = crate::utils::elevation_weight(est.elevation);
                let e = acc.entry(key).or_insert((0.0, 0.0));
                e.0 += w * est.mp_value * est.mp_value;
                e.1 += w;
            }
            for stat in &mut self.statistics {
                if let Some((sw, w)) = acc.get(&stat.signal) {
                    if *w > 0.0 {
                        stat.weighted_rms = (sw / w).sqrt();
                    }
                }
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
    obs_data: Arc<ObservationData>,
    config: AnalysisConfig,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyMultipathAnalyzer {
    #[new]
    #[pyo3(signature = (obs_data, elevation_cutoff=10.0, systems=None,
        bias_window_seconds=1500.0, min_arc_seconds=300.0, arc_gap_factor=5.0,
        include_codes=None, exclude_codes=None, max_epochs=None,
        detect_cycle_slips=true, ion_delta_base=0.10, ion_delta_rate=0.003,
        cp_delta_base=5.0, cp_delta_rate=0.10))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        obs_data: &PyRinexObsData,
        elevation_cutoff: f64,
        systems: Option<Vec<String>>,
        bias_window_seconds: Option<f64>,
        min_arc_seconds: f64,
        arc_gap_factor: f64,
        include_codes: Option<Vec<String>>,
        exclude_codes: Option<Vec<String>>,
        max_epochs: Option<usize>,
        detect_cycle_slips: bool,
        ion_delta_base: f64,
        ion_delta_rate: f64,
        cp_delta_base: f64,
        cp_delta_rate: f64,
    ) -> PyResult<Self> {
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

        // Validate code filters: "C1C" (all systems) or "GC1C" (one system)
        let validate = |list: &Option<Vec<String>>, name: &str| -> PyResult<Vec<String>> {
            let mut out = Vec::new();
            if let Some(items) = list {
                for item in items {
                    let it = item.trim().to_ascii_uppercase();
                    let code_part = match it.len() {
                        3 => it.as_str(),
                        4 => &it[1..],
                        _ => {
                            return Err(PyValueError::new_err(format!(
                                "{name} entry '{item}' must be a 3-char code ('C1C') or system+code ('GC1C')"
                            )))
                        }
                    };
                    if SignalCode::parse(code_part).is_none() {
                        return Err(PyValueError::new_err(format!(
                            "{name} entry '{item}' is not a valid signal code"
                        )));
                    }
                    if it.len() == 4 && GnssSystem::from_char(it.chars().next().unwrap()).is_none() {
                        return Err(PyValueError::new_err(format!(
                            "{name} entry '{item}' has an unknown system prefix"
                        )));
                    }
                    out.push(it);
                }
            }
            Ok(out)
        };

        let config = AnalysisConfig {
            elevation_cutoff,
            systems,
            bias_window_seconds,
            min_arc_seconds,
            arc_gap_factor,
            include_codes: validate(&include_codes, "include_codes")?,
            exclude_codes: validate(&exclude_codes, "exclude_codes")?,
            max_epochs,
            detect_cycle_slips,
            slip: SlipThresholds {
                ion_delta_base,
                ion_delta_rate,
                cp_delta_base,
                cp_delta_rate,
                ..SlipThresholds::default()
            },
            ..AnalysisConfig::default()
        };

        Ok(Self {
            obs_data: obs_data.inner.clone(),
            config,
        })
    }

    fn analyze(&self, py: Python<'_>) -> PyResult<PyAnalysisResults> {
        let obs_data = self.obs_data.clone();
        let config = self.config.clone();

        // Engine AND result conversion are pure Rust (PyO3 structs are plain
        // data until wrapped) — release the GIL for the whole pipeline so
        // callers can analyze multiple files in parallel threads
        let out = py.allow_threads(move || -> Result<PyAnalysisResults, String> {
            let results = MultipathAnalyzer::new(obs_data, config)
                .analyze()
                .map_err(|e| format!("Multipath analysis failed: {e}"))?;

            // Flatten estimates, sorted for deterministic output
            let mut keys: Vec<&String> = results.estimates.keys().collect();
            keys.sort();
            let mut estimates = Vec::with_capacity(results.summary.total_estimates);
            for key in &keys {
                for est in &results.estimates[*key] {
                    estimates.push(PyMultipathEstimate {
                        satellite: est.satellite.to_string(),
                        system: est.satellite.system.to_char().to_string(),
                        epoch: est.epoch.to_iso_string(),
                        mp_value: est.mp_value,
                        elevation: est.elevation,
                        azimuth: est.azimuth,
                        snr: est.snr,
                        signal: est.signal.clone(),
                    });
                }
            }

            let mut statistics: Vec<PyMultipathStats> = results
                .statistics
                .iter()
                .map(|(name, s)| PyMultipathStats {
                    signal: name.clone(),
                    count: s.count,
                    rms: s.rms,
                    mean: s.mean,
                    std_dev: s.std_dev,
                    min: s.min,
                    max: s.max,
                    weighted_rms: s.weighted_rms,
                    system: s.system.clone(),
                    code: s.code.clone(),
                    cycle_slips: s.cycle_slips,
                })
                .collect();
            statistics.sort_by(|a, b| a.signal.cmp(&b.signal));

            let cycle_slips: Vec<PyCycleSlip> = results
                .cycle_slips
                .iter()
                .map(|cs| PyCycleSlip {
                    satellite: cs.satellite.to_string(),
                    epoch: cs.epoch.to_iso_string(),
                    magnitude: cs.magnitude,
                    method: cs.method.as_str().to_string(),
                    signal: cs
                        .signals
                        .first()
                        .map(|s| s.to_string())
                        .unwrap_or_default(),
                    system: cs.satellite.system.to_char().to_string(),
                    threshold: cs.threshold,
                })
                .collect();

            Ok(PyAnalysisResults {
                estimates,
                statistics,
                cycle_slips,
                cycle_slip_counts: results.cycle_slip_counts,
            })
        });

        out.map_err(PyValueError::new_err)
    }
}

/// SNR time series for one satellite and one S-code
#[cfg(feature = "python")]
#[pyclass(name = "SnrSeries")]
#[derive(Clone)]
pub struct PySnrSeries {
    #[pyo3(get)]
    pub satellite: String,
    #[pyo3(get)]
    pub system: String,
    #[pyo3(get)]
    pub code: String,
    /// Unix seconds (file time scale)
    #[pyo3(get)]
    pub times: Vec<f64>,
    /// SNR values (dB-Hz)
    #[pyo3(get)]
    pub values: Vec<f64>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PySnrSeries {
    /// Epochs as ISO 8601 strings (computed on demand)
    fn epochs_iso(&self) -> Vec<String> {
        self.times
            .iter()
            .map(|t| {
                let secs = t.floor() as i64;
                let nanos = ((t - t.floor()) * 1e9) as u32;
                chrono::DateTime::from_timestamp(secs, nanos)
                    .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
                    .unwrap_or_default()
            })
            .collect()
    }

    fn __len__(&self) -> usize {
        self.values.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "SnrSeries(satellite='{}', code='{}', points={})",
            self.satellite,
            self.code,
            self.values.len()
        )
    }
}

/// Read RINEX observation file
#[cfg(feature = "python")]
#[pyfunction]
fn read_rinex_obs(py: Python<'_>, path: &str) -> PyResult<PyRinexObsData> {
    let path = path.to_string();
    let data = py
        .allow_threads(move || RinexObsReader::new().read(&path))
        .map_err(|e| PyIOError::new_err(format!("Failed to read RINEX: {}", e)))?;

    Ok(PyRinexObsData { inner: Arc::new(data) })
}

/// Read RINEX observation from bytes
#[cfg(feature = "python")]
#[pyfunction]
fn read_rinex_obs_bytes(py: Python<'_>, data: &[u8], filename: &str) -> PyResult<PyRinexObsData> {
    // Write to temp file and read
    use std::io::Write;
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(filename);
    
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| PyIOError::new_err(format!("Failed to create temp file: {}", e)))?;
    file.write_all(data)
        .map_err(|e| PyIOError::new_err(format!("Failed to write temp file: {}", e)))?;
    drop(file);
    
    let p = temp_path.clone();
    let obs_data = py
        .allow_threads(move || RinexObsReader::new().read(p.to_str().unwrap()))
        .map_err(|e| PyIOError::new_err(format!("Failed to parse RINEX: {}", e)))?;

    // Clean up
    let _ = std::fs::remove_file(&temp_path);

    Ok(PyRinexObsData { inner: Arc::new(obs_data) })
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
    m.add_class::<PySnrSeries>()?;
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
