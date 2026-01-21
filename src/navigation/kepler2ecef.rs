//! Keplerian elements to ECEF coordinate transformation.
//!
//! Implements the conversion from broadcast ephemeris Keplerian elements
//! to Earth-Centered Earth-Fixed (ECEF) coordinates for GPS, Galileo,
//! BeiDou, QZSS, and NavIC satellites.

use crate::utils::{
    constants::{GM_WGS84, OMEGA_EARTH_WGS84, SPEED_OF_LIGHT, thresholds::KEPLER_CONVERGENCE},
    Ecef, Epoch,
};

use super::types::{KeplerianElements, SatellitePosition};
use crate::rinex::Satellite;

/// Kepler to ECEF coordinate converter
pub struct Kepler2Ecef {
    /// Maximum iterations for Kepler equation
    max_iterations: usize,
    /// Convergence threshold
    convergence_threshold: f64,
    /// Gravitational constant (can be overridden for BeiDou)
    gm: f64,
    /// Earth rotation rate
    omega_e: f64,
}

impl Default for Kepler2Ecef {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            convergence_threshold: KEPLER_CONVERGENCE,
            gm: GM_WGS84,
            omega_e: OMEGA_EARTH_WGS84,
        }
    }
}

impl Kepler2Ecef {
    /// Create a new converter
    pub fn new() -> Self {
        Self::default()
    }

    /// Create converter for BeiDou (uses CGCS2000 parameters)
    pub fn for_beidou() -> Self {
        Self {
            gm: 3.986004418e14,  // CGCS2000 GM
            omega_e: 7.2921150e-5,  // CGCS2000 omega
            ..Default::default()
        }
    }

    /// Convert Keplerian elements to ECEF at given time
    /// 
    /// # Arguments
    /// * `elements` - Keplerian orbital elements
    /// * `epoch` - Target epoch
    /// * `receiver_pos` - Optional receiver position for Sagnac correction
    /// 
    /// # Returns
    /// Satellite position in ECEF with clock corrections
    pub fn compute(
        &self,
        satellite: Satellite,
        elements: &KeplerianElements,
        epoch: &Epoch,
        receiver_pos: Option<&Ecef>,
    ) -> SatellitePosition {
        // Get GPS time in seconds
        let gps_time = epoch.to_gps_time();
        let tow_rec = gps_time.tow;

        // Time since ephemeris reference epoch (TOE)
        let mut tk = tow_rec - elements.toe;
        
        // Account for week rollover
        if tk > 302400.0 {
            tk -= 604800.0;
        } else if tk < -302400.0 {
            tk += 604800.0;
        }

        // Semi-major axis
        let a = elements.sqrt_a * elements.sqrt_a;

        // Mean motion (rad/s)
        let n0 = (self.gm / (a * a * a)).sqrt();
        
        // Corrected mean motion
        let n = n0 + elements.delta_n;

        // Mean anomaly at time tk
        let mk = elements.m0 + n * tk;

        // Solve Kepler's equation: E = M + e*sin(E)
        let ek = self.solve_kepler(mk, elements.e);

        // True anomaly
        let sin_ek = ek.sin();
        let cos_ek = ek.cos();
        let e = elements.e;
        
        let sin_vk = ((1.0 - e * e).sqrt() * sin_ek) / (1.0 - e * cos_ek);
        let cos_vk = (cos_ek - e) / (1.0 - e * cos_ek);
        let vk = sin_vk.atan2(cos_vk);

        // Argument of latitude
        let phi_k = vk + elements.omega;

        // Second harmonic corrections
        let sin_2phi = (2.0 * phi_k).sin();
        let cos_2phi = (2.0 * phi_k).cos();

        // Corrected argument of latitude
        let delta_uk = elements.cus * sin_2phi + elements.cuc * cos_2phi;
        let uk = phi_k + delta_uk;

        // Corrected radius
        let delta_rk = elements.crs * sin_2phi + elements.crc * cos_2phi;
        let rk = a * (1.0 - e * cos_ek) + delta_rk;

        // Corrected inclination
        let delta_ik = elements.cis * sin_2phi + elements.cic * cos_2phi;
        let ik = elements.i0 + delta_ik + elements.idot * tk;

        // Position in orbital plane
        let x_prime = rk * uk.cos();
        let y_prime = rk * uk.sin();

        // Longitude of ascending node
        let omega_k = elements.omega0 + (elements.omega_dot - self.omega_e) * tk 
                      - self.omega_e * elements.toe;

        // ECEF coordinates (without Sagnac correction)
        let sin_omega = omega_k.sin();
        let cos_omega = omega_k.cos();
        let sin_ik = ik.sin();
        let cos_ik = ik.cos();

        let mut x = x_prime * cos_omega - y_prime * cos_ik * sin_omega;
        let mut y = x_prime * sin_omega + y_prime * cos_ik * cos_omega;
        let mut z = y_prime * sin_ik;

        // Relativistic clock correction
        let rel_corr = -2.0 * (self.gm * a).sqrt() / (SPEED_OF_LIGHT * SPEED_OF_LIGHT) 
                       * e * sin_ek;

        // Clock correction
        let dt_sv = elements.af0 + elements.af1 * tk + elements.af2 * tk * tk;

        // Apply Sagnac correction if receiver position is known
        if let Some(rec_pos) = receiver_pos {
            let (x_corr, y_corr, z_corr) = self.sagnac_correction(
                x, y, z,
                rec_pos,
                elements,
                tk,
            );
            x = x_corr;
            y = y_corr;
            z = z_corr;
        }

        SatellitePosition {
            satellite,
            epoch: *epoch,
            position: Ecef::new(x, y, z),
            velocity: None,
            clock_bias: dt_sv,
            clock_drift: elements.af1 + 2.0 * elements.af2 * tk,
            relativistic_correction: rel_corr,
        }
    }

