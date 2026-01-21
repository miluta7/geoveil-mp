//! GNSS position estimation using least squares.
//!
//! Implements Single Point Positioning (SPP) using pseudorange observations
//! and least squares adjustment.

use std::collections::HashMap;
use nalgebra::{DMatrix, DVector, Matrix4, Vector4};
use serde::{Deserialize, Serialize};

use crate::rinex::{
    EpochObservations, GnssSystem, ObservationData, ObservationType, 
    Satellite, SignalCode,
};
use crate::navigation::{SatellitePosition, SatellitePositionProvider};
use crate::utils::{
    constants::{SPEED_OF_LIGHT, thresholds},
    calculate_azel, Ecef, Epoch, Error, Result,
};

/// Position solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSolution {
    /// Estimated position (ECEF)
    pub position: Ecef,
    /// Receiver clock bias (seconds)
    pub clock_bias: f64,
    /// Estimated velocity (ECEF, m/s) if available
    pub velocity: Option<[f64; 3]>,
    /// Epoch
    pub epoch: Epoch,
    /// Number of satellites used
    pub num_satellites: usize,
    /// Statistical information
    pub statistics: PositionStatistics,
    /// Satellites used in solution
    pub satellites_used: Vec<Satellite>,
}

/// Position estimation statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PositionStatistics {
    /// Residuals for each satellite
    pub residuals: Vec<f64>,
    /// Sum of squared errors
    pub sse: f64,
    /// Standard deviation of unit weight
    pub sigma0: f64,
    /// Position standard deviations (X, Y, Z) in meters
    pub std_x: f64,
    pub std_y: f64,
    pub std_z: f64,
    /// Clock bias standard deviation (seconds)
    pub std_clock: f64,
    /// Position Dilution of Precision
    pub pdop: f64,
    /// Time Dilution of Precision
    pub tdop: f64,
    /// Geometric Dilution of Precision
    pub gdop: f64,
    /// Horizontal Dilution of Precision
    pub hdop: f64,
    /// Vertical Dilution of Precision
    pub vdop: f64,
}

/// Position estimator
pub struct PositionEstimator {
    /// Elevation cutoff angle (degrees)
    elevation_cutoff: f64,
    /// Maximum iterations
    max_iterations: usize,
    /// Convergence threshold (meters)
    convergence_threshold: f64,
    /// Systems to use
    systems: Vec<GnssSystem>,
}

impl Default for PositionEstimator {
    fn default() -> Self {
        Self {
            elevation_cutoff: thresholds::ELEVATION_CUTOFF,
            max_iterations: thresholds::MAX_ITERATIONS,
            convergence_threshold: thresholds::POSITION_CONVERGENCE,
            systems: vec![GnssSystem::Gps],
        }
    }
}

impl PositionEstimator {
    /// Create a new estimator
    pub fn new() -> Self {
        Self::default()
    }

    /// Set elevation cutoff
    pub fn with_elevation_cutoff(mut self, cutoff: f64) -> Self {
        self.elevation_cutoff = cutoff;
        self
    }

    /// Set systems to use
    pub fn with_systems(mut self, systems: Vec<GnssSystem>) -> Self {
        self.systems = systems;
        self
    }

