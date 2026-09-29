# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.1] - 2026-09-29

### Fixed
- **RINEX 2 observation files are parsed correctly.** Epoch records were
  detected by checking only columns 2-3 for a number, so almost every
  observation line started a new "epoch" and files came back with no
  satellites. Epoch lines are now matched on their fixed-column layout.
  A daily 30 s RINEX 2.11 file now reads as 2,880 epochs instead of 78,590.
- **GLONASS multipath without a `GLONASS SLOT / FRQ #` header record.**
  RINEX 2 and early 3.0x headers carry no frequency channels, so every
  satellite fell back to channel 0 and GLONASS MP came out near 100 m.
  The nominal slot-to-channel table is now used when the header has none;
  header values still take precedence.
- Event epochs (flags 2 to 5) in RINEX 2 files no longer produce empty epochs.

## [1.0.0] - 2026-09-29

First stable release. The API of 0.2.x is unchanged; everything below is additive
apart from the license.

### Added
- **SNR wavelet analysis in Rust** (`analysis::advanced`, Hunegnaw & Teferle,
  *Sensors* 2022, 22, 3384): Morlet continuous wavelet transform
  (Torrence & Compo 1998 FFT formulation), lag-1 red-noise 95 % significance,
  scale-averaged band power, interval-adaptive period bands, polynomial
  δSNR detrending with automatic order selection, and arc splitting at gaps.
- **First Fresnel zone mapping**: zone semi-axes and specular-point distance
  per satellite track sample (article eq. 6), for any antenna height and
  wavelength.
- **Python API**: `analyze_snr_wavelets(obs)`, `scalogram()`, `morlet_cwt()`,
  `band_power()`, `red_noise_significance()`, `period_bands()`,
  `detrend_arc()`, `split_arcs()`, `fresnel_zone()`, `fresnel_map()` and the
  `GPS_L1_WAVELENGTH` constant. Heavy calls release the GIL and run satellites
  in parallel with rayon.
- Numerically identical to the numpy reference used by the GeoVeil batch
  platform (relative error below 1e-9 on wavelet power), and about 20x faster
  on a 1 Hz, 51-satellite hourly file.
- CI now runs the Python test suite against the built wheel.

### Changed
- **License changed from MIT to PolyForm Noncommercial 1.0.0** with an
  attribution term. Non-commercial use remains free with credit; commercial
  use now requires a separate license. Releases up to and including 0.2.1
  remain available under MIT.
- Development status classifier raised to Production/Stable.

## [0.2.1] - 2026-07-03

### Changed
- **Release the GIL during analysis**: `MultipathAnalyzer.analyze()`,
  `read_rinex_obs()`, `read_rinex_obs_bytes()` and
  `AnalysisResults.compute_elevations()` now run their pure-Rust cores with
  the Python GIL released. Callers using thread pools (e.g. the GeoVeil
  batch worker) get true multi-core parallelism — previously N worker
  threads processed files effectively serially.

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
