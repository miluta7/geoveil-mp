geoveil-mp 1.0.1 from PyPI, Python 3.12, Intel(R) Xeon(R) CPU E5-2620 v3 @ 2.40GHz, 12 threads. Median of 3 runs per stage; the file is already on local disk.

| File | Size | Epochs × sats | Parse | Multipath | SNR wavelets | Total | Total, 1 thread | Peak memory | Wavelets vs numpy |
|---|---|---|---|---|---|---|---|---|---|
| RINEX 2.11 · 24 h · 30 s · ZIMM (Hatanaka + gzip) | 5.4 MB | 2,880 × 32 | 0.15 s | 0.42 s | 0.07 s | **0.64 s** | 0.90 s | 129 MB | 18× |
| RINEX 3.02 · 24 h · 30 s · BOR1 (Hatanaka + gzip) | 19.4 MB | 2,880 × 106 | 0.34 s | 1.47 s | 0.20 s | **2.01 s** | 3.43 s | 398 MB | 22× |
| RINEX 3.02 · 24 h · 30 s · BOR1 (gzip, read directly) | 5.8 MB | 2,880 × 106 | 0.45 s | 1.75 s | 0.21 s | **2.42 s** | 3.59 s | 401 MB | 23× |
| RINEX 3.05 · 15 min · 1 Hz · BUCU (Hatanaka) | 9.6 MB | 900 × 44 | 0.14 s | 0.44 s | 0.08 s | **0.66 s** | 0.98 s | 135 MB | 28× |
| RINEX 3.05 · 1 h · 1 Hz · BUCU (Hatanaka) | 39.9 MB | 3,600 × 52 | 0.61 s | 1.94 s | 0.35 s | **2.90 s** | 4.57 s | 562 MB | 19× |
| RINEX 3.05 · 24 h · 1 Hz · BUCU (Hatanaka) | 948.1 MB | 86,400 × 127 | 14.6 s | 46.4 s | 5.13 s | **66.1 s** | 96.3 s | 10.7 GB | 14× |
