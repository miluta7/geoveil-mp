//! Neville's algorithm for polynomial interpolation of SP3 precise ephemerides.
//!
//! SP3 files provide satellite positions at discrete epochs (typically 15-minute
//! intervals). This module implements Neville's algorithm to interpolate
//! positions to any desired epoch.

use crate::rinex::Satellite;
use crate::utils::{Ecef, Epoch};

use super::types::{Sp3Data, Sp3Entry, SatellitePosition};

/// SP3 interpolator using Neville's algorithm
pub struct NevilleInterpolator {
    /// Number of points to use for interpolation
    num_points: usize,
}

impl Default for NevilleInterpolator {
    fn default() -> Self {
        Self { num_points: 9 }
    }
}

impl NevilleInterpolator {
    /// Create a new interpolator
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom number of interpolation points
    /// 
    /// # Arguments
    /// * `num_points` - Number of points to use (typically 7-11)
    pub fn with_points(num_points: usize) -> Self {
        Self { 
            num_points: num_points.max(3).min(15) 
        }
    }

    /// Interpolate SP3 data to target epoch
    ///
    /// # Arguments
    /// * `sp3` - SP3 data
    /// * `satellite` - Target satellite
    /// * `epoch` - Target epoch
    ///
    /// # Returns
    /// Interpolated satellite position, or None if insufficient data
    pub fn interpolate(
        &self,
        sp3: &Sp3Data,
        satellite: &Satellite,
        epoch: &Epoch,
    ) -> Option<SatellitePosition> {
        // Get entries for this satellite
        let entries = sp3.get_satellite(satellite)?;
        
        if entries.len() < self.num_points {
            return None;
        }

        // Find nearest entries to target epoch
        let nearest = self.find_nearest_entries(entries, epoch);
        
        if nearest.len() < 3 {
            return None;
        }

        // Convert epochs to seconds for interpolation
        let target_seconds = epoch.to_julian_date() * 86400.0;
        
        let times: Vec<f64> = nearest
            .iter()
            .map(|e| e.epoch.to_julian_date() * 86400.0)
            .collect();

        // Interpolate X, Y, Z, and clock
        let x_values: Vec<f64> = nearest.iter().map(|e| e.x).collect();
        let y_values: Vec<f64> = nearest.iter().map(|e| e.y).collect();
        let z_values: Vec<f64> = nearest.iter().map(|e| e.z).collect();
        let clock_values: Vec<f64> = nearest.iter().map(|e| e.clock).collect();

        let x = self.neville_interpolate(&times, &x_values, target_seconds);
        let y = self.neville_interpolate(&times, &y_values, target_seconds);
        let z = self.neville_interpolate(&times, &z_values, target_seconds);
        let clock = self.neville_interpolate(&times, &clock_values, target_seconds);

        // Calculate velocity if possible (using finite differences)
        let velocity = if nearest.len() >= 2 {
            let dt = 1.0; // 1 second for velocity calculation
            let target_plus = target_seconds + dt;
            let target_minus = target_seconds - dt;
            
            let x_plus = self.neville_interpolate(&times, &x_values, target_plus);
            let x_minus = self.neville_interpolate(&times, &x_values, target_minus);
            let y_plus = self.neville_interpolate(&times, &y_values, target_plus);
            let y_minus = self.neville_interpolate(&times, &y_values, target_minus);
            let z_plus = self.neville_interpolate(&times, &z_values, target_plus);
            let z_minus = self.neville_interpolate(&times, &z_values, target_minus);
            
            Some([
                (x_plus - x_minus) / (2.0 * dt),
                (y_plus - y_minus) / (2.0 * dt),
                (z_plus - z_minus) / (2.0 * dt),
            ])
        } else {
            None
        };

        Some(SatellitePosition {
            satellite: *satellite,
            epoch: *epoch,
            position: Ecef::new(x, y, z),
            velocity,
            clock_bias: clock * 1e-6, // Convert from microseconds to seconds
            clock_drift: 0.0,
            relativistic_correction: 0.0,
        })
    }

