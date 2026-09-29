"""Turn results.jsonl + prep.json into the README table and the landing-page data block.

Usage: python report.py <bench_dir>   (reads results.jsonl, prep.json, cpu.txt there)
"""
import json
import sys
from pathlib import Path

bench = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
rows = [json.loads(l) for l in (bench / "results.jsonl").read_text().splitlines() if l.strip()]
prep = json.loads((bench / "prep.json").read_text())
cpu = (bench / "cpu.txt").read_text()
cpu_model = next(l.split(":", 1)[1].strip() for l in cpu.splitlines() if l.startswith("Model name"))
threads = next(l.split(":", 1)[1].strip() for l in cpu.splitlines() if l.startswith("CPU(s)"))

LABELS = {
    "zimm3650.25o": ("RINEX 2.11", "24 h", "30 s", "ZIMM", "Hatanaka + gzip"),
    "BOR100POL_R_20253650000_01D_30S_MO.rnx": ("RINEX 3.02", "24 h", "30 s", "BOR1", "Hatanaka + gzip"),
    "BOR100POL_R_20253650000_01D_30S_MO.rnx.gz": ("RINEX 3.02", "24 h", "30 s", "BOR1", "gzip, read directly"),
    "BUCU_15M_01S.rnx": ("RINEX 3.05", "15 min", "1 Hz", "BUCU", "Hatanaka"),
    "BUCU_01H_01S.rnx": ("RINEX 3.05", "1 h", "1 Hz", "BUCU", "Hatanaka"),
    "BUCU_01D_01S.rnx": ("RINEX 3.05", "24 h", "1 Hz", "BUCU", "Hatanaka"),
}
multi = {r["file"]: r for r in rows if r["threads"] != "1"}
single = {r["file"]: r for r in rows if r["threads"] == "1"}
out = []
for f, (ver, span, rate, station, comp) in LABELS.items():
    m, s = multi[f], single[f]
    p = prep.get(f, {})
    total = m["parse_s"] + m["multipath_s"] + m["wavelets_s"]
    out.append({
        "id": f, "version": ver, "span": span, "rate": rate, "station": station, "compression": comp,
        "systems": "".join(sorted(m["systems"])), "epochs": m["epochs"], "satellites": m["satellites"],
        "mb": m["bytes"] / 1e6, "compressed_mb": p.get("compressed_bytes", 0) / 1e6,
        "decompress_s": p.get("decompress_s"),
        "parse_s": m["parse_s"], "multipath_s": m["multipath_s"], "wavelets_s": m["wavelets_s"],
        "total_s": total, "total_1t_s": s["parse_s"] + s["multipath_s"] + s["wavelets_s"],
        "wavelets_1t_s": s["wavelets_s"], "numpy_wavelets_s": m.get("numpy_wavelets_s"),
        "estimates": m["estimates"], "signals": m["signals"], "peak_rss_mb": m["peak_rss_mb"],
        "version_lib": m.get("geoveil_mp", "?"),
    })

meta = {"cpu": cpu_model, "threads": threads, "lib": out[0]["version_lib"]}
(bench / "report.json").write_text(json.dumps({"meta": meta, "rows": out}, indent=1))


def fmt_s(x):
    return "–" if x is None else (f"{x:.2f} s" if x < 10 else f"{x:.1f} s")


def fmt_mem(mb):
    return f"{mb / 1024:.1f} GB" if mb >= 1024 else f"{mb:.0f} MB"


md = [
    f"geoveil-mp {meta['lib']} from PyPI, Python 3.12, {cpu_model}, {threads} threads. "
    "Median of 3 runs per stage; the file is already on local disk.",
    "",
    "| File | Size | Epochs × sats | Parse | Multipath | SNR wavelets | Total | Total, 1 thread | Peak memory | Wavelets vs numpy |",
    "|---|---|---|---|---|---|---|---|---|---|",
]
for r in out:
    speed = f"{r['numpy_wavelets_s'] / r['wavelets_s']:.0f}×" if r["numpy_wavelets_s"] else "–"
    md.append(
        f"| {r['version']} · {r['span']} · {r['rate']} · {r['station']} ({r['compression']}) "
        f"| {r['mb']:.1f} MB | {r['epochs']:,} × {r['satellites']} "
        f"| {fmt_s(r['parse_s'])} | {fmt_s(r['multipath_s'])} | {fmt_s(r['wavelets_s'])} | **{fmt_s(r['total_s'])}** "
        f"| {fmt_s(r['total_1t_s'])} | {fmt_mem(r['peak_rss_mb'])} | {speed} |"
    )
(bench / "results.md").write_text("\n".join(md) + "\n", encoding="utf-8")
print("\n".join(md))
