//! GNSS analysis algorithms.
//!
//! This module provides:
//! - Multipath estimation using code-phase combinations
//! - Cycle slip detection
//! - Ionospheric analysis
//! - Position estimation using least squares

mod multipath;
mod cycle_slip;
mod position;

pub use multipath::{
    MultipathAnalyzer, MultipathEstimate, MultipathStatistics,
    AnalysisConfig, AnalysisResults, SummaryStatistics,
};
pub use cycle_slip::{CycleSlipDetector, CycleSlip, CycleSlipMethod};
pub use position::{PositionEstimator, PositionSolution, PositionStatistics};

/// Re-export commonly used types
pub use crate::utils::{Ecef, Epoch};

/// Statistics helper structure
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Statistics {
    pub count: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub rms: f64,
}

impl Statistics {
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self::default();
        }

        let n = values.len();
        let sum: f64 = values.iter().sum();
        let mean = sum / n as f64;
        let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let rms = (values.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();

        Self { count: n, mean, std_dev, min, max, rms }
    }
}

/// Ionospheric delay analyzer
pub struct IonosphericAnalyzer;

impl IonosphericAnalyzer {
    pub fn compute_iono_delay(p1: f64, p2: f64, f1: f64, f2: f64) -> f64 {
        let f1_sq = f1 * f1;
        let f2_sq = f2 * f2;
        (f2_sq / (f1_sq - f2_sq)) * (p1 - p2)
    }

    pub fn map_to_zenith(iono_delay: f64, elevation_deg: f64) -> f64 {
        let mapping = crate::utils::ionospheric_mapping(elevation_deg);
        iono_delay / mapping
    }
}