    /// Find nearest entries to target epoch
    fn find_nearest_entries<'a>(
        &self,
        entries: &'a [Sp3Entry],
        epoch: &Epoch,
    ) -> Vec<&'a Sp3Entry> {
        let mut sorted_entries: Vec<&Sp3Entry> = entries.iter().collect();
        
        // Sort by distance from target epoch
        sorted_entries.sort_by(|a, b| {
            let diff_a = epoch.diff(&a.epoch).abs();
            let diff_b = epoch.diff(&b.epoch).abs();
            diff_a.partial_cmp(&diff_b).unwrap()
        });

        // Take nearest points
        let mut nearest: Vec<&Sp3Entry> = sorted_entries
            .into_iter()
            .take(self.num_points)
            .collect();

        // Sort by time for interpolation
        nearest.sort_by(|a, b| {
            a.epoch.partial_cmp(&b.epoch).unwrap()
        });

        nearest
    }

    /// Neville's polynomial interpolation algorithm
    ///
    /// # Arguments
    /// * `x` - Known x values (times)
    /// * `y` - Known y values (data)
    /// * `target` - Target x value
    ///
    /// # Returns
    /// Interpolated y value at target
    fn neville_interpolate(&self, x: &[f64], y: &[f64], target: f64) -> f64 {
        let n = x.len();
        
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return y[0];
        }

        // Create working array
        let mut p: Vec<f64> = y.to_vec();
        
        // Compute time differences from target
        let dx: Vec<f64> = x.iter().map(|&xi| xi - target).collect();

        // Neville's algorithm
        for j in 1..n {
            for i in 0..(n - j) {
                // p[i] = ((x[i+j] - target) * p[i] - (x[i] - target) * p[i+1]) / (x[i+j] - x[i])
                p[i] = (dx[i + j] * p[i] - dx[i] * p[i + 1]) / (x[i + j] - x[i]);
            }
        }

        p[0]
    }

    /// Lagrange interpolation (alternative method)
    #[allow(dead_code)]
    fn lagrange_interpolate(&self, x: &[f64], y: &[f64], target: f64) -> f64 {
        let n = x.len();
        let mut result = 0.0;

        for i in 0..n {
            let mut term = y[i];
            for j in 0..n {
                if i != j {
                    term *= (target - x[j]) / (x[i] - x[j]);
                }
            }
            result += term;
        }

        result
    }
}

/// SP3 file reader
pub struct Sp3Reader;

impl Sp3Reader {
    /// Read SP3 file
    pub fn read<P: AsRef<std::path::Path>>(path: P) -> crate::utils::Result<Sp3Data> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use crate::utils::Error;

        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        
        let mut sp3 = Sp3Data::default();
        let mut current_epoch: Option<Epoch> = None;

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            
            if line.is_empty() {
                continue;
            }

            let first_char = line.chars().next().unwrap_or(' ');

