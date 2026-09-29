"""Tests for the SNR wavelet and Fresnel zone API (geoveil-mp >= 1.0)."""
import math

import geoveil_mp as gm


def _sine(n, dt, period):
    t = [i * dt for i in range(n)]
    return t, [math.sin(2 * math.pi * x / period) for x in t]


def test_cwt_peaks_at_signal_period():
    t, y = _sine(2880, 30.0, 300.0)
    r = gm.morlet_cwt(y, 30.0)
    assert len(r["power"]) == len(r["scales"]) == len(r["periods"])
    assert len(r["power"][0]) == len(y) == len(r["coi"])
    mean = [sum(row) / len(row) for row in r["power"]]
    peak = r["periods"][mean.index(max(mean))]
    assert abs(peak - 300.0) / 300.0 < 0.15


def test_band_power_isolates_band():
    _, y = _sine(2880, 30.0, 300.0)
    inside, outside = gm.band_power(y, 30.0, [(120, 300), (900, 1800)])
    assert sum(inside) > 10 * sum(outside)


def test_period_bands_follow_interval():
    assert gm.period_bands(1.0)[0] == (2.0, 20.0)
    assert gm.period_bands(30.0)[-1] == (900.0, 1800.0)


def test_detrend_and_arcs():
    t = [i * 30.0 for i in range(500)]
    v = [3 + 1e-3 * x - 2e-8 * x * x for x in t]
    res, order = gm.detrend_arc(t, v)
    assert order >= 2 and max(abs(r) for r in res) < 1e-8
    assert gm.split_arcs([0, 30, 60, 600, 630], 30.0) == [(0, 3), (3, 5)]


def test_fresnel_zone_geometry():
    a, b, dist = gm.fresnel_zone(15.0, 3.5)
    assert 1.0 < b < 3.0 and a > b and dist > 10.0
    lam = gm.GPS_L1_WAVELENGTH
    a, b, dist = gm.fresnel_zone(90.0, 2.0)
    r = math.sqrt(lam * 2.0 + lam * lam / 4)
    assert math.isclose(a, r) and math.isclose(b, r) and abs(dist) < 1e-9


def test_fresnel_map_samples_cells():
    azs = [i * 1.75 for i in range(200)]
    els = [2 + i * 0.39 for i in range(200)]
    zones = gm.fresnel_map([("G05", azs, els)], 2.0)
    assert zones and all(z["el"] >= 5.0 and z["sat"] == "G05" for z in zones)
    assert {"sat", "az", "el", "a", "b", "dist"} <= set(zones[0])


def test_scalogram_shape():
    t, y = _sine(900, 30.0, 600.0)
    s = gm.scalogram(t, y, 30.0, max_columns=300)
    assert len(s["power"]) == len(s["periods"]) == len(s["significance"])
    assert len(s["t"]) == len(s["coi"]) == len(s["power"][0]) <= 300
