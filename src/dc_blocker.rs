//! First-order high-pass filter to remove DC offset from the audio output.
//!
//! Standard form: `y[n] = x[n] - x[n-1] + R * y[n-1]`
//! with `R = 1 - 2 * PI * fc / fs`.

pub struct DcBlocker {
    prev_input: f32,
    prev_output: f32,
    r: f32,
}

impl DcBlocker {
    pub fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        let r = 1.0 - 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate;
        Self {
            prev_input: 0.0,
            prev_output: 0.0,
            r,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = input - self.prev_input + self.r * self.prev_output;
        self.prev_input = input;
        self.prev_output = output;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SAMPLE_RATE: f32 = 44_100.0;
    const CUTOFF: f32 = 5.0;

    #[test]
    fn silence_stays_silent() {
        let mut hpf = DcBlocker::new(SAMPLE_RATE, CUTOFF);
        for _ in 0..1024 {
            assert_eq!(hpf.process(0.0), 0.0);
        }
    }

    #[test]
    fn dc_input_converges_to_zero() {
        let mut hpf = DcBlocker::new(SAMPLE_RATE, CUTOFF);
        let dc = 0.5_f32;
        // Run for 2 seconds — well past the time constant of a 5 Hz HPF.
        let mut last = 0.0;
        for _ in 0..(SAMPLE_RATE as usize * 2) {
            last = hpf.process(dc);
        }
        assert!(last.abs() < 1e-3, "DC residual: {last}");
    }

    #[test]
    fn high_frequency_passes_through() {
        // 1 kHz sinusoid — way above the 5 Hz cutoff. Amplitude must be preserved.
        let mut hpf = DcBlocker::new(SAMPLE_RATE, CUTOFF);
        let freq = 1_000.0;
        let n_samples = SAMPLE_RATE as usize / 10; // 100 ms
        let mut peak = 0.0_f32;

        // Skip the first 100 samples to let transient settle.
        for i in 0..n_samples {
            let phase = 2.0 * PI * freq * (i as f32) / SAMPLE_RATE;
            let sample = phase.sin();
            let out = hpf.process(sample);
            if i > 100 {
                peak = peak.max(out.abs());
            }
        }
        assert!(
            peak > 0.95 && peak <= 1.01,
            "expected ~1.0 amplitude at 1 kHz, got {peak}"
        );
    }

    #[test]
    fn near_cutoff_is_attenuated() {
        // At fc = 5 Hz the magnitude response of a first-order HPF is -3 dB (~0.707).
        // We sanity-check that signals near the cutoff lose energy.
        let mut hpf = DcBlocker::new(SAMPLE_RATE, CUTOFF);
        let freq = 5.0;
        let n_samples = SAMPLE_RATE as usize * 2;
        let mut peak = 0.0_f32;

        for i in 0..n_samples {
            let phase = 2.0 * PI * freq * (i as f32) / SAMPLE_RATE;
            let sample = phase.sin();
            let out = hpf.process(sample);
            // Sample peak in the second half (steady state).
            if i > n_samples / 2 {
                peak = peak.max(out.abs());
            }
        }
        assert!(peak < 0.95, "expected attenuation at cutoff, got {peak}");
    }
}
