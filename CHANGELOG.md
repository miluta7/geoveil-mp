# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-07-02

Anubis/TEQC-style per-code multipath engine, interval-aware cycle-slip
detection, and SNR series export — enables long-timeseries 30 s multipath
monitoring (Hunegnaw & Teferle 2022 methodology).

### Added
- **Per-code MP combinations**: MP is now computed for EVERY pseudorange code
  observable (C1C, C1X, C2W, C2X, C5X, ...) with a deterministic phase-pair
  selection per system (partner-band priority lists). Previously many codes
  (GPS C2W on L1/L2-only files, GLONASS band 3, Galileo C5X on E1/E5a-only)
  produced no MP at all.
- Statistics signal names in article style: `GPSM1C`, `GLOM3X`, `BDSM2I`;
  `MultipathStats` gains `system`, `code` and `cycle_slips` fields.
- **Cycle-slip detection in the production path** — `analyze()` previously
  returned an empty `cycle_slips` list unconditionally. Slips are now detected
  (geometry-free per phase code, code-phase, LLI restricted to phase
  observables), counted per signal code (`cycle_slip_counts`, e.g. `GPSL2W`),
  and reset the MP debias arcs.
- Delta-domain, interval-aware slip thresholds `|Δ| > base + rate·dt`: a
  single-cycle L1 slip is now detectable at 30 s sampling (the old per-second
  rate thresholds only caught slips ≥ ~7 cycles at 30 s).
- Time-based moving-average bias removal (`bias_window_seconds`, default
  1500 s = TEQC's 50 × 30 s) with O(n) prefix sums; whole-arc mean for short
  arcs (Anubis behavior).
- Arc management: break on gap > `arc_gap_factor`·interval (default 5×), on
  cycle slip on either phase of the pair, or on |ΔMP| > 10 m; arcs shorter
  than `min_arc_seconds` (default 300 s) or 10 epochs are dropped.
- New `MultipathAnalyzer` kwargs: `bias_window_seconds`, `min_arc_seconds`,
  `arc_gap_factor`, `include_codes`, `exclude_codes`, `max_epochs`
  (uniform decimation), `detect_cycle_slips`, `ion_delta_base/rate`,
  `cp_delta_base/rate`.
- `RinexObsData.get_snr_series(satellite, code=None)` → list of `SnrSeries`
  (per S-code, unix-second times + dB-Hz values, `epochs_iso()` helper) and
  `RinexObsData.snr_codes(satellite)` — feeds SNR-residual wavelet analysis.
- `CycleSlip` Python objects gain `signal`, `system`, `threshold`;
  `Epoch.to_unix_seconds()`; `ObservationData::sampling_interval()`
  (median-based, robust to a leading gap); GLONASS FCN from the RINEX header
  is now used in slip detection (was hardcoded to channel 0).
- `compute_elevations()` recomputes elevation-weighted RMS after attaching
  real elevations.

### Changed
- `statistics[].signal` value format changed from `"G_C1C"` to `"GPSM1C"`.
- MP RMS values shift relative to 0.1.x (deterministic phase pick, correct
  per-arc debiasing, moving-average bias window, slip-reset arcs).
- `RinexObsData` is shared internally via `Arc` — constructing an analyzer no
  longer deep-copies the whole dataset.

### Fixed
- Rust-path (CLI) bias removal was algebraically a 2-point smoother that
  retained the full phase-ambiguity bias, making CLI RMS output meaningless.
- Nondeterministic phase selection (HashMap iteration order) when a band had
  multiple phase attributes (L2W vs L2X).
- LLI flags on non-phase observables (code/SNR) no longer trigger slips.

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
