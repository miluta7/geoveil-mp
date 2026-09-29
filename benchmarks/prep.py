"""Download public IGS data (BKG archive, 2025-12-31) and build the benchmark file set."""
import glob
import gzip
import json
import os
import shutil
import tarfile
import time
import urllib.request

import hatanaka

RAW, W = "/bench/raw", "/work"
os.makedirs(RAW, exist_ok=True)
os.makedirs(W, exist_ok=True)
prep = {}

ARCHIVE = "https://igs.bkg.bund.de/root_ftp/IGS"
for rel in ("obs/2025/365/zimm3650.25d.gz",
            "obs/2025/365/BOR100POL_R_20253650000_01D_30S_MO.crx.gz",
            "highrate/2025/365/BUCU00ROU_S_20253650000_01D_01S_MO.crx.tar.gz"):
    out = f"{RAW}/{os.path.basename(rel)}"
    if not os.path.exists(out):
        urllib.request.urlretrieve(f"{ARCHIVE}/{rel}", out)


def decompress(src, dst):
    t = time.perf_counter()
    data = hatanaka.decompress(open(src, "rb").read())
    dt = time.perf_counter() - t
    open(dst, "wb").write(data)
    return dt, len(data)


def body(path):
    lines = open(path, encoding="ascii", errors="replace").read().splitlines(keepends=True)
    i = next(k for k, l in enumerate(lines) if "END OF HEADER" in l[60:])
    return lines[: i + 1], lines[i + 1:]


def obs_types(header):
    return [l for l in header if "SYS / # / OBS TYPES" in l[60:] or "# / TYPES OF OBSERV" in l[60:]]


# RINEX 2.11 daily 30 s (Hatanaka + gzip)
dt, n = decompress(f"{RAW}/zimm3650.25d.gz", f"{W}/zimm3650.25o")
prep["zimm3650.25o"] = {"source": "zimm3650.25d.gz", "compressed_bytes": os.path.getsize(f"{RAW}/zimm3650.25d.gz"), "decompress_s": dt}

# RINEX 3 daily 30 s (Hatanaka + gzip), plus a gzip-only copy for the .gz read path
bor = "BOR100POL_R_20253650000_01D_30S_MO"
dt, n = decompress(f"{RAW}/{bor}.crx.gz", f"{W}/{bor}.rnx")
prep[f"{bor}.rnx"] = {"source": f"{bor}.crx.gz", "compressed_bytes": os.path.getsize(f"{RAW}/{bor}.crx.gz"), "decompress_s": dt}
with open(f"{W}/{bor}.rnx", "rb") as f, gzip.open(f"{W}/{bor}.rnx.gz", "wb", compresslevel=6) as g:
    shutil.copyfileobj(f, g)
prep[f"{bor}.rnx.gz"] = {"source": f"{bor}.rnx (gzip -6)", "compressed_bytes": os.path.getsize(f"{W}/{bor}.rnx.gz"), "decompress_s": None}

# RINEX 3 1 Hz: 96 x 15-minute Hatanaka files -> 15 min, 1 hour, 24 hours
hr = f"{W}/hr"
os.makedirs(hr, exist_ok=True)
tarfile.open(f"{RAW}/BUCU00ROU_S_20253650000_01D_01S_MO.crx.tar.gz").extractall(hr, filter="data")
crx = sorted(glob.glob(f"{hr}/**/*.crx", recursive=True))
assert len(crx) == 96, len(crx)
t_total, rnx = 0.0, []
for c in crx:
    dt, _ = decompress(c, c[:-4] + ".rnx")
    t_total += dt
    rnx.append(c[:-4] + ".rnx")
crx_bytes = sum(os.path.getsize(c) for c in crx)
head0, _ = body(rnx[0])
types0 = obs_types(head0)
mismatch = [r for r in rnx if obs_types(body(r)[0]) != types0]


def splice(files, out):
    h, b = body(files[0])
    with open(out, "w", encoding="ascii", newline="") as f:
        f.writelines(h)
        f.writelines(b)
        for r in files[1:]:
            f.writelines(body(r)[1])


usable = [r for r in rnx if r not in mismatch]
shutil.copy(rnx[0], f"{W}/BUCU_15M_01S.rnx")
splice(usable[:4], f"{W}/BUCU_01H_01S.rnx")
splice(usable, f"{W}/BUCU_01D_01S.rnx")
per_file = t_total / 96
prep["BUCU_15M_01S.rnx"] = {"source": os.path.basename(crx[0]), "compressed_bytes": os.path.getsize(crx[0]), "decompress_s": per_file}
prep["BUCU_01H_01S.rnx"] = {"source": "4 x 15M crx", "compressed_bytes": sum(os.path.getsize(c) for c in crx[:4]), "decompress_s": per_file * 4}
prep["BUCU_01D_01S.rnx"] = {"source": f"{len(usable)} x 15M crx", "compressed_bytes": crx_bytes, "decompress_s": t_total}
prep["_notes"] = {"hr_files_with_different_obs_types": [os.path.basename(m) for m in mismatch]}
json.dump(prep, open("/bench/prep.json", "w"), indent=1)
print(json.dumps(prep, indent=1))
