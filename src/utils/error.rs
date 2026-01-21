//! Error handling and result types for the GNSS multipath library.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Custom error types for the GNSS multipath library
#[derive(Error, Debug)]
pub enum Error {
    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// RINEX parsing errors
    #[error("RINEX parsing error at line {line}: {message}")]
    RinexParse { line: usize, message: String },

    /// RINEX version not supported
    #[error("Unsupported RINEX version: {0}")]
    UnsupportedRinexVersion(String),

    /// Invalid RINEX header
    #[error("Invalid RINEX header: {0}")]
    InvalidHeader(String),

    /// Missing required header field
    #[error("Missing required header field: {0}")]
    MissingHeaderField(String),

    /// SP3 parsing errors
    #[error("SP3 parsing error at line {line}: {message}")]
    Sp3Parse { line: usize, message: String },

    /// Navigation file errors
    #[error("Navigation file error: {0}")]
    NavigationError(String),

    /// Invalid epoch format
    #[error("Invalid epoch format: {0}")]
    InvalidEpoch(String),

    /// Invalid satellite identifier
    #[error("Invalid satellite identifier: {0}")]
    InvalidSatellite(String),

    /// Invalid observation code
    #[error("Invalid observation code: {0}")]
    InvalidObservationCode(String),

    /// Invalid GNSS system
    #[error("Invalid GNSS system: {0}")]
    InvalidGnssSystem(char),

    /// Insufficient data for computation
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    /// Computation error (e.g., singular matrix)
    #[error("Computation error: {0}")]
    ComputationError(String),

    /// Convergence failure
    #[error("Failed to converge after {iterations} iterations: {message}")]
    ConvergenceFailure { iterations: usize, message: String },

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Invalid file format
    #[error("Invalid file format: expected {expected}, got {actual}")]
    InvalidFileFormat { expected: String, actual: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// R plotting error
    #[error("R plotting error: {0}")]
    PlottingError(String),

    /// Coordinate transformation error
    #[error("Coordinate transformation error: {0}")]
    CoordinateError(String),

    /// HTTP/download error
    #[error("Download error: {0}")]
    DownloadError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Time scale conversion error
    #[error("Time scale error: {0}")]
    TimeScaleError(String),

    /// Satellite not found in ephemeris
    #[error("Satellite {sat} not found in ephemeris at epoch {epoch}")]
    SatelliteNotFound { sat: String, epoch: String },

    /// Invalid frequency/band
    #[error("Invalid frequency band: {0}")]
    InvalidFrequency(String),

    /// Cycle slip detected (informational)
    #[error("Cycle slip detected for {satellite} at epoch {epoch}")]
    CycleSlipDetected { satellite: String, epoch: String },

    /// Generic error wrapper
    #[error("{0}")]
    Generic(String),
}

/// Result type alias for library operations
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a RINEX parse error
    pub fn rinex_parse(line: usize, message: impl Into<String>) -> Self {
        Error::RinexParse {
            line,
            message: message.into(),
        }
    }

    /// Create an SP3 parse error
    pub fn sp3_parse(line: usize, message: impl Into<String>) -> Self {
        Error::Sp3Parse {
            line,
            message: message.into(),
        }
    }

    /// Create a convergence failure error
    pub fn convergence_failure(iterations: usize, message: impl Into<String>) -> Self {
        Error::ConvergenceFailure {
            iterations,
            message: message.into(),
        }
    }

    /// Create a satellite not found error
    pub fn satellite_not_found(sat: impl Into<String>, epoch: impl Into<String>) -> Self {
        Error::SatelliteNotFound {
            sat: sat.into(),
            epoch: epoch.into(),
        }
    }
}

// Implement From for common error types
impl From<std::num::ParseFloatError> for Error {
    fn from(e: std::num::ParseFloatError) -> Self {
        Error::Generic(format!("Float parse error: {}", e))
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Error::Generic(format!("Integer parse error: {}", e))
    }
}

impl From<chrono::ParseError> for Error {
    fn from(e: chrono::ParseError) -> Self {
        Error::InvalidEpoch(format!("DateTime parse error: {}", e))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::SerializationError(format!("JSON error: {}", e))
    }
}

impl From<csv::Error> for Error {
    fn from(e: csv::Error) -> Self {
        Error::SerializationError(format!("CSV error: {}", e))
    }
}

#[cfg(feature = "http")]
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::DownloadError(format!("HTTP error: {}", e))
    }
}

/// Extension trait for adding context to errors
pub trait ResultExt<T> {
    /// Add context to an error
    fn context(self, msg: impl Into<String>) -> Result<T>;
    
    /// Add context with a closure (lazy evaluation)
    fn with_context<F: FnOnce() -> String>(self, f: F) -> Result<T>;
}

impl<T, E: Into<Error>> ResultExt<T> for std::result::Result<T, E> {
    fn context(self, msg: impl Into<String>) -> Result<T> {
        self.map_err(|e| {
            let inner = e.into();
            Error::Generic(format!("{}: {}", msg.into(), inner))
        })
    }

    fn with_context<F: FnOnce() -> String>(self, f: F) -> Result<T> {
        self.map_err(|e| {
            let inner = e.into();
            Error::Generic(format!("{}: {}", f(), inner))
        })
    }
}