            match first_char {
                '#' => {
                    // Header line
                    if line.len() >= 2 {
                        let second_char = line.chars().nth(1).unwrap_or(' ');
                        match second_char {
                            'a' | 'b' | 'c' | 'd' => {
                                sp3.version = second_char;
                                // Parse first header line
                                if line.len() >= 60 {
                                    sp3.pos_vel_flag = line.chars().nth(2).unwrap_or('P');
                                    // Parse start epoch
                                    if let Ok(epoch) = Self::parse_epoch(&line[3..32]) {
                                        sp3.start_epoch = Some(epoch);
                                    }
                                    sp3.num_epochs = line[32..39].trim().parse().unwrap_or(0);
                                }
                            }
                            '#' => {
                                // Second header line: week, seconds, interval, MJD
                                if line.len() >= 60 {
                                    sp3.gps_week = line[3..7].trim().parse().unwrap_or(0);
                                    sp3.seconds_of_week = line[8..23].trim().parse().unwrap_or(0.0);
                                    sp3.interval = line[24..38].trim().parse().unwrap_or(900.0);
                                    sp3.mjd = line[39..44].trim().parse().unwrap_or(0.0);
                                    sp3.fractional_day = line[45..60].trim().parse().unwrap_or(0.0);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                '+' => {
                    // Satellite list or accuracy
                    if line.len() >= 3 && line[1..3].trim().chars().all(|c| c.is_ascii_digit()) {
                        // Satellite list line
                        Self::parse_satellite_list(&line, &mut sp3);
                    }
                }
                '*' => {
                    // Epoch record
                    if let Ok(epoch) = Self::parse_epoch(&line[3..31]) {
                        current_epoch = Some(epoch);
                    }
                }
                'P' => {
                    // Position record
                    if let Some(epoch) = current_epoch {
                        if let Some((sat, entry)) = Self::parse_position_record(&line, epoch)? {
                            sp3.entries.entry(sat).or_insert_with(Vec::new).push(entry);
                        }
                    }
                }
                'V' => {
                    // Velocity record (if present)
                    // Can be added to previous position entry
                }
                '%' => {
                    // Comment or file type
                    if line.starts_with("%c") && line.len() >= 5 {
                        sp3.file_type = line.chars().nth(3).unwrap_or('M');
                        if line.len() >= 9 {
                            sp3.time_system = line[5..8].trim().to_string();
                        }
                    }
                }
                _ => {}
            }
        }

        // Count satellites
        sp3.num_satellites = sp3.entries.len();
        sp3.satellites = sp3.entries.keys().cloned().collect();

        Ok(sp3)
    }

    /// Parse epoch from SP3 format
    fn parse_epoch(s: &str) -> Result<Epoch, crate::utils::Error> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() < 6 {
            return Err(crate::utils::Error::InvalidEpoch(s.to_string()));
        }

        Ok(Epoch {
            year: parts[0].parse()?,
            month: parts[1].parse()?,
            day: parts[2].parse()?,
            hour: parts[3].parse()?,
            minute: parts[4].parse()?,
            second: parts[5].parse()?,
        })
    }

    /// Parse satellite list line
    fn parse_satellite_list(line: &str, sp3: &mut Sp3Data) {
        // Format: +   XX sat1sat2sat3...
        if line.len() < 10 {
            return;
        }

        let sat_str = &line[9..];
        for i in (0..sat_str.len()).step_by(3) {
            let end = (i + 3).min(sat_str.len());
            let sat_id = sat_str[i..end].trim();
            if let Some(sat) = Satellite::parse(sat_id) {
                if !sp3.satellites.contains(&sat) {
                    sp3.satellites.push(sat);
                }
            }
        }
    }

    /// Parse position record
    fn parse_position_record(
        line: &str, 
        epoch: Epoch
    ) -> Result<Option<(Satellite, Sp3Entry)>, crate::utils::Error> {
        if line.len() < 60 {
            return Ok(None);
        }

        let sat_id = &line[1..4];
        let satellite = match Satellite::parse(sat_id) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Parse position (km) and clock (microseconds)
        let x: f64 = line[4..18].trim().parse().unwrap_or(0.0) * 1000.0; // km to m
        let y: f64 = line[18..32].trim().parse().unwrap_or(0.0) * 1000.0;
        let z: f64 = line[32..46].trim().parse().unwrap_or(0.0) * 1000.0;
        let clock: f64 = line[46..60].trim().parse().unwrap_or(999999.999999);

        // Check for bad data (999999)
        if clock.abs() > 900000.0 {
            return Ok(None);
        }

        let entry = Sp3Entry {
            epoch,
            x,
            y,
            z,
            clock,
            x_sdev: None,
            y_sdev: None,
            z_sdev: None,
            clock_sdev: None,
            vx: None,
            vy: None,
            vz: None,
        };

        Ok(Some((satellite, entry)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neville_linear() {
        let interp = NevilleInterpolator::new();
        
        // Linear function y = 2x + 1
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 3.0, 5.0];
        
        let result = interp.neville_interpolate(&x, &y, 1.5);
        assert!((result - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_neville_quadratic() {
        let interp = NevilleInterpolator::new();
        
        // Quadratic function y = x^2
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
        
        let result = interp.neville_interpolate(&x, &y, 2.5);
        assert!((result - 6.25).abs() < 1e-10);
    }

    #[test]
    fn test_interpolator_creation() {
        let interp = NevilleInterpolator::new();
        assert_eq!(interp.num_points, 9);

        let interp2 = NevilleInterpolator::with_points(7);
        assert_eq!(interp2.num_points, 7);
    }
}
