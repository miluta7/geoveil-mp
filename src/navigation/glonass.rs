//! GLONASS state vector interpolation using 4th-order Runge-Kutta.
//!
//! GLONASS satellites transmit state vectors (position, velocity, acceleration)
//! rather than Keplerian elements. This module implements numerical integration
//! to propagate the state vector to any desired epoch.

use crate::rinex::Satellite;
use crate::utils::{
    constants::{GM_PZ90, J2_PZ90, OMEGA_EARTH_PZ90, EARTH_RADIUS_PZ90},
    Ecef, Epoch,
};

use super::types::{GlonassEphemeris, SatellitePosition};

/// GLONASS state vector interpolator using 4th-order Runge-Kutta
pub struct GlonassInterpolator {
    /// Integration step size (seconds)
    step_size: f64,
    /// Maximum propagation time (seconds)
    max_propagation: f64,
}

impl Default for GlonassInterpolator {
    fn default() -> Self {
        Self {
            step_size: 90.0,  // 90 seconds default
            max_propagation: 7200.0,  // 2 hours max
        }
    }
}

impl GlonassInterpolator {
    /// Create a new interpolator
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom step size
    pub fn with_step_size(step_size: f64) -> Self {
        Self {
            step_size,
            ..Default::default()
        }
    }

    /// Interpolate GLONASS ephemeris to target epoch
    pub fn interpolate(
        &self,
        satellite: Satellite,
        ephemeris: &GlonassEphemeris,
        target_epoch: &Epoch,
        leap_seconds: i32,
    ) -> SatellitePosition {
        // Convert GLONASS time to GPS time
        let glonass_epoch_utc = ephemeris.epoch.add_seconds(-3.0 * 3600.0);
        let glonass_epoch_gps = glonass_epoch_utc.add_seconds(leap_seconds as f64);

        // Time difference from ephemeris epoch
        let dt = target_epoch.diff(&glonass_epoch_gps);

        // Initial state vector (convert km to m, km/s to m/s, km/s² to m/s²)
        let mut state = StateVector {
            x: ephemeris.x * 1000.0,
            y: ephemeris.y * 1000.0,
            z: ephemeris.z * 1000.0,
            vx: ephemeris.vx * 1000.0,
            vy: ephemeris.vy * 1000.0,
            vz: ephemeris.vz * 1000.0,
        };

        // Acceleration (luni-solar perturbations) - constant during propagation
        let accel = Acceleration {
            jx: ephemeris.ax * 1000.0,
            jy: ephemeris.ay * 1000.0,
            jz: ephemeris.az * 1000.0,
        };

        // Propagate state vector using Runge-Kutta
        let remaining_time = dt;
        let step = if remaining_time >= 0.0 {
            self.step_size
        } else {
            -self.step_size
        };

        let mut time = 0.0;
        let abs_remaining = remaining_time.abs();

        while (remaining_time - time).abs() > 1e-9 && time.abs() < abs_remaining {
            let current_step = if (remaining_time - time).abs() < step.abs() {
                remaining_time - time
            } else {
                step
            };

            state = self.runge_kutta_step(&state, &accel, current_step);
            time += current_step;
        }

        // Clock correction
        let clock_bias = -ephemeris.tau_n + ephemeris.gamma_n * dt;
        let clock_drift = ephemeris.gamma_n;

        SatellitePosition {
            satellite,
            epoch: *target_epoch,
            position: Ecef::new(state.x, state.y, state.z),
            velocity: Some([state.vx, state.vy, state.vz]),
            clock_bias,
            clock_drift,
            relativistic_correction: 0.0,
        }
    }

