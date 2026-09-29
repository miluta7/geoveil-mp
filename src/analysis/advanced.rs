//! Advanced multipath diagnostics: SNR-residual Morlet wavelets and first
//! Fresnel zone mapping, following Hunegnaw & Teferle, *Sensors* 2022, 22, 3384.
//!
//! The continuous wavelet transform is the Torrence & Compo (1998) FFT
//! formulation. Everything here is pure Rust with no FFT dependency: series are
//! zero-padded to a power of two, so a radix-2 transform is sufficient.

use std::f64::consts::PI;
use std::ops::Range;

use nalgebra::{Complex, DMatrix, DVector};
use rayon::prelude::*;

use crate::rinex::{ObservationData, Satellite};
use crate::utils::constants::frequencies::gps::L1_WAVELENGTH;

type C64 = Complex<f64>;

/// Morlet wavenumber (Torrence & Compo standard).
pub const OMEGA0: f64 = 6.0;
/// Reconstruction factor for the Morlet wavelet (Torrence & Compo, Table 2).
pub const C_DELTA: f64 = 0.776;
/// Default scale step (article value).
pub const DJ: f64 = 0.125;
/// Default GPS L1 wavelength used for Fresnel zones (m).
pub const DEFAULT_WAVELENGTH: f64 = L1_WAVELENGTH;

// ---------------------------------------------------------------------------
// FFT
// ---------------------------------------------------------------------------

/// In-place iterative radix-2 FFT. `buf.len()` must be a power of two.
/// The inverse transform includes the 1/N normalisation (numpy convention).
fn fft_in_place(buf: &mut [C64], inverse: bool) {
    let n = buf.len();
    debug_assert!(n.is_power_of_two());
    if n <= 1 {
        return;
    }
    // Bit-reversal permutation
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            buf.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let twiddles: Vec<C64> = (0..n / 2)
        .map(|k| C64::from_polar(1.0, sign * 2.0 * PI * k as f64 / n as f64))
        .collect();
    let mut len = 2;
    while len <= n {
        let stride = n / len;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let w = twiddles[k * stride];
                let a = buf[start + k];
                let b = buf[start + k + len / 2] * w;
                buf[start + k] = a + b;
                buf[start + k + len / 2] = a - b;
            }
        }
        len <<= 1;
    }
    if inverse {
        let scale = 1.0 / n as f64;
        buf.iter_mut().for_each(|x| *x *= scale);
    }
}

// ---------------------------------------------------------------------------
// Continuous wavelet transform
// ---------------------------------------------------------------------------

/// Result of a Morlet continuous wavelet transform.
#[derive(Debug, Clone)]
pub struct Cwt {
    /// Complex coefficients, one row per scale, `n` columns.
    pub coefficients: Vec<Vec<C64>>,
    /// Wavelet scales (s).
    pub scales: Vec<f64>,
    /// Fourier-equivalent periods (s).
    pub periods: Vec<f64>,
    /// Cone-of-influence period per time index (s).
    pub coi: Vec<f64>,
    /// Scale step used.
    pub dj: f64,
    /// Sampling interval (s).
    pub dt: f64,
}

impl Cwt {
    /// Wavelet power |W|² per scale and time index.
    pub fn power(&self) -> Vec<Vec<f64>> {
        self.coefficients
            .iter()
            .map(|row| row.iter().map(|c| c.norm_sqr()).collect())
            .collect()
    }

    /// Scale-averaged wavelet power in the period band `[lo, hi)` (T&C eq. 24).
    pub fn band_power(&self, band: (f64, f64)) -> Vec<f64> {
        let n = self.coefficients.first().map_or(0, Vec::len);
        let mut out = vec![0.0; n];
        for ((row, &s), &p) in self.coefficients.iter().zip(&self.scales).zip(&self.periods) {
            if p >= band.0 && p < band.1 {
                for (o, c) in out.iter_mut().zip(row) {
                    *o += c.norm_sqr() / s;
                }
            }
        }
        let k = self.dj * self.dt / C_DELTA;
        out.iter_mut().for_each(|o| *o *= k);
        out
    }
}

fn fourier_factor() -> f64 {
    4.0 * PI / (OMEGA0 + (2.0 + OMEGA0 * OMEGA0).sqrt())
}

