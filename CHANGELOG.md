# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial planning for v0.2.0 features

## [0.1.0] - 2026-01-21

### Added
- Initial release of GeoVeil-MP
- RINEX v2.xx, v3.xx, and v4.xx observation file support
- Multi-GNSS support: GPS, GLONASS, Galileo, BeiDou, QZSS, NavIC, SBAS
- SP3 precise orbit file parsing with Neville interpolation
- Broadcast ephemeris support (Keplerian elements)
- GLONASS state vector propagation using 4th-order Runge-Kutta
- Code multipath estimation using linear combinations
- Cycle slip detection (ionospheric residuals, code-phase)
- Position estimation (least squares SPP)
- Python bindings via PyO3
- R plotting integration for visualizations
- CLI tool for command-line analysis
- Memory-mapped I/O for large files
- Parallel processing with Rayon

### Performance
- RINEX parsing: ~500ms for 24-hour file
- SP3 reading: ~50ms
- Multipath analysis: ~200ms
- Position estimation: ~2s for all epochs

### Documentation
- Comprehensive README with Rust and Python examples
- API documentation
- Example scripts

## [0.0.1] - 2026-01-01

### Added
- Project scaffolding
- Basic RINEX parsing structure
- Core data types

[Unreleased]: https://github.com/miluta7/geoveil-mp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/miluta7/geoveil-mp/releases/tag/v0.1.0
[0.0.1]: https://github.com/miluta7/geoveil-mp/releases/tag/v0.0.1