    /// Perform one 4th-order Runge-Kutta integration step
    fn runge_kutta_step(
        &self,
        state: &StateVector,
        accel: &Acceleration,
        dt: f64,
    ) -> StateVector {
        let k1 = self.equations_of_motion(state, accel);
        
        let state2 = StateVector {
            x: state.x + k1.x * dt / 2.0,
            y: state.y + k1.y * dt / 2.0,
            z: state.z + k1.z * dt / 2.0,
            vx: state.vx + k1.vx * dt / 2.0,
            vy: state.vy + k1.vy * dt / 2.0,
            vz: state.vz + k1.vz * dt / 2.0,
        };
        let k2 = self.equations_of_motion(&state2, accel);
        
        let state3 = StateVector {
            x: state.x + k2.x * dt / 2.0,
            y: state.y + k2.y * dt / 2.0,
            z: state.z + k2.z * dt / 2.0,
            vx: state.vx + k2.vx * dt / 2.0,
            vy: state.vy + k2.vy * dt / 2.0,
            vz: state.vz + k2.vz * dt / 2.0,
        };
        let k3 = self.equations_of_motion(&state3, accel);
        
        let state4 = StateVector {
            x: state.x + k3.x * dt,
            y: state.y + k3.y * dt,
            z: state.z + k3.z * dt,
            vx: state.vx + k3.vx * dt,
            vy: state.vy + k3.vy * dt,
            vz: state.vz + k3.vz * dt,
        };
        let k4 = self.equations_of_motion(&state4, accel);

        StateVector {
            x: state.x + (k1.x + 2.0 * k2.x + 2.0 * k3.x + k4.x) * dt / 6.0,
            y: state.y + (k1.y + 2.0 * k2.y + 2.0 * k3.y + k4.y) * dt / 6.0,
            z: state.z + (k1.z + 2.0 * k2.z + 2.0 * k3.z + k4.z) * dt / 6.0,
            vx: state.vx + (k1.vx + 2.0 * k2.vx + 2.0 * k3.vx + k4.vx) * dt / 6.0,
            vy: state.vy + (k1.vy + 2.0 * k2.vy + 2.0 * k3.vy + k4.vy) * dt / 6.0,
            vz: state.vz + (k1.vz + 2.0 * k2.vz + 2.0 * k3.vz + k4.vz) * dt / 6.0,
        }
    }

    /// GLONASS equations of motion
    fn equations_of_motion(&self, state: &StateVector, accel: &Acceleration) -> StateDerivative {
        let x = state.x;
        let y = state.y;
        let z = state.z;
        let vx = state.vx;
        let vy = state.vy;

        let r = (x * x + y * y + z * z).sqrt();
        let r2 = r * r;
        let r3 = r2 * r;
        let r5 = r3 * r2;

        let a_grav = -GM_PZ90 / r3;

        let ae2 = EARTH_RADIUS_PZ90 * EARTH_RADIUS_PZ90;
        let z2_r2 = (z * z) / r2;
        let j2_factor = 1.5 * J2_PZ90 * GM_PZ90 * ae2 / r5 * (1.0 - 5.0 * z2_r2);

        let omega = OMEGA_EARTH_PZ90;
        let omega2 = omega * omega;

        let ax = a_grav * x + j2_factor * x + 2.0 * omega * vy + omega2 * x + accel.jx;
        let ay = a_grav * y + j2_factor * y - 2.0 * omega * vx + omega2 * y + accel.jy;
        
        let j2_z_factor = 1.5 * J2_PZ90 * GM_PZ90 * ae2 / r5 * (3.0 - 5.0 * z2_r2);
        let az = a_grav * z + j2_z_factor * z + accel.jz;

        StateDerivative {
            x: vx,
            y: vy,
            z: state.vz,
            vx: ax,
            vy: ay,
            vz: az,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct StateVector {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
}

#[derive(Debug, Clone, Copy)]
struct StateDerivative {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
}

#[derive(Debug, Clone, Copy)]
struct Acceleration {
    jx: f64,
    jy: f64,
    jz: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rinex::GnssSystem;

    #[test]
    fn test_glonass_interpolator_creation() {
        let interp = GlonassInterpolator::new();
        assert!((interp.step_size - 90.0).abs() < 0.001);
    }
}
