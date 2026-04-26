//! Small DSP helpers shared across the synth.
//!
//! Only the sine wavetable is precomputed (it is hit per-sample on the audio
//! thread); the rest are short formulas inlined into free functions because
//! caching them in a global table costs more lines than it saves cycles.

use std::f32::consts::PI;
use std::sync::LazyLock;

const SINE_TABLE_SIZE: usize = 4096;
const SINE_TABLE_MASK: usize = SINE_TABLE_SIZE - 1; // power-of-two for cheap wrap

/// One full cycle of `sin(2π · i / 4096)`, computed once at first access.
static SINE_TABLE: LazyLock<[f32; SINE_TABLE_SIZE]> = LazyLock::new(|| {
    let mut t = [0.0_f32; SINE_TABLE_SIZE];
    for (i, slot) in t.iter_mut().enumerate() {
        *slot = ((i as f32 / SINE_TABLE_SIZE as f32) * 2.0 * PI).sin();
    }
    t
});

/// Sine lookup with linear interpolation. Accepts any real phase (negative,
/// multi-cycle); wraps automatically. With 4096 entries the worst-case
/// interpolation error is below 1e-6, well under the noise floor of the rest
/// of the audio chain — Catmull-Rom interpolation buys nothing audible at
/// this density and costs five extra multiplies per sample.
pub fn fast_sin(phase: f32) -> f32 {
    const INV_TWO_PI: f32 = 1.0 / (2.0 * PI);
    let index_f = (phase * INV_TWO_PI).rem_euclid(1.0) * SINE_TABLE_SIZE as f32;
    let i0 = index_f as usize & SINE_TABLE_MASK;
    let frac = index_f - i0 as f32;
    let y0 = SINE_TABLE[i0];
    let y1 = SINE_TABLE[(i0 + 1) & SINE_TABLE_MASK];
    y0 + (y1 - y0) * frac
}

/// MIDI note number → Hz (equal temperament, A4 = 440 Hz).
pub fn midi_to_hz(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}

/// Polyphony attenuation: 1/√N (RMS-preserving). Returns 1.0 for n ≤ 1.
pub fn voice_scale(n: usize) -> f32 {
    if n <= 1 {
        1.0
    } else {
        1.0 / (n as f32).sqrt()
    }
}

/// DX7 operator level (0..99) → linear amplitude.
///
/// Each level step is ~0.75 dB and level 99 = 0 dB (unity), per the DX7
/// service manual. Level 0 is hard silence.
pub fn dx7_level_to_amplitude(level: u8) -> f32 {
    let l = level.min(99);
    if l == 0 {
        return 0.0;
    }
    10.0_f32.powf((l as f32 - 99.0) * (0.75 / 20.0))
}

/// DX7 envelope rate (0..99) → seconds for one full envelope segment.
///
/// Calibrated piecewise log-interpolation between published reference points;
/// not a bit-exact ROM dump but well within ±15% of the DX7 service manual
/// curve. The 8 ms floor matches the hardware lower bound.
pub fn dx7_rate_to_time(rate: u8) -> f32 {
    const REFS: &[(u8, f32)] = &[
        (0, 38.0),
        (25, 3.8),
        (50, 0.60),
        (75, 0.105),
        (90, 0.037),
        (99, 0.012),
    ];
    let r = rate.min(99);
    for pair in REFS.windows(2) {
        let (r_lo, t_lo) = pair[0];
        let (r_hi, t_hi) = pair[1];
        if r >= r_lo && r <= r_hi {
            let frac = (r - r_lo) as f32 / (r_hi - r_lo) as f32;
            let log_lo = t_lo.log2();
            let log_hi = t_hi.log2();
            return 2.0_f32.powf(log_lo + (log_hi - log_lo) * frac).max(0.008);
        }
    }
    REFS.last().unwrap().1
}

