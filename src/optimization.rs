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
}