    /// Solve Kepler's equation iteratively
    /// E = M + e*sin(E)
    fn solve_kepler(&self, m: f64, e: f64) -> f64 {
        let mut ek = m; // Initial guess
        
        for _ in 0..self.max_iterations {
            let ek_new = m + e * ek.sin();
            
            if (ek_new - ek).abs() < self.convergence_threshold {
                return ek_new;
            }
            
            ek = ek_new;
        }
        
        ek
    }

    /// Apply Sagnac (Earth rotation) correction
    /// This iteratively adjusts satellite position for Earth rotation
    /// during signal travel time
    fn sagnac_correction(
        &self,
        x: f64,
        y: f64,
        z: f64,
        rec_pos: &Ecef,
        elements: &KeplerianElements,
        tk: f64,
    ) -> (f64, f64, f64) {
        let a = elements.sqrt_a * elements.sqrt_a;
        let e = elements.e;
        
        // Initial travel time estimate
        let mut trans_time = 0.075; // ~75 ms initial guess
        
        let (mut x_sat, mut y_sat, mut z_sat) = (x, y, z);
        
        for _ in 0..10 {
            // Recalculate longitude of ascending node with travel time
            let omega_k = elements.omega0 
                         + (elements.omega_dot - self.omega_e) * tk
                         - self.omega_e * (elements.toe + trans_time);

            // Mean anomaly
            let n0 = (self.gm / (a * a * a)).sqrt();
            let n = n0 + elements.delta_n;
            let mk = elements.m0 + n * tk;
            
            // Eccentric anomaly
            let ek = self.solve_kepler(mk, e);
            let sin_ek = ek.sin();
            let cos_ek = ek.cos();
            
            // True anomaly
            let sin_vk = ((1.0 - e * e).sqrt() * sin_ek) / (1.0 - e * cos_ek);
            let cos_vk = (cos_ek - e) / (1.0 - e * cos_ek);
            let vk = sin_vk.atan2(cos_vk);

            // Argument of latitude with corrections
            let phi_k = vk + elements.omega;
            let sin_2phi = (2.0 * phi_k).sin();
            let cos_2phi = (2.0 * phi_k).cos();
            let delta_uk = elements.cus * sin_2phi + elements.cuc * cos_2phi;
            let uk = phi_k + delta_uk;

            // Corrected radius
            let delta_rk = elements.crs * sin_2phi + elements.crc * cos_2phi;
            let rk = a * (1.0 - e * cos_ek) + delta_rk;

            // Corrected inclination
            let delta_ik = elements.cis * sin_2phi + elements.cic * cos_2phi;
            let ik = elements.i0 + delta_ik + elements.idot * tk;

            // Position in orbital plane
            let x_prime = rk * uk.cos();
            let y_prime = rk * uk.sin();

            // ECEF coordinates
            let sin_omega = omega_k.sin();
            let cos_omega = omega_k.cos();
            let cos_ik = ik.cos();
            let sin_ik = ik.sin();

            x_sat = x_prime * cos_omega - y_prime * cos_ik * sin_omega;
            y_sat = x_prime * sin_omega + y_prime * cos_ik * cos_omega;
            z_sat = y_prime * sin_ik;

            // Compute geometric distance
            let dx = x_sat - rec_pos.x;
            let dy = y_sat - rec_pos.y;
            let dz = z_sat - rec_pos.z;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            // Update travel time
            let new_trans = distance / SPEED_OF_LIGHT;
            
            if (new_trans - trans_time).abs() < 1e-10 {
                break;
            }
            
            trans_time = new_trans;
        }

        (x_sat, y_sat, z_sat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kepler_solver() {
        let converter = Kepler2Ecef::new();
        
        // Test with known values
        // Circular orbit: E should equal M
        let e = converter.solve_kepler(1.0, 0.0);
        assert!((e - 1.0).abs() < 1e-10);
        
        // Small eccentricity
        let e = converter.solve_kepler(1.0, 0.01);
        assert!((e - 1.0).abs() < 0.02); // Should be close to M
    }

    #[test]
    fn test_basic_conversion() {
        let converter = Kepler2Ecef::new();
        
        // Create test elements (approximate GPS satellite)
        let elements = KeplerianElements {
            sqrt_a: 5153.7,  // ~26559 km altitude
            e: 0.01,
            i0: 0.96,  // ~55 degrees
            omega0: 0.0,
            omega: 0.0,
            m0: 0.0,
            omega_dot: -8.0e-9,
            idot: 0.0,
            delta_n: 4.0e-9,
            cuc: 0.0,
            cus: 0.0,
            crc: 0.0,
            crs: 0.0,
            cic: 0.0,
            cis: 0.0,
            af0: 0.0,
            af1: 0.0,
            af2: 0.0,
            week: 2300,
            toe: 0.0,
            toc: 0.0,
            ..Default::default()
        };

        let sat = Satellite::new(crate::rinex::GnssSystem::Gps, 1);
        let epoch = Epoch::new(2024, 1, 1, 0, 0, 0.0);
        
        let pos = converter.compute(sat, &elements, &epoch, None);
        
        // Check that position magnitude is approximately correct for GPS orbit
        let magnitude = pos.position.magnitude();
        assert!(magnitude > 20_000_000.0 && magnitude < 30_000_000.0);
    }
}
