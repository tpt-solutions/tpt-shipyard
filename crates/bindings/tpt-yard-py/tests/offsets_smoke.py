"""Smoke test for the offsets path of the installed `tpt_yard_py` wheel.

Run from the repository root after `pip install` of the wheel:
    python crates/bindings/tpt-yard-py/tests/offsets_smoke.py
Checks the Wigley hull against its closed-form hydrostatics.
"""
import pathlib

import tpt_yard_py as t

root = pathlib.Path(__file__).resolve().parents[4]
hull = t.OffsetsHull.from_csv_file(str(root / "test-data/stability/wigley-offsets.csv"))

# Wigley, L=100 B=10 T=5: V = 4/9 L B T, KB = 5T/8, BM = 3 B^2 / (35 T).
hs = hull.hydrostatics(5.0)
exact_disp = 4 / 9 * 100 * 10 * 5 * 1.025
assert abs(hs["displacement_t"] - exact_disp) < 5e-3 * exact_disp, hs
assert abs(hs["kb_m"] - 5 * 5 / 8) < 5e-3, hs
gm = 5 * 5 / 8 + 3 * 100 / (35 * 5) - 3.5
assert abs(hull.imo_2008_check(5.0, 3.5)["gm_corrected_m"] - gm) < 0.02

curve = hull.gz_curve(5.0, 3.5, 40.0, 10.0)
assert curve[0] == (0.0, 0.0) and curve[-1][0] == 40.0 and curve[1][1] > 0
assert hull.imo_2008_check(5.0, 3.5)["passed"] == 1.0
assert hull.imo_2008_check(5.0, 4.8)["passed"] == 0.0  # KG above the GM limit

for bad in (
    lambda: hull.hydrostatics(99.0),
    lambda: t.OffsetsHull.from_csv("1,2"),
    lambda: hull.gz_curve(5.0, 3.5, 60.0, 0.0),
    lambda: hull.imo_2008_check(5.0, 3.5, -1.0),
):
    try:
        bad()
    except ValueError:
        continue
    raise SystemExit("expected ValueError")
print("offsets smoke test ok")