/// Continuous wavelet transform of `y` (sampling `dt` seconds) with a Morlet
/// mother wavelet. `s0` defaults to `2·dt`.
pub fn morlet_cwt(y: &[f64], dt: f64, dj: f64, s0: Option<f64>) -> Cwt {
    let n = y.len();
    let s0 = s0.unwrap_or(2.0 * dt);
    let jmax = (((n as f64 * dt / s0).log2() / dj) as i64).max(1) as usize;
    let scales: Vec<f64> = (0..=jmax).map(|j| s0 * 2f64.powf(dj * j as f64)).collect();

    let npad = n.max(1).next_power_of_two();
    let mean = y.iter().sum::<f64>() / n.max(1) as f64;
    let mut spectrum: Vec<C64> = (0..npad)
        .map(|i| C64::new(if i < n { y[i] - mean } else { 0.0 }, 0.0))
        .collect();
    fft_in_place(&mut spectrum, false);
    // Angular frequencies in numpy fftfreq order
    let omega: Vec<f64> = (0..npad)
        .map(|k| {
            let kk = if k < npad.div_ceil(2) { k as f64 } else { k as f64 - npad as f64 };
            2.0 * PI * kk / (npad as f64 * dt)
        })
        .collect();

    let norm = PI.powf(-0.25);
    let coefficients: Vec<Vec<C64>> = scales
        .par_iter()
        .map(|&s| {
            let amp = norm * (2.0 * PI * s / dt).sqrt();
            let mut buf: Vec<C64> = spectrum
                .iter()
                .zip(&omega)
                .map(|(f, &w)| {
                    if w > 0.0 {
                        f * (amp * (-0.5 * (s * w - OMEGA0).powi(2)).exp())
                    } else {
                        C64::new(0.0, 0.0)
                    }
                })
                .collect();
            fft_in_place(&mut buf, true);
            buf.truncate(n);
            buf
        })
        .collect();

    let ff = fourier_factor();
    let periods = scales.iter().map(|s| s * ff).collect();
    let coi = (0..n)
        .map(|i| ff * (i.min(n - 1 - i) as f64 * dt) / 2f64.sqrt())
        .collect();
    Cwt { coefficients, scales, periods, coi, dj, dt }
}

/// Significance level per period against a lag-1 red-noise background
/// (T&C eq. 16-18), in the same units as |W|². `confidence` ≥ 0.95 uses the
/// 95 % χ² level, otherwise 90 %.
pub fn red_noise_significance(y: &[f64], dt: f64, periods: &[f64], confidence: f64) -> Vec<f64> {
    let n = y.len();
    let mean = y.iter().sum::<f64>() / n.max(1) as f64;
    let var = y.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n.max(1) as f64;
    if n < 3 || var <= 0.0 {
        return vec![f64::INFINITY; periods.len()];
    }
    let alpha = pearson(&y[..n - 1], &y[1..]);
    let alpha = if alpha.is_nan() { 0.0 } else { alpha.clamp(0.0, 0.99) };
    let chi2 = if confidence >= 0.95 { 5.991 } else { 4.605 };
    periods
        .iter()
        .map(|p| {
            let f = dt / p;
            let pk = (1.0 - alpha * alpha) / (1.0 + alpha * alpha - 2.0 * alpha * (2.0 * PI * f).cos());
            var * pk * chi2 / 2.0
        })
        .collect()
}

fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma).powi(2);
        sbb += (y - mb).powi(2);
    }
    sab / (saa * sbb).sqrt()
}

/// Interval-adaptive period bands (s). The article's 1 s bands are
/// unreachable at 30 s sampling (Nyquist period = 2·dt), so bands scale with dt.
pub fn period_bands(dt: f64) -> Vec<(f64, f64)> {
    if dt <= 2.0 {
        vec![(2.0, 20.0), (20.0, 40.0), (40.0, 80.0), (80.0, 120.0)]
    } else if dt <= 15.0 {
        vec![(30.0, 60.0), (60.0, 120.0), (120.0, 300.0), (300.0, 600.0)]
    } else {
        vec![(60.0, 120.0), (120.0, 300.0), (300.0, 900.0), (900.0, 1800.0)]
    }
}

// ---------------------------------------------------------------------------
// Detrending and arcs
// ---------------------------------------------------------------------------

