# GeoVeil-MP Notebooks

Jupyter notebooks for GNSS multipath analysis using the `geoveil-mp` Rust library.

## Available Notebooks

| Notebook | Description |
|----------|-------------|
| `geoveil_mp_analysis.ipynb` | Main multipath analysis with interactive widgets |

## Requirements

- Python 3.8+
- Jupyter Notebook or JupyterLab
- `geoveil-mp` library

## Installation

### Option A: Install from pip (if published)
```bash
pip install geoveil-mp
```

### Option B: Build from source
```bash
# Requires Rust: https://rustup.rs
cd ..  # go to geoveil-mp root
pip install maturin
maturin develop --release --features python
```

## Quick Start

```bash
# Install dependencies
pip install numpy pandas matplotlib plotly ipywidgets

# Launch Jupyter
jupyter notebook geoveil_mp_analysis.ipynb
```

## Notebook Features

- **File Input**: Upload widget or file path for RINEX files
- **Optional Ephemeris**: SP3 auto-download or use your own (can be disabled)
- **Multi-GNSS**: GPS, GLONASS, Galileo, BeiDou support
- **Visualizations**: Time series, elevation plots, skyplots (Plotly)
- **Export**: CSV data + PNG plots

## Ephemeris Options

The notebook supports three modes for satellite elevations:

1. **Auto-download**: Downloads SP3 based on RINEX date (default)
2. **Manual path**: Use your own SP3/ephemeris file
3. **Disabled**: Skip ephemeris entirely (faster, approximate elevations)

## Data Sources

SP3 files are downloaded from (no authentication required):
- ESA Navigation Office
- GFZ Potsdam (via BKG)
- CODE (via BKG)
- WHU (via BKG)
