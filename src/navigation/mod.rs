//! Navigation and ephemeris processing.
//!
//! This module provides:
//! - Keplerian elements to ECEF conversion (GPS, Galileo, BeiDou, QZSS, NavIC)
//! - GLONASS state vector propagation using Runge-Kutta
//! - SP3 precise orbit interpolation using Neville's algorithm
//! - Navigation file readers

mod types;
mod kepler2ecef;
mod glonass;
mod sp3;

pub use types::*;
pub use kepler2ecef::Kepler2Ecef;
pub use glonass::GlonassInterpolator;
pub use sp3::{NevilleInterpolator, Sp3Reader};

use crate::rinex::{GnssSystem, ObservationData, Satellite};
use crate::utils::{Ecef, Epoch, Error, Result};

/// Satellite position provider (unified interface)
pub struct SatellitePositionProvider {
    /// Navigation data (broadcast ephemerides)
    nav_data: Option<NavigationData>,
    /// SP3 data (precise ephemerides)
    sp3_data: Option<Sp3Data>,
    /// Kepler to ECEF converter
    kepler_conv: Kepler2Ecef,
    /// GLONASS interpolator
    glonass_interp: GlonassInterpolator,
    /// Neville interpolator for SP3
    sp3_interp: NevilleInterpolator,
    /// Prefer SP3 over broadcast
    prefer_sp3: bool,
    /// GLONASS frequency channel map
    glonass_fcn: std::collections::HashMap<u32, i8>,
}

impl SatellitePositionProvider {
    /// Create a new provider with navigation data
    pub fn with_nav(nav_data: NavigationData) -> Self {
        Self {
            nav_data: Some(nav_data),
            sp3_data: None,
            kepler_conv: Kepler2Ecef::new(),
            glonass_interp: GlonassInterpolator::new(),
            sp3_interp: NevilleInterpolator::new(),
            prefer_sp3: false,
            glonass_fcn: std::collections::HashMap::new(),
        }
    }

    /// Create a new provider with SP3 data
    pub fn with_sp3(sp3_data: Sp3Data) -> Self {
        Self {
            nav_data: None,
            sp3_data: Some(sp3_data),
            kepler_conv: Kepler2Ecef::new(),
            glonass_interp: GlonassInterpolator::new(),
            sp3_interp: NevilleInterpolator::new(),
            prefer_sp3: true,
            glonass_fcn: std::collections::HashMap::new(),
        }
    }

    /// Add SP3 data to existing provider
    pub fn add_sp3(&mut self, sp3_data: Sp3Data) {
        self.sp3_data = Some(sp3_data);
        self.prefer_sp3 = true;
    }

    /// Set GLONASS frequency channel map (from observation file header)
    pub fn set_glonass_fcn(&mut self, fcn: std::collections::HashMap<u32, i8>) {
        self.glonass_fcn = fcn;
    }

    /// Get satellite position at epoch
    pub fn get_position(
        &self,
        satellite: &Satellite,
        epoch: &Epoch,
        receiver_pos: Option<&Ecef>,
    ) -> Result<SatellitePosition> {
        // Try SP3 first if preferred and available
        if self.prefer_sp3 {
            if let Some(ref sp3) = self.sp3_data {
                if let Some(pos) = self.sp3_interp.interpolate(sp3, satellite, epoch) {
                    return Ok(pos);
                }
            }
        }

        // Fall back to navigation data
        if let Some(ref nav) = self.nav_data {
            return self.compute_from_nav(nav, satellite, epoch, receiver_pos);
        }

        // If not preferring SP3, try it now
        if !self.prefer_sp3 {
            if let Some(ref sp3) = self.sp3_data {
                if let Some(pos) = self.sp3_interp.interpolate(sp3, satellite, epoch) {
                    return Ok(pos);
                }
            }
        }

        Err(Error::satellite_not_found(
            satellite.to_string(),
            epoch.to_string(),
        ))
    }

    /// Compute position from navigation data
    fn compute_from_nav(
        &self,
        nav: &NavigationData,
        satellite: &Satellite,
        epoch: &Epoch,
        receiver_pos: Option<&Ecef>,
    ) -> Result<SatellitePosition> {
        let eph = nav.get_ephemeris(satellite, epoch).ok_or_else(|| {
            Error::satellite_not_found(satellite.to_string(), epoch.to_string())
        })?;

        match eph {
            Ephemeris::Keplerian(kepler) => {
                let converter = match satellite.system {
                    GnssSystem::Beidou => Kepler2Ecef::for_beidou(),
                    _ => Kepler2Ecef::new(),
                };
                Ok(converter.compute(*satellite, kepler, epoch, receiver_pos))
            }
            Ephemeris::Glonass(glo) => {
                let leap = nav.leap_seconds.unwrap_or(18);
                Ok(self.glonass_interp.interpolate(*satellite, glo, epoch, leap))
            }
        }
    }

    /// Get positions for all satellites at an epoch
    pub fn get_all_positions(
        &self,
        epoch: &Epoch,
        receiver_pos: Option<&Ecef>,
    ) -> Vec<SatellitePosition> {
        let mut positions = Vec::new();

        // Get list of available satellites
        let satellites: Vec<Satellite> = if let Some(ref sp3) = self.sp3_data {
            sp3.satellites.clone()
        } else if let Some(ref nav) = self.nav_data {
            nav.ephemerides.keys().cloned().collect()
        } else {
            return positions;
        };

        for sat in satellites {
            if let Ok(pos) = self.get_position(&sat, epoch, receiver_pos) {
                positions.push(pos);
            }
        }

        positions
    }
}

/// Type alias for the SP3 file structure
pub type Sp3File = Sp3Data;
