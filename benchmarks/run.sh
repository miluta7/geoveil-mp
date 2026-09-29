#!/bin/sh
# Usage: docker run --rm -v "$PWD/benchmarks:/bench" python:3.12-slim sh /bench/run.sh [version]
set -e
VERSION=${1:-1.0.1}
pip install -q --no-cache-dir --root-user-action=ignore "geoveil-mp==$VERSION" numpy hatanaka
lscpu | grep -E 'Model name|^CPU\(s\)' | sed 's/  */ /g' > /bench/cpu.txt
python --version >> /bench/cpu.txt
python /bench/prep.py > /dev/null
: > /bench/results.jsonl
for f in zimm3650.25o BOR100POL_R_20253650000_01D_30S_MO.rnx BOR100POL_R_20253650000_01D_30S_MO.rnx.gz \
         BUCU_15M_01S.rnx BUCU_01H_01S.rnx BUCU_01D_01S.rnx; do
  python /bench/bench_one.py "/work/$f" 3 1 | tee -a /bench/results.jsonl
  RAYON_NUM_THREADS=1 python /bench/bench_one.py "/work/$f" 3 0 | tee -a /bench/results.jsonl
done