/// DX7 envelope rate (0..99) → per-second multiplier (`1 / time`).
pub fn dx7_rate_to_multiplier(rate: u8) -> f32 {
    1.0 / dx7_rate_to_time(rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // fast_sin
    // -----------------------------------------------------------------------

    #[test]
    fn fast_sin_matches_built_in_within_tolerance() {
        for i in 0..256 {
            let phase = (i as f32 / 256.0) * 2.0 * PI;
            let approx = fast_sin(phase);
            let exact = phase.sin();
            assert!(
                (approx - exact).abs() < 1e-3,
                "phase={phase}, approx={approx}, exact={exact}"
            );
        }
    }

    #[test]
    fn fast_sin_handles_negative_phase() {
        let neg = fast_sin(-PI / 2.0);
        assert!((neg + 1.0).abs() < 1e-3);
    }

    #[test]
    fn fast_sin_periodic_above_two_pi() {
        let a = fast_sin(PI / 4.0);
        let b = fast_sin(PI / 4.0 + 2.0 * PI);
        assert!((a - b).abs() < 1e-3);
    }

    // -----------------------------------------------------------------------
    // midi_to_hz
    // -----------------------------------------------------------------------

    #[test]
    fn a4_midi_69_is_440_hz() {
        assert!((midi_to_hz(69) - 440.0).abs() < 0.01);
    }

    #[test]
    fn a3_midi_57_is_220_hz() {
        assert!((midi_to_hz(57) - 220.0).abs() < 0.05);
    }

    // -----------------------------------------------------------------------
    // voice_scale
    // -----------------------------------------------------------------------

    #[test]
    fn voice_scale_zero_and_one_are_unity() {
        assert_eq!(voice_scale(0), 1.0);
        assert_eq!(voice_scale(1), 1.0);
    }

    #[test]
    fn voice_scale_decreases_with_more_voices() {
        assert!(voice_scale(4) > voice_scale(16));
        assert!(voice_scale(2) > voice_scale(4));
    }

    // -----------------------------------------------------------------------
    // DX7 level
    // -----------------------------------------------------------------------

    #[test]
    fn dx7_level_99_is_unity_amplitude() {
        assert!((dx7_level_to_amplitude(99) - 1.0).abs() < 0.01);
    }

    #[test]
    fn dx7_level_zero_is_silent() {
        assert_eq!(dx7_level_to_amplitude(0), 0.0);
    }

    #[test]
    fn dx7_level_is_monotonic_and_clamps() {
        let mut prev = 0.0;
        for level in 1..=99u8 {
            let a = dx7_level_to_amplitude(level);
            assert!(a > prev, "level {level} not monotonic: {a} <= {prev}");
            prev = a;
        }
        assert_eq!(dx7_level_to_amplitude(99), dx7_level_to_amplitude(200));
    }

    // -----------------------------------------------------------------------
    // DX7 rate
    // -----------------------------------------------------------------------

    #[test]
    fn dx7_rate_zero_is_about_38_seconds() {
        assert!((35.0..=42.0).contains(&dx7_rate_to_time(0)));
    }

    #[test]
    fn dx7_rate_99_is_about_12_ms() {
        assert!(dx7_rate_to_time(99) <= 0.020);
    }

    #[test]
    fn dx7_rate_table_within_15_percent_of_reference() {
        for (rate, expected) in [(0u8, 38.0), (25, 3.8), (50, 0.60), (75, 0.105), (99, 0.012)] {
            let actual = dx7_rate_to_time(rate);
            let ratio = actual / expected;
            assert!(
                (0.85..=1.15).contains(&ratio),
                "rate {rate}: expected ~{expected}s, got {actual:.4}s (ratio {ratio:.3})"
            );
        }
    }

    #[test]
    fn dx7_rate_to_multiplier_inverts_time() {
        let time = dx7_rate_to_time(50);
        let mult = dx7_rate_to_multiplier(50);
        assert!((mult * time - 1.0).abs() < 1e-3);
    }

    #[test]
    fn dx7_rate_clamps_above_99() {
        assert_eq!(dx7_rate_to_time(99), dx7_rate_to_time(200));
    }
}
