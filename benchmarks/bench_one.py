"""Time the geoveil-mp pipeline stages on one RINEX file; prints one JSON line."""
import json
import os
import resource
import statistics
import sys
import time

import geoveil_mp as gm

path, runs, with_ref = sys.argv[1], int(sys.argv[2]), sys.argv[3] == "1"


def timed(fn):
    ts, out = [], None
    for _ in range(runs):
        t = time.perf_counter()
        out = fn()
        ts.append(time.perf_counter() - t)
    return statistics.median(ts), out


version = next(l[:20].strip() for l in open(path, "rb").read(4000).decode("ascii", "replace").splitlines() if "RINEX VERSION" in l) \
    if not path.endswith(".gz") else "gz"
parse_s, obs = timed(lambda: gm.read_rinex_obs(path))
mp_s, res = timed(lambda: gm.MultipathAnalyzer(obs, 10.0, ["G", "R", "E", "C", "J"]).analyze())
wav_s, wav = timed(lambda: gm.analyze_snr_wavelets(obs))

out = {
    "file": os.path.basename(path),
    "geoveil_mp": gm.version(),
    "bytes": os.path.getsize(path),
    "version": version,
    "interval": obs.interval,
    "epochs": obs.num_epochs,
    "satellites": obs.num_satellites,
    "systems": obs.satellites_by_system(),
    "threads": os.environ.get("RAYON_NUM_THREADS", str(os.cpu_count())),
    "parse_s": parse_s,
    "multipath_s": mp_s,
    "wavelets_s": wav_s,
    "estimates": res.total_estimates(),
    "signals": len(res.statistics),
    "slips": res.total_cycle_slips(),
    "wavelet_sats": len(wav["residuals"]),
    "peak_rss_mb": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024,
}
ref_dir = os.environ.get("NUMPY_REFERENCE_DIR")
if with_ref and ref_dir:
    # Optional: the numpy implementation used by the GeoVeil batch platform
    sys.path.insert(0, ref_dir)
    import advanced_multipath as ref
    sats = sorted(s.id for s in obs.satellites())
    t = time.perf_counter()
    ref.analyze_snr_wavelets(obs, sats, float(obs.interval or 30.0))
    out["numpy_wavelets_s"] = time.perf_counter() - t
print(json.dumps(out))