    /// Estimate position for a single epoch
    pub fn estimate(
        &self,
        epoch_obs: &EpochObservations,
        sat_provider: &SatellitePositionProvider,
        initial_pos: Option<&Ecef>,
    ) -> Result<PositionSolution> {
        // Get pseudorange observations
        let mut observations: Vec<(Satellite, f64, SatellitePosition)> = Vec::new();

        for (sat, sat_obs) in &epoch_obs.satellites {
            // Check if system is enabled
            if !self.systems.contains(&sat.system) {
                continue;
            }

            // Find a pseudorange observation (prefer C1C, then others)
            let pseudorange = self.get_best_pseudorange(sat_obs)?;
            
            if pseudorange.is_none() {
                continue;
            }
            let (code, pr) = pseudorange.unwrap();

            // Get satellite position
            let sat_pos = match sat_provider.get_position(sat, &epoch_obs.epoch, initial_pos) {
                Ok(p) => p,
                Err(_) => continue,
            };

            observations.push((*sat, pr, sat_pos));
        }

        if observations.len() < 4 {
            return Err(Error::InsufficientData(format!(
                "Need at least 4 satellites, got {}",
                observations.len()
            )));
        }

        // Initial position estimate
        let mut pos = initial_pos.cloned().unwrap_or(Ecef::new(0.0, 0.0, 0.0));
        let mut clock_bias = 0.0;

        // Iterative least squares
        for iteration in 0..self.max_iterations {
            // Filter by elevation
            let valid_obs: Vec<&(Satellite, f64, SatellitePosition)> = observations
                .iter()
                .filter(|(_, _, sat_pos)| {
                    if pos.magnitude() > 1000.0 {
                        let azel = calculate_azel(&pos, &sat_pos.position);
                        azel.elevation >= self.elevation_cutoff
                    } else {
                        true // Accept all if no valid position yet
                    }
                })
                .collect();

            if valid_obs.len() < 4 {
                return Err(Error::InsufficientData(format!(
                    "Only {} satellites above elevation cutoff",
                    valid_obs.len()
                )));
            }

            let n = valid_obs.len();

            // Build design matrix A and observation vector L
            let mut a_data = Vec::with_capacity(n * 4);
            let mut l_data = Vec::with_capacity(n);

            for (_, pr, sat_pos) in valid_obs.iter() {
                // Geometric range
                let rho = pos.distance(&sat_pos.position);
                
                // Direction cosines
                let dx = sat_pos.position.x - pos.x;
                let dy = sat_pos.position.y - pos.y;
                let dz = sat_pos.position.z - pos.z;

                // Design matrix row: [-dx/rho, -dy/rho, -dz/rho, c]
                a_data.push(-dx / rho);
                a_data.push(-dy / rho);
                a_data.push(-dz / rho);
                a_data.push(SPEED_OF_LIGHT);

                // Observation: pseudorange - computed range - satellite clock correction
                let clock_corr = sat_pos.total_clock_correction() * SPEED_OF_LIGHT;
                let computed = rho + clock_bias * SPEED_OF_LIGHT - clock_corr;
                l_data.push(*pr - computed);
            }

            // Create matrices
            let a = DMatrix::from_row_slice(n, 4, &a_data);
            let l = DVector::from_vec(l_data);

            // Normal equations: N = A'A, h = A'L
            let at = a.transpose();
            let n_mat = &at * &a;
            let h = &at * &l;

            // Solve: dx = N^(-1) * h
            let n_inv = match n_mat.try_inverse() {
                Some(inv) => inv,
                None => return Err(Error::ComputationError("Singular normal matrix".to_string())),
            };

            let dx = &n_inv * &h;

            // Update position and clock
            pos.x += dx[0];
            pos.y += dx[1];
            pos.z += dx[2];
            clock_bias += dx[3] / SPEED_OF_LIGHT;

            // Check convergence
            let improvement = (dx[0].powi(2) + dx[1].powi(2) + dx[2].powi(2)).sqrt();
            if improvement < self.convergence_threshold {
                // Compute final statistics
                let stats = self.compute_statistics(&a, &l, &n_inv, n);

                let satellites_used: Vec<Satellite> = valid_obs
                    .iter()
                    .map(|(sat, _, _)| *sat)
                    .collect();

                return Ok(PositionSolution {
                    position: pos,
                    clock_bias,
                    velocity: None,
                    epoch: epoch_obs.epoch,
                    num_satellites: n,
                    statistics: stats,
                    satellites_used,
                });
            }
        }

        Err(Error::convergence_failure(
            self.max_iterations,
            "Position estimation did not converge",
        ))
    }

