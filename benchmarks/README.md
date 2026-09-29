# Benchmarks

Wall-clock time of the geoveil-mp pipeline on public IGS observation files from 31 December 2025 (BKG archive).

| File set | What it tests |
|---|---|
| `zimm3650.25d.gz` | RINEX 2.11, 24 h at 30 s, GPS, 13 observation types (3 lines per satellite) |
| `BOR100POL_R_20253650000_01D_30S_MO.crx.gz` | RINEX 3.02, 24 h at 30 s, GPS + GLONASS + Galileo + BeiDou + QZSS; read as plain RINEX and as gzip |
| `BUCU00ROU_S_20253650000_01D_01S_MO.crx.tar.gz` | RINEX 3.05 at 1 Hz, 96 files of 15 minutes; benchmarked as 15 min, 1 h and 24 h |

Each file runs in its own process, once with all cores and once with `RAYON_NUM_THREADS=1`. Every stage is the median of three runs:

- **Parse**: `read_rinex_obs`
- **Multipath**: `MultipathAnalyzer(obs, 10.0, ["G", "R", "E", "C", "J"]).analyze()`, including cycle-slip detection
- **SNR wavelets**: `analyze_snr_wavelets(obs)`

Hatanaka decompression uses the `hatanaka` Python package. It is timed separately because geoveil-mp reads plain and gzip RINEX, not Hatanaka.

## Run it

```bash
docker run --rm -v "$PWD/benchmarks:/bench" python:3.12-slim sh /bench/run.sh 1.0.1
python benchmarks/report.py benchmarks
```

`run.sh` installs the given geoveil-mp version from PyPI, downloads the data (about 80 MB) into `benchmarks/raw/`, and writes `results.jsonl`. `report.py` turns it into `results.md`. Set `NUMPY_REFERENCE_DIR` to a folder containing the GeoVeil platform's `advanced_multipath.py` to also time the numpy wavelet reference.

## Results

See [results.md](results.md). Hardware: [cpu.txt](cpu.txt).
