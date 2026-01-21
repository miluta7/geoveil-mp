//! Coordinate transformation utilities.
//!
//! Provides transformations between:
//! - ECEF (Earth-Centered, Earth-Fixed)
//! - Geodetic (Latitude, Longitude, Height)
//! - ENU (East-North-Up) local coordinates
//! - Azimuth and Elevation angles

use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use super::constants::{DEG_TO_RAD, EARTH_FLATTENING_WGS84, EARTH_RADIUS_WGS84, RAD_TO_DEG};
use super::error::Result;

/// ECEF coordinates (meters)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ecef {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Ecef {
    /// Create new ECEF coordinates
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Create from array
    pub fn from_array(arr: [f64; 3]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }

    /// Convert to array
    pub fn to_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    /// Convert to nalgebra Vector3
    pub fn to_vector(&self) -> Vector3<f64> {
        Vector3::new(self.x, self.y, self.z)
    }

    /// Create from nalgebra Vector3
    pub fn from_vector(v: &Vector3<f64>) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
        }
    }

    /// Distance to another point
    pub fn distance(&self, other: &Ecef) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Magnitude (distance from origin)
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Convert to geodetic coordinates (WGS84)
    pub fn to_geodetic(&self) -> Geodetic {
        ecef_to_geodetic(self)
    }

    /// Check if coordinates are valid (non-zero)
    pub fn is_valid(&self) -> bool {
        self.magnitude() > 1.0 // At least 1 meter from origin
    }
}

impl std::ops::Sub for Ecef {
    type Output = Ecef;

    fn sub(self, other: Self) -> Self::Output {
        Ecef::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

/// Geodetic coordinates (degrees and meters)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Geodetic {
    /// Latitude in degrees (-90 to +90)
    pub lat: f64,
    /// Longitude in degrees (-180 to +180)
    pub lon: f64,
    /// Height above ellipsoid in meters
    pub height: f64,
}

impl Geodetic {
    /// Create new geodetic coordinates
    pub fn new(lat: f64, lon: f64, height: f64) -> Self {
        Self { lat, lon, height }
    }

    /// Convert to ECEF coordinates (WGS84)
    pub fn to_ecef(&self) -> Ecef {
        geodetic_to_ecef(self)
    }

    /// Latitude in radians
    pub fn lat_rad(&self) -> f64 {
        self.lat * DEG_TO_RAD
    }

    /// Longitude in radians
    pub fn lon_rad(&self) -> f64 {
        self.lon * DEG_TO_RAD
    }
}

/// ENU (East-North-Up) local coordinates
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Enu {
    pub east: f64,
    pub north: f64,
    pub up: f64,
}

impl Enu {
    /// Create new ENU coordinates
    pub fn new(east: f64, north: f64, up: f64) -> Self {
        Self { east, north, up }
    }

    /// Convert to azimuth and elevation
    pub fn to_azel(&self) -> AzEl {
        enu_to_azel(self)
    }

    /// Horizontal distance
    pub fn horizontal_distance(&self) -> f64 {
        (self.east * self.east + self.north * self.north).sqrt()
    }

    /// Slant range (total distance)
    pub fn slant_range(&self) -> f64 {
        (self.east * self.east + self.north * self.north + self.up * self.up).sqrt()
    }
}

/// Azimuth and Elevation angles (degrees)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AzEl {
    /// Azimuth in degrees (0-360, measured clockwise from North)
    pub azimuth: f64,
    /// Elevation in degrees (-90 to +90)
    pub elevation: f64,
}

impl AzEl {
    /// Create new azimuth/elevation
    pub fn new(azimuth: f64, elevation: f64) -> Self {
        Self { azimuth, elevation }
    }

    /// Azimuth in radians
    pub fn azimuth_rad(&self) -> f64 {
        self.azimuth * DEG_TO_RAD
    }

    /// Elevation in radians
    pub fn elevation_rad(&self) -> f64 {
        self.elevation * DEG_TO_RAD
    }

    /// Check if above horizon
    pub fn is_visible(&self) -> bool {
        self.elevation > 0.0
    }

