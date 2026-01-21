# GeoVeil-MP: GNSS Multipath Analysis Library

[![Crates.io](https://img.shields.io/crates/v/geoveil-mp.svg)](https://crates.io/crates/geoveil-mp)
[![PyPI](https://img.shields.io/pypi/v/geoveil-mp.svg)](https://pypi.org/project/geoveil-mp/)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/python-3.8%2B-blue.svg)](https://www.python.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI/CD](https://github.com/miluta7/geoveil-mp/actions/workflows/ci.yml/badge.svg)](https://github.com/miluta7/geoveil-mp/actions)

A high-performance Rust library for GNSS multipath analysis with Python bindings. Part of the **GeoVeil** suite for GNSS signal quality analysis.

## Features

- 🛰️ **RINEX Support**: Full support for RINEX v2.xx, v3.xx, and v4.xx observation files
- 🌍 **Multi-GNSS**: GPS, GLONASS, Galileo, BeiDou, QZSS, NavIC/IRNSS, SBAS
- 📡 **Navigation Data**: 
  - Broadcast ephemerides (Keplerian elements)
  - GLONASS state vector propagation (4th-order Runge-Kutta)
  - SP3 precise orbit interpolation (Neville's algorithm)
- 📊 **Multipath Analysis**: Code multipath estimation using linear combinations
- ⚡ **Cycle Slip Detection**: Ionospheric residuals and code-phase combinations
- 📍 **Position Estimation**: Least squares SPP with DOP calculation
- 🎨 **Visualization**: R plotting integration for publication-quality figures
- 🚀 **High Performance**: Parallel processing with Rayon, memory-mapped I/O
- 🐍 **Python Bindings**: Full Python API via PyO3

## Installation

### Python (PyPI)

```bash
pip install geoveil-mp
```

### Rust (Cargo)

```toml
[dependencies]
geoveil_mp = "0.1"
```

Or with all features:

```toml
[dependencies]
geoveil_mp = { version = "0.1", features = ["full"] }
```

### From Source

```bash
# Clone the repository
git clone https://github.com/miluta7/geoveil-mp.git
cd geoveil-mp

# Build Rust library
cargo build --release

# Build Python wheel
pip install maturin
maturin develop --release --features python
```

## Quick Start

### Python Usage

```python
import geoveil_mp as gm

# Read RINEX observation file
obs = gm.read_rinex_obs("observation.24o")
print(f"Loaded {obs.num_epochs} epochs, {obs.num_satellites} satellites")

# Read SP3 precise ephemeris (optional, for accurate elevations)
sp3 = gm.read_sp3("ephemeris.sp3")

# Create analyzer with elevation cutoff
analyzer = gm.MultipathAnalyzer(
    obs,
    elevation_cutoff=10.0,
    systems=["G", "E", "R", "C"]  # GPS, Galileo, GLONASS, BeiDou
)

# Run multipath analysis
results = analyzer.analyze()

# Compute satellite elevations from SP3
if sp3 and obs.approx_position:
    computed, failed = results.compute_elevations(sp3, obs.approx_position)
    print(f"Elevations: {computed} computed, {failed} failed")

# Access results
print(f"Total estimates: {results.total_estimates()}")
print(f"Cycle slips detected: {results.total_cycle_slips()}")

# Print statistics by signal
for stat in results.statistics:
    print(f"{stat.signal}: RMS={stat.rms:.4f}m, Count={stat.count}")

# Export to CSV
import pandas as pd
data = [{
    'satellite': e.satellite,
    'epoch': e.epoch,
    'mp_value': e.mp_value,
    'elevation': e.elevation,
    'signal': e.signal
} for e in results.estimates]
df = pd.DataFrame(data)
df.to_csv("multipath_results.csv", index=False)
```

### Rust Usage

```rust
use geoveil_mp::{
    prelude::*,
    RinexObsReader, Sp3Reader,
    navigation::SatellitePositionProvider,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read RINEX observation file
    let obs_data = RinexObsReader::new().read("observation.24o")?;
    
    // Read SP3 precise ephemeris
    let sp3_data = Sp3Reader::read("ephemeris.sp3")?;
    
    // Configure analysis
    let config = AnalysisConfig::default()
        .with_elevation_cutoff(10.0)
        .with_systems(&["G", "E", "R", "C"]);
    
    // Run multipath analysis
    let analyzer = MultipathAnalyzer::new(obs_data, config);
    let results = analyzer.analyze()?;
    
    // Export results
    results.to_csv("results.csv")?;
    
    // Print statistics
    for (signal, stats) in &results.statistics {
        println!("{}: RMS={:.4}m, Count={}", signal, stats.rms, stats.count);
    }
    
    Ok(())
}
```

## CLI Usage

```bash
# Analyze RINEX file with SP3 orbits
geoveil-mp analyze --obs observation.24o --sp3 ephemeris.sp3 --csv results.csv

# Get file information
geoveil-mp info observation.24o

# Estimate position from pseudoranges
geoveil-mp position --obs observation.24o --nav navigation.24n
```

## Multipath Estimation

The code multipath is estimated using the linear combination:

$$MP_1 = R_1 - \left(1+\frac{2}{\alpha - 1}\right)\Phi_1 + \left(\frac{2}{\alpha - 1}\right)\Phi_2$$

where:
- $R_1$ is the code observation on frequency 1
- $\Phi_1, \Phi_2$ are phase observations on frequencies 1 and 2  
- $\alpha = f_1^2/f_2^2$ is the frequency ratio squared

This combination eliminates ionospheric delay and geometry, leaving only code multipath, code noise, and ambiguity-related biases.

## Supported GNSS Systems

| System | Code | Frequencies | Navigation |
|--------|------|-------------|------------|
| GPS | G | L1, L2, L5 | Keplerian |
| GLONASS | R | G1, G2, G3 | State Vector (RK4) |
| Galileo | E | E1, E5a, E5b, E6 | Keplerian |
| BeiDou | C | B1I, B1C, B2a, B2b, B3I | Keplerian |
| QZSS | J | L1, L2, L5, L6 | Keplerian |
| NavIC | I | L5, S, L1 | Keplerian |
| SBAS | S | L1, L5 | - |

## Data Sources

### SP3 Precise Orbits (No Authentication Required)

```
ESA:  http://navigation-office.esa.int/products/gnss-products/{week}/
GFZ:  https://igs.bkg.bund.de/root_ftp/IGS/products/mgex/{week}/
CODE: https://igs.bkg.bund.de/root_ftp/IGS/products/mgex/{week}/
```

### Broadcast Navigation

```
BKG:  https://igs.bkg.bund.de/root_ftp/IGS/BRDC/{year}/{doy}/
IGN:  https://igs.ign.fr/pub/igs/data/{year}/{doy}/
```

## Python API Reference

### Classes

| Class | Description |
|-------|-------------|
| `GnssSystem` | GNSS constellation identifier (G, R, E, C, J, I, S) |
| `Satellite` | Satellite PRN identifier (e.g., "G01", "E11") |
| `Epoch` | Time representation with GPS/Julian conversions |
| `Ecef` | Earth-Centered Earth-Fixed coordinates |
| `Geodetic` | Latitude/Longitude/Height coordinates |
| `RinexObsData` | RINEX observation data container |
| `Sp3Data` | SP3 precise orbit data container |
| `MultipathAnalyzer` | Main multipath analysis engine |
| `AnalysisResults` | Analysis results container |

### Functions

| Function | Description |
|----------|-------------|
| `read_rinex_obs(path)` | Read RINEX observation file |
| `read_rinex_obs_bytes(data, filename)` | Read RINEX from bytes |
| `read_sp3(path)` | Read SP3 precise orbit file |
| `get_frequency(system, band, fcn)` | Get signal frequency (Hz) |
| `get_wavelength(system, band, fcn)` | Get signal wavelength (m) |
| `calculate_azel(receiver, satellite)` | Compute azimuth/elevation |
| `compute_elevation(sp3, receiver, sat, epoch)` | Compute elevation from SP3 |
| `version()` | Get library version |

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `SPEED_OF_LIGHT` | 299792458.0 | Speed of light (m/s) |
| `GM_WGS84` | 3.986005e14 | Earth gravitational parameter (m³/s²) |
| `EARTH_RADIUS` | 6378137.0 | WGS84 Earth radius (m) |

## Architecture

```
geoveil_mp/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── python.rs           # Python bindings (PyO3)
│   ├── rinex/              # RINEX parsing
│   │   ├── types.rs        # Data structures
│   │   └── obs_reader.rs   # Observation file reader
│   ├── navigation/         # Ephemeris handling
│   │   ├── types.rs        # Ephemeris types
│   │   ├── kepler2ecef.rs  # Keplerian to ECEF
│   │   ├── glonass.rs      # GLONASS Runge-Kutta
│   │   └── sp3.rs          # SP3 Neville interpolation
│   ├── analysis/           # Analysis algorithms
│   │   ├── multipath.rs    # Multipath estimation
│   │   ├── cycle_slip.rs   # Cycle slip detection
│   │   └── position.rs     # Position estimation
│   ├── plotting/           # R integration
│   └── utils/              # Utilities
│       ├── constants.rs    # Physical constants
│       ├── coordinates.rs  # Coordinate transforms
│       ├── time.rs         # Time handling
│       └── error.rs        # Error types
├── examples/
├── tests/
└── r_scripts/              # R plotting scripts
```

## Performance

Benchmarks on a typical 24-hour multi-GNSS RINEX file (~100MB):

| Operation | Time | Notes |
|-----------|------|-------|
| Read RINEX 3.05 | ~500ms | Memory-mapped I/O |
| Read SP3 | ~50ms | Neville interpolation ready |
| Multipath analysis | ~200ms | Parallel processing |
| Position estimation | ~2s | All epochs |

## Related Projects

- **[geoveil-cn0](https://github.com/miluta7/geoveil-cn0)**: CN0 (carrier-to-noise ratio) analysis and interference detection
- **[GNSS_Multipath_Analysis_Software](https://github.com/paarnes/GNSS_Multipath_Analysis_Software)**: Original Python implementation (inspiration)

## References

- RINEX 4.02 Specification (IGS/RTCM)
- GPS Interface Control Document (IS-GPS-200)
- GLONASS Interface Control Document (ICD-GLONASS)
- Galileo OS-SDD (Open Service Signal-in-Space ICD)
- IGS Multi-GNSS Experiment (MGEX)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.
