//! RINEX file parsing and handling.
//!
//! This module provides comprehensive support for reading RINEX files:
//! - Observation files (RINEX 2.xx, 3.xx, 4.xx)
//! - Navigation files (broadcast ephemerides)
//! - Compression support (gzip, Hatanaka)

mod types;
mod obs_reader;

pub use types::*;
pub use obs_reader::{RinexObsReader, read_rinex_obs};

/// Type alias for the main RINEX observation data structure
pub type RinexObservation = ObservationData;

/// Type alias for navigation data (implemented in navigation module)
pub type RinexNavigation = ();