    /// Check if above given cutoff elevation
    pub fn above_cutoff(&self, cutoff: f64) -> bool {
        self.elevation >= cutoff
    }
}

/// Convert ECEF to geodetic coordinates (WGS84)
/// Uses iterative method for accuracy
pub fn ecef_to_geodetic(ecef: &Ecef) -> Geodetic {
    let a = EARTH_RADIUS_WGS84;
    let f = EARTH_FLATTENING_WGS84;
    let b = a * (1.0 - f); // Semi-minor axis
    let e2 = 2.0 * f - f * f; // First eccentricity squared
    let ep2 = (a * a - b * b) / (b * b); // Second eccentricity squared

    let p = (ecef.x * ecef.x + ecef.y * ecef.y).sqrt();
    let lon = ecef.y.atan2(ecef.x);

    // Initial estimate
    let theta = (ecef.z * a).atan2(p * b);
    let sin_theta = theta.sin();
    let cos_theta = theta.cos();

    let lat = (ecef.z + ep2 * b * sin_theta.powi(3))
        .atan2(p - e2 * a * cos_theta.powi(3));

    let sin_lat = lat.sin();
    let n = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();
    let height = p / lat.cos() - n;

    Geodetic {
        lat: lat * RAD_TO_DEG,
        lon: lon * RAD_TO_DEG,
        height,
    }
}

/// Convert geodetic to ECEF coordinates (WGS84)
pub fn geodetic_to_ecef(geo: &Geodetic) -> Ecef {
    let a = EARTH_RADIUS_WGS84;
    let f = EARTH_FLATTENING_WGS84;
    let e2 = 2.0 * f - f * f;

    let lat = geo.lat * DEG_TO_RAD;
    let lon = geo.lon * DEG_TO_RAD;

    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    let n = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();

    Ecef {
        x: (n + geo.height) * cos_lat * cos_lon,
        y: (n + geo.height) * cos_lat * sin_lon,
        z: (n * (1.0 - e2) + geo.height) * sin_lat,
    }
}

/// Convert ECEF difference to ENU coordinates
/// `origin` is the reference point (receiver position)
/// `target` is the target point (satellite position)
pub fn ecef_to_enu(origin: &Ecef, target: &Ecef) -> Enu {
    let geo = origin.to_geodetic();
    let lat = geo.lat * DEG_TO_RAD;
    let lon = geo.lon * DEG_TO_RAD;

    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    // Rotation matrix from ECEF to ENU
    let rot = Matrix3::new(
        -sin_lon,           cos_lon,          0.0,
        -sin_lat * cos_lon, -sin_lat * sin_lon, cos_lat,
        cos_lat * cos_lon,  cos_lat * sin_lon,  sin_lat,
    );

    let diff = Vector3::new(
        target.x - origin.x,
        target.y - origin.y,
        target.z - origin.z,
    );

    let enu = rot * diff;

    Enu {
        east: enu[0],
        north: enu[1],
        up: enu[2],
    }
}

/// Convert ENU to azimuth and elevation
pub fn enu_to_azel(enu: &Enu) -> AzEl {
    let horizontal = (enu.east * enu.east + enu.north * enu.north).sqrt();
    let slant = (horizontal * horizontal + enu.up * enu.up).sqrt();

    // Azimuth: measured clockwise from North
    let mut azimuth = enu.east.atan2(enu.north) * RAD_TO_DEG;
    if azimuth < 0.0 {
        azimuth += 360.0;
    }

    // Elevation: angle above horizon
    let elevation = if slant > 0.0 {
        (enu.up / slant).asin() * RAD_TO_DEG
    } else {
        0.0
    };

    AzEl { azimuth, elevation }
}

/// Calculate azimuth and elevation from receiver to satellite
pub fn calculate_azel(receiver: &Ecef, satellite: &Ecef) -> AzEl {
    let enu = ecef_to_enu(receiver, satellite);
    enu_to_azel(&enu)
}

/// Rotation matrix for Earth rotation during signal travel time
/// Used for Sagnac correction
pub fn earth_rotation_matrix(omega_e: f64, dt: f64) -> Matrix3<f64> {
    let angle = omega_e * dt;
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    Matrix3::new(
        cos_a, sin_a, 0.0,
        -sin_a, cos_a, 0.0,
        0.0, 0.0, 1.0,
    )
}

