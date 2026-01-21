//! # GeoVeil-MP: GNSS Multipath Analysis Library
//!
//! A high-performance Rust library for analyzing multipath effects in GNSS observations.
//! Part of the GeoVeil suite for GNSS signal quality analysis.
//! 
//! ## Features
//! 
//! - **RINEX Support**: Full support for RINEX v2.xx, v3.xx, and v4.xx observation files
//! - **Navigation Files**: Support for broadcast ephemerides (RINEX NAV) and precise orbits (SP3)
//! - **Multi-GNSS**: GPS, GLONASS, Galileo, BeiDou, QZSS, NavIC/IRNSS, SBAS
//! - **Multipath Analysis**: Code multipath estimation using linear combinations
//! - **Cycle Slip Detection**: Ionospheric residuals and code-phase combinations
//! - **Position Estimation**: Least squares positioning from pseudoranges
//! - **R Plotting**: Integration with R for high-quality visualizations
//! - **Python Bindings**: Full Python API via PyO3
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use geoveil_mp::prelude::*;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load RINEX observation file
//!     let obs = RinexObsReader::new().read("observation.24o")?;
//!     
//!     // Configure analysis
//!     let config = AnalysisConfig::default()
//!         .with_elevation_cutoff(10.0)
//!         .with_systems(&["G", "E", "R", "C"]);
//!     
//!     // Run multipath analysis
//!     let analyzer = MultipathAnalyzer::new(obs, config);
//!     let results = analyzer.analyze()?;
//!     
//!     // Export results
//!     results.to_csv("results.csv")?;
//!     
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod rinex;
pub mod navigation;
pub mod analysis;
pub mod plotting;
pub mod utils;

#[cfg(feature = "python")]
pub mod python;

// Re-exports for convenience
pub use rinex::{
    RinexObsReader, ObservationData, 
    Satellite, GnssSystem, SignalCode, ObservationType,
    EpochObservations, ObservationValue, Header,
};

pub use navigation::{
    Sp3Data, Sp3Reader, Ephemeris, SatellitePosition,
    Kepler2Ecef, GlonassInterpolator, NevilleInterpolator,
    SatellitePositionProvider,
};

pub use analysis::{
    MultipathAnalyzer, MultipathEstimate, MultipathStatistics,
    CycleSlipDetector, CycleSlip, CycleSlipMethod,
    IonosphericAnalyzer, PositionEstimator, PositionSolution,
    AnalysisConfig, AnalysisResults, Statistics,
};

pub use plotting::{
    RPlotter, PlotConfig, ColorScheme,
};

pub use utils::{
    Ecef, Geodetic, AzEl, Epoch, GpsTime,
    calculate_azel, ecef_to_geodetic, geodetic_to_ecef,
    elevation_weight, ionospheric_mapping, tropospheric_mapping,
    Error, Result,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude module for common imports
pub mod prelude {
    pub use crate::{
        RinexObsReader, ObservationData,
        MultipathAnalyzer, AnalysisConfig, AnalysisResults,
        MultipathEstimate, MultipathStatistics,
        CycleSlipDetector, CycleSlip,
        GnssSystem, Satellite, SignalCode,
        Ecef, Geodetic, Epoch, 
        Error, Result,
    };
}