    /// Get best pseudorange observation
    fn get_best_pseudorange(
        &self,
        obs: &HashMap<SignalCode, crate::rinex::ObservationValue>,
    ) -> Result<Option<(SignalCode, f64)>> {
        // Priority order for pseudoranges
        let priorities = ["C1C", "C1W", "C1X", "C2W", "C2C", "C5X", "C5Q"];

        for code_str in &priorities {
            if let Some(code) = SignalCode::parse(code_str) {
                if let Some(value) = obs.get(&code) {
                    if value.value > 1e6 && value.value < 1e8 {
                        // Valid range (roughly 1000km to 100000km)
                        return Ok(Some((code, value.value)));
                    }
                }
            }
        }

        // Try any code observation
        for (code, value) in obs {
            if code.is_code() && value.value > 1e6 && value.value < 1e8 {
                return Ok(Some((code.clone(), value.value)));
            }
        }

        Ok(None)
    }

    /// Compute position statistics
    fn compute_statistics(
        &self,
        a: &DMatrix<f64>,
        l: &DVector<f64>,
        q_xx: &DMatrix<f64>,
        n: usize,
    ) -> PositionStatistics {
        // Residuals: v = A*x - l (approximated as -l for final iteration)
        let residuals: Vec<f64> = l.iter().cloned().collect();
        
        // Sum of squared errors
        let sse: f64 = residuals.iter().map(|r| r * r).sum();
        
        // Degrees of freedom
        let dof = n.saturating_sub(4) as f64;
        
        // Standard deviation of unit weight
        let sigma0 = if dof > 0.0 {
            (sse / dof).sqrt()
        } else {
            0.0
        };

        // Covariance matrix: C_xx = sigma0^2 * Q_xx
        let c_xx = q_xx * sigma0 * sigma0;

        // Standard deviations
        let std_x = c_xx[(0, 0)].sqrt();
        let std_y = c_xx[(1, 1)].sqrt();
        let std_z = c_xx[(2, 2)].sqrt();
        let std_clock = c_xx[(3, 3)].sqrt() / SPEED_OF_LIGHT;

        // DOP values from cofactor matrix
        let q_x = q_xx[(0, 0)];
        let q_y = q_xx[(1, 1)];
        let q_z = q_xx[(2, 2)];
        let q_t = q_xx[(3, 3)] / (SPEED_OF_LIGHT * SPEED_OF_LIGHT);

        let pdop = (q_x + q_y + q_z).sqrt();
        let tdop = q_t.sqrt();
        let gdop = (q_x + q_y + q_z + q_t * SPEED_OF_LIGHT * SPEED_OF_LIGHT).sqrt();

        // HDOP and VDOP require conversion to local coordinates
        // Simplified: assume equal contribution
        let hdop = (q_x + q_y).sqrt();
        let vdop = q_z.sqrt();

        PositionStatistics {
            residuals,
            sse,
            sigma0,
            std_x,
            std_y,
            std_z,
            std_clock,
            pdop,
            tdop,
            gdop,
            hdop,
            vdop,
        }
    }

    /// Estimate positions for all epochs
    pub fn estimate_all(
        &self,
        obs_data: &ObservationData,
        sat_provider: &SatellitePositionProvider,
    ) -> Vec<PositionSolution> {
        let mut solutions = Vec::new();
        let mut prev_pos: Option<Ecef> = obs_data.header.approx_position.clone();

        for epoch_obs in &obs_data.epochs {
            match self.estimate(epoch_obs, sat_provider, prev_pos.as_ref()) {
                Ok(sol) => {
                    prev_pos = Some(sol.position);
                    solutions.push(sol);
                }
                Err(_) => {
                    // Skip failed epochs
                    continue;
                }
            }
        }

        solutions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimator_creation() {
        let est = PositionEstimator::new();
        assert!((est.elevation_cutoff - thresholds::ELEVATION_CUTOFF).abs() < 0.001);
    }

    #[test]
    fn test_custom_settings() {
        let est = PositionEstimator::new()
            .with_elevation_cutoff(15.0)
            .with_systems(vec![GnssSystem::Gps, GnssSystem::Galileo]);
        
        assert!((est.elevation_cutoff - 15.0).abs() < 0.001);
        assert_eq!(est.systems.len(), 2);
    }
}