/// Apply Earth rotation correction to satellite position
pub fn apply_earth_rotation(sat_pos: &Ecef, travel_time: f64) -> Ecef {
    use super::constants::OMEGA_EARTH_WGS84;
    
    let rot = earth_rotation_matrix(OMEGA_EARTH_WGS84, travel_time);
    let pos = sat_pos.to_vector();
    let corrected = rot * pos;
    
    Ecef::from_vector(&corrected)
}

/// Tropospheric mapping function (simple cosecant model)
pub fn tropospheric_mapping(elevation_deg: f64) -> f64 {
    let el_rad = elevation_deg * DEG_TO_RAD;
    if el_rad > 0.0 {
        1.0 / el_rad.sin()
    } else {
        1.0
    }
}

/// Ionospheric mapping function
pub fn ionospheric_mapping(elevation_deg: f64) -> f64 {
    let el_rad = elevation_deg * DEG_TO_RAD;
    let earth_radius = EARTH_RADIUS_WGS84;
    let iono_height = 350_000.0; // Ionospheric shell height (m)
    
    let sin_z = (earth_radius / (earth_radius + iono_height)) * el_rad.sin().acos().cos();
    1.0 / (1.0 - sin_z * sin_z).sqrt()
}

/// Elevation-dependent weighting function for least squares
/// Returns weight based on elevation angle
pub fn elevation_weight(elevation_deg: f64) -> f64 {
    let el_rad = elevation_deg * DEG_TO_RAD;
    
    if elevation_deg >= 30.0 {
        1.0
    } else if elevation_deg > 0.0 {
        let sin_el = el_rad.sin();
        1.0 / (4.0 * sin_el * sin_el)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_ecef_geodetic_roundtrip() {
        // Test point: roughly in Europe
        let geo = Geodetic::new(52.0, 13.0, 100.0);
        let ecef = geo.to_ecef();
        let geo_back = ecef.to_geodetic();

        assert_relative_eq!(geo.lat, geo_back.lat, epsilon = 1e-6);
        assert_relative_eq!(geo.lon, geo_back.lon, epsilon = 1e-6);
        assert_relative_eq!(geo.height, geo_back.height, epsilon = 1e-3);
    }

    #[test]
    fn test_zero_zero_position() {
        let geo = Geodetic::new(0.0, 0.0, 0.0);
        let ecef = geo.to_ecef();

        assert_relative_eq!(ecef.x, EARTH_RADIUS_WGS84, epsilon = 1.0);
        assert_relative_eq!(ecef.y, 0.0, epsilon = 1.0);
        assert_relative_eq!(ecef.z, 0.0, epsilon = 1.0);
    }

    #[test]
    fn test_north_pole() {
        let geo = Geodetic::new(90.0, 0.0, 0.0);
        let ecef = geo.to_ecef();

        assert_relative_eq!(ecef.x, 0.0, epsilon = 1.0);
        assert_relative_eq!(ecef.y, 0.0, epsilon = 1.0);
        assert!(ecef.z > 6_350_000.0); // Should be near semi-minor axis
    }

    #[test]
    fn test_azel_calculation() {
        // Receiver at origin
        let receiver = Geodetic::new(0.0, 0.0, 0.0).to_ecef();
        
        // Satellite directly above
        let satellite_above = Geodetic::new(0.0, 0.0, 20_200_000.0).to_ecef();
        let azel = calculate_azel(&receiver, &satellite_above);
        
        assert_relative_eq!(azel.elevation, 90.0, epsilon = 1.0);
    }

    #[test]
    fn test_elevation_weight() {
        assert_relative_eq!(elevation_weight(90.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(elevation_weight(45.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(elevation_weight(30.0), 1.0, epsilon = 0.001);
        assert!(elevation_weight(10.0) > 1.0); // Should be weighted higher
        assert_relative_eq!(elevation_weight(0.0), 0.0, epsilon = 0.001);
    }
}