/// Remove the polynomial trend (order 2..=`max_order`) that minimises the
/// residual RMS. Order is capped so `n > 3·order`. Returns the residual and
/// the chosen order (0 when only the mean was removed).
pub fn detrend_arc(t: &[f64], v: &[f64], max_order: usize) -> (Vec<f64>, usize) {
    let n = v.len();
    let mean = v.iter().sum::<f64>() / n.max(1) as f64;
    let demeaned = || v.iter().map(|x| x - mean).collect::<Vec<f64>>();
    if n < 12 {
        return (demeaned(), 0);
    }
    let span = (t[n - 1] - t[0]).max(1.0);
    let tc: Vec<f64> = t.iter().map(|x| (x - t[0]) / span).collect();
    let b = DVector::from_column_slice(v);

    let mut best: Option<(f64, Vec<f64>, usize)> = None;
    for order in 2..=max_order {
        if n <= 3 * order {
            break;
        }
        let a = DMatrix::from_fn(n, order + 1, |i, j| tc[i].powi(j as i32));
        let svd = a.clone().svd(true, true);
        let eps = svd.singular_values.max() * n as f64 * f64::EPSILON;
        let Ok(coef) = svd.solve(&b, eps) else { continue };
        let res: Vec<f64> = (&b - &a * coef).iter().copied().collect();
        let rms = (res.iter().map(|r| r * r).sum::<f64>() / n as f64).sqrt();
        if best.as_ref().map_or(true, |(r, _, _)| rms < *r) {
            best = Some((rms, res, order));
        }
    }
    match best {
        Some((_, res, order)) => (res, order),
        None => (demeaned(), 0),
    }
}

/// Split an increasing epoch-time array into contiguous arcs at gaps larger
/// than `gap_factor·dt`.
pub fn split_arcs(times: &[f64], gap_factor: f64, dt: f64) -> Vec<Range<usize>> {
    if times.is_empty() {
        return Vec::new();
    }
    let mut arcs = Vec::new();
    let mut start = 0;
    for i in 1..times.len() {
        if times[i] - times[i - 1] > gap_factor * dt {
            arcs.push(start..i);
            start = i;
        }
    }
    arcs.push(start..times.len());
    arcs
}

fn median_diff(t: &[f64]) -> Option<f64> {
    if t.len() < 2 {
        return None;
    }
    let mut d: Vec<f64> = t.windows(2).map(|w| w[1] - w[0]).collect();
    d.sort_by(|a, b| a.total_cmp(b));
    let m = d.len() / 2;
    Some(if d.len() % 2 == 1 { d[m] } else { 0.5 * (d[m - 1] + d[m]) })
}

// ---------------------------------------------------------------------------
// Fresnel zones (article eq. 6)
// ---------------------------------------------------------------------------

/// First Fresnel zone of a horizontal reflector below the antenna.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FresnelZone {
    /// Semi-major axis, along the azimuth (m).
    pub a: f64,
    /// Semi-minor axis (m).
    pub b: f64,
    /// Horizontal distance from the antenna to the specular point (m).
    pub distance: f64,
}

/// First Fresnel zone for a satellite at `elevation_deg` and an antenna
/// `antenna_height` metres above the reflector. Elevation is floored at 0.5°.
pub fn fresnel_zone(elevation_deg: f64, antenna_height: f64, wavelength: f64) -> FresnelZone {
    let theta = elevation_deg.max(0.5).to_radians();
    let sin_t = theta.sin();
    let b = (wavelength * antenna_height / sin_t + (wavelength / (2.0 * sin_t)).powi(2)).sqrt();
    FresnelZone { a: b / sin_t, b, distance: antenna_height / theta.tan() }
}

/// One Fresnel footprint sample along a satellite track.
#[derive(Debug, Clone, PartialEq)]
pub struct FresnelSample {
    /// Satellite id, e.g. "G05".
    pub satellite: String,
    /// Azimuth (deg).
    pub azimuth: f64,
    /// Elevation (deg).
    pub elevation: f64,
    /// Zone geometry.
    pub zone: FresnelZone,
}

/// Sample each satellite track once per 10°×10° azimuth/elevation cell above
/// `min_elevation` and attach its first-Fresnel-zone footprint.
/// `tracks` holds `(satellite, azimuths, elevations)` in degrees.
pub fn fresnel_map(
    tracks: &[(String, Vec<f64>, Vec<f64>)],
    antenna_height: f64,
    min_elevation: f64,
    wavelength: f64,
) -> Vec<FresnelSample> {
    let mut out = Vec::new();
    for (sat, azs, els) in tracks {
        let mut seen = std::collections::HashSet::new();
        for (&az, &el) in azs.iter().zip(els) {
            if el < min_elevation || !seen.insert(((az / 10.0).floor() as i64, (el / 10.0).floor() as i64)) {
                continue;
            }
            out.push(FresnelSample {
                satellite: sat.clone(),
                azimuth: az,
                elevation: el,
                zone: fresnel_zone(el, antenna_height, wavelength),
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Scalogram
// ---------------------------------------------------------------------------

/// Wavelet scalogram of the longest contiguous arc of a series.
#[derive(Debug, Clone)]
pub struct Scalogram {
    /// Time offsets of the (decimated) columns (s).
    pub t: Vec<f64>,
    /// Fourier periods of the rows (s).
    pub periods: Vec<f64>,
    /// Wavelet power, one row per period.
    pub power: Vec<Vec<f64>>,
    /// 95 % red-noise significance level per period.
    pub significance: Vec<f64>,
    /// Cone of influence per column (s).
    pub coi: Vec<f64>,
    /// Effective sampling interval (s).
    pub interval: f64,
}

/// Scalogram with significance and cone of influence, time axis decimated to
/// at most `max_columns` columns. Needs at least 40 samples in the longest arc.
pub fn scalogram(t: &[f64], v: &[f64], interval: f64, max_columns: usize) -> crate::Result<Scalogram> {
    let arc = split_arcs(t, 5.0, interval)
        .into_iter()
        .reduce(|best, a| if a.len() > best.len() { a } else { best })
        .unwrap_or(0..0);
    let (t, v) = (&t[arc.clone()], &v[arc]);
    if v.len() < 40 {
        return Err(crate::Error::InsufficientData("series too short for a scalogram (< 40 samples)".into()));
    }
    let dt = median_diff(t).unwrap_or(interval);
    let cwt = morlet_cwt(v, dt, DJ, None);
    let significance = red_noise_significance(v, dt, &cwt.periods, 0.95);
    let step = (v.len() / max_columns.max(1)).max(1);
    Ok(Scalogram {
        t: t.iter().step_by(step).copied().collect(),
        power: cwt.power().into_iter().map(|row| row.into_iter().step_by(step).collect()).collect(),
        coi: cwt.coi.iter().step_by(step).copied().collect(),
        periods: cwt.periods,
        significance,
        interval: dt,
    })
}

// ---------------------------------------------------------------------------
// Per-satellite SNR wavelet analysis
// ---------------------------------------------------------------------------

/// Tuning for [`snr_wavelets`].
#[derive(Debug, Clone)]
pub struct SnrWaveletConfig {
    /// Period bands for band power (s). Empty = [`period_bands`] of the interval.
    pub bands: Vec<(f64, f64)>,
    /// Decimate longer series to about this many points before the CWT.
    pub max_points: usize,
    /// Number of time bins for the stored band-power series.
    pub band_power_bins: usize,
    /// Split arcs at gaps larger than this many sampling intervals.
    pub gap_factor: f64,
    /// Highest detrending polynomial order.
    pub max_order: usize,
}

impl Default for SnrWaveletConfig {
    fn default() -> Self {
        Self { bands: Vec::new(), max_points: 20_000, band_power_bins: 96, gap_factor: 5.0, max_order: 9 }
    }
}

/// δSNR residuals and scale-averaged band power for one satellite.
#[derive(Debug, Clone)]
pub struct SnrWavelet {
    /// Satellite id.
    pub satellite: String,
    /// SNR observable used, e.g. "S1C".
    pub code: String,
    /// First epoch (unix seconds, file time scale).
    pub t0: f64,
    /// Residual time offsets from `t0` (s).
    pub t: Vec<f64>,
    /// Detrended linear-amplitude SNR residuals.
    pub residual: Vec<f64>,
    /// Polynomial order chosen per arc.
    pub orders: Vec<usize>,
    /// Band-power bin centres as offsets from `t0` (s). Empty if no arc ≥ 40 samples.
    pub band_t: Vec<f64>,
    /// Binned band power, one series per band.
    pub band_power: Vec<Vec<f64>>,
}

/// Prefer a band-1 SNR code (the article uses L1), then the lowest band.
fn pick_snr_code(codes: &[crate::rinex::SignalCode]) -> Option<crate::rinex::SignalCode> {
    codes.iter().min_by_key(|c| (c.band != 1, c.to_string())).cloned()
}

/// SNR wavelet analysis of one series (`times` in seconds, `snr_db` in dB-Hz).
/// Returns `None` for fewer than 40 samples.
pub fn snr_wavelet_series(
    satellite: &str,
    code: &str,
    times: &[f64],
    snr_db: &[f64],
    dt: f64,
    cfg: &SnrWaveletConfig,
) -> Option<SnrWavelet> {
    if times.len() < 40 {
        return None;
    }
    let bands = if cfg.bands.is_empty() { period_bands(dt) } else { cfg.bands.clone() };
    // dB-Hz → linear amplitude (article §6.2), decimated for very long series
    let step = times.len().div_ceil(cfg.max_points.max(1)).max(1);
    let t: Vec<f64> = times.iter().step_by(step).copied().collect();
    let v: Vec<f64> = snr_db.iter().step_by(step).map(|d| 10f64.powf(d / 20.0)).collect();
    let eff_dt = median_diff(&t).unwrap_or(dt);
    let arcs = split_arcs(&t, cfg.gap_factor, eff_dt);

    let mut residual = vec![0.0; v.len()];
    let mut orders = Vec::with_capacity(arcs.len());
    for arc in &arcs {
        let (r, order) = detrend_arc(&t[arc.clone()], &v[arc.clone()], cfg.max_order);
        residual[arc.clone()].copy_from_slice(&r);
        orders.push(order);
    }

    // Band power per arc, concatenated (a CWT across gaps is invalid)
    let mut bp_t = Vec::new();
    let mut bp: Vec<Vec<f64>> = vec![Vec::new(); bands.len()];
    for arc in arcs.iter().filter(|a| a.len() >= 40) {
        let cwt = morlet_cwt(&residual[arc.clone()], eff_dt, DJ, None);
        for (acc, band) in bp.iter_mut().zip(&bands) {
            acc.extend(cwt.band_power(*band));
        }
        bp_t.extend_from_slice(&t[arc.clone()]);
    }

    let t0 = t[0];
    let (band_t, band_power) = bin_band_power(&bp_t, &bp, cfg.band_power_bins, t0);
    Some(SnrWavelet {
        satellite: satellite.to_string(),
        code: code.to_string(),
        t0,
        t: t.iter().map(|x| x - t0).collect(),
        residual,
        orders,
        band_t,
        band_power,
    })
}

/// Average band power into `nbins` equal time bins (numpy linspace/digitize
/// semantics), dropping empty bins. Times are returned as offsets from `t0`.
fn bin_band_power(bp_t: &[f64], bp: &[Vec<f64>], nbins: usize, t0: f64) -> (Vec<f64>, Vec<Vec<f64>>) {
    if bp_t.is_empty() {
        return (Vec::new(), vec![Vec::new(); bp.len()]);
    }
    let nbins = nbins.min(bp_t.len()).max(1);
    let (lo, hi) = (bp_t[0], bp_t[bp_t.len() - 1] + 1.0);
    let width = (hi - lo) / nbins as f64;
    let mut edges: Vec<f64> = (0..=nbins).map(|k| lo + k as f64 * width).collect();
    edges[nbins] = hi;

    let mut sum_t = vec![0.0; nbins];
    let mut sum_p = vec![vec![0.0; nbins]; bp.len()];
    let mut count = vec![0usize; nbins];
    for (i, &x) in bp_t.iter().enumerate() {
        let idx = edges.partition_point(|e| *e <= x);
        if idx == 0 || idx > nbins {
            continue;
        }
        let b = idx - 1;
        sum_t[b] += x;
        count[b] += 1;
        for (s, series) in sum_p.iter_mut().zip(bp) {
            s[b] += series[i];
        }
    }
    let keep: Vec<usize> = (0..nbins).filter(|&b| count[b] > 0).collect();
    let band_t = keep.iter().map(|&b| sum_t[b] / count[b] as f64 - t0).collect();
    let power = sum_p
        .iter()
        .map(|s| keep.iter().map(|&b| s[b] / count[b] as f64).collect())
        .collect();
    (band_t, power)
}

/// SNR wavelet analysis for `satellites` of an observation file (all
/// satellites when empty), in parallel. `dt` defaults to the file interval or 30 s.
pub fn snr_wavelets(
    obs: &ObservationData,
    satellites: &[Satellite],
    dt: Option<f64>,
    cfg: &SnrWaveletConfig,
) -> Vec<SnrWavelet> {
    let dt = dt.or_else(|| obs.interval()).unwrap_or(30.0);
    let sats = if satellites.is_empty() { obs.satellites() } else { satellites.to_vec() };
    let mut out: Vec<SnrWavelet> = sats
        .par_iter()
        .filter_map(|sat| {
            let mut codes: Vec<_> = obs
                .epochs
                .iter()
                .filter_map(|e| e.satellites.get(sat))
                .flat_map(|o| o.keys().filter(|c| c.is_snr()).cloned())
                .collect();
            codes.sort_by_key(|c| c.to_string());
            codes.dedup();
            let code = pick_snr_code(&codes)?;
            let (times, values): (Vec<f64>, Vec<f64>) = obs
                .epochs
                .iter()
                .filter_map(|e| {
                    let v = e.satellites.get(sat)?.get(&code)?.value;
                    (v > 0.0).then(|| (e.epoch.to_unix_seconds(), v))
                })
                .unzip();
            snr_wavelet_series(&sat.to_string(), &code.to_string(), &times, &values, dt, cfg)
        })
        .collect();
    out.sort_by(|a, b| a.satellite.cmp(&b.satellite));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_matches_naive_dft() {
        let x: Vec<C64> = (0..16).map(|i| C64::new((i as f64 * 0.7).sin(), (i as f64 * 0.3).cos())).collect();
        let mut fast = x.clone();
        fft_in_place(&mut fast, false);
        for (k, f) in fast.iter().enumerate() {
            let naive: C64 = x
                .iter()
                .enumerate()
                .map(|(n, v)| v * C64::from_polar(1.0, -2.0 * PI * (k * n) as f64 / 16.0))
                .sum();
            assert!((f - naive).norm() < 1e-9);
        }
        fft_in_place(&mut fast, true);
        for (a, b) in fast.iter().zip(&x) {
            assert!((a - b).norm() < 1e-12);
        }
    }

    #[test]
    fn cwt_peaks_at_signal_period() {
        let dt = 30.0;
        let y: Vec<f64> = (0..2880).map(|i| (2.0 * PI * i as f64 * dt / 300.0).sin()).collect();
        let cwt = morlet_cwt(&y, dt, DJ, None);
        let mean_power: Vec<f64> = cwt.power().iter().map(|r| r.iter().sum::<f64>() / r.len() as f64).collect();
        let peak = mean_power.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        let period = cwt.periods[peak];
        assert!((period - 300.0).abs() / 300.0 < 0.15, "peak period {period}");

        let inside: f64 = cwt.band_power((120.0, 300.0)).iter().sum();
        let outside: f64 = cwt.band_power((900.0, 1800.0)).iter().sum();
        assert!(inside > 10.0 * outside);
    }

    #[test]
    fn fresnel_matches_article_geometry() {
        let z = fresnel_zone(15.0, 3.5, DEFAULT_WAVELENGTH);
        assert!(z.b > 1.0 && z.b < 3.0 && z.a > z.b && z.distance > 10.0, "{z:?}");
        // Zenith: circle of radius sqrt(λh + λ²/4), specular point under the antenna
        let z = fresnel_zone(90.0, 2.0, DEFAULT_WAVELENGTH);
        let r = (DEFAULT_WAVELENGTH * 2.0 + DEFAULT_WAVELENGTH.powi(2) / 4.0).sqrt();
        assert!((z.a - r).abs() < 1e-9 && (z.b - r).abs() < 1e-9 && z.distance.abs() < 1e-9);
    }

    #[test]
    fn detrend_removes_polynomial_and_arcs_split() {
        let t: Vec<f64> = (0..500).map(|i| i as f64 * 30.0).collect();
        let v: Vec<f64> = t.iter().map(|x| 3.0 + 1e-3 * x - 2e-8 * x * x).collect();
        let (res, order) = detrend_arc(&t, &v, 9);
        assert!(order >= 2);
        assert!(res.iter().all(|r| r.abs() < 1e-8));

        let times = [0.0, 30.0, 60.0, 600.0, 630.0];
        assert_eq!(split_arcs(&times, 5.0, 30.0), vec![0..3, 3..5]);
    }

    #[test]
    fn band_power_bins_are_averaged() {
        let t: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let p = vec![t.clone()];
        let (bt, bp) = bin_band_power(&t, &p, 5, 0.0);
        assert_eq!(bt.len(), 5);
        assert!((bt[0] - 0.5).abs() < 1e-12 && (bp[0][4] - 8.5).abs() < 1e-12, "{bt:?} {bp:?}");
    }
}
