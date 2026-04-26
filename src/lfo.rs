use crate::optimization::fast_sin;
use std::f32::consts::PI;

/// DX7 ROM LFO rate-to-Hz table indexed 0..99 (rate parameter).
///
/// Source: `lfoSource[100]` in MSFA / Dexed `lfo.cc`. Replaces an analytical
/// `0.062 * (1 + fast_exp(rate/99) * 320)` curve that topped out near 20 Hz —
/// the real DX7 reaches ~49 Hz at rate 99, so fast LFO patches (saw/triangle
/// at high rate, used for buzz/zip effects) lost most of their character.
///
/// The literals carry more decimals than `f32` can represent so a future
/// reader can match them byte-for-byte against the upstream ROM dump; the
/// trailing digits are silently truncated by the compiler.
#[allow(clippy::excessive_precision)]
const LFO_FREQ_TABLE: [f32; 100] = [
    0.062541, 0.125031, 0.312393, 0.437120, 0.624610, 0.750694, 0.936330, 1.125302, 1.249609,
    1.436782, 1.560915, 1.752081, 1.875117, 2.062494, 2.247191, 2.374451, 2.560492, 2.686728,
    2.873976, 2.998950, 3.188013, 3.369840, 3.500175, 3.682224, 3.812065, 4.000800, 4.186202,
    4.310716, 4.501260, 4.623209, 4.814636, 4.930480, 5.121901, 5.315191, 5.434783, 5.617346,
    5.750431, 5.946717, 6.062811, 6.248438, 6.431695, 6.564264, 6.749460, 6.868132, 7.052186,
    7.250580, 7.375719, 7.556294, 7.687577, 7.877738, 7.993605, 8.181967, 8.372405, 8.504848,
    8.685079, 8.810573, 8.986341, 9.122423, 9.300595, 9.500285, 9.607994, 9.798158, 9.950249,
    10.117361, 11.251125, 11.384335, 12.562814, 13.676149, 13.904338, 15.092062, 16.366612,
    16.638935, 17.869907, 19.193858, 19.425019, 20.833333, 21.034918, 22.502250, 24.003841,
    24.260068, 25.746653, 27.173913, 27.578599, 29.052876, 30.693677, 31.191516, 32.658393,
    34.317090, 34.674064, 36.416606, 38.197097, 38.550501, 40.387722, 40.749796, 42.625746,
    44.326241, 44.883303, 46.772685, 48.590865, 49.261084,
];

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LFOWaveform {
    #[default]
    Triangle,
    SawDown,
    SawUp,
    Square,
    Sine,
    SampleHold,
}

impl LFOWaveform {
    pub fn all() -> &'static [LFOWaveform] {
        &[
            LFOWaveform::Triangle,
            LFOWaveform::SawDown,
            LFOWaveform::SawUp,
            LFOWaveform::Square,
            LFOWaveform::Sine,
            LFOWaveform::SampleHold,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LFOWaveform::Triangle => "Triangle",
            LFOWaveform::SawDown => "Saw Down",
            LFOWaveform::SawUp => "Saw Up",
            LFOWaveform::Square => "Square",
            LFOWaveform::Sine => "Sine",
            LFOWaveform::SampleHold => "S&H",
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub struct LFO {
    // DX7-style parameters (0-99 range)
    pub rate: f32,        // LFO speed
    pub delay: f32,       // Delay before LFO starts
    pub pitch_depth: f32, // Pitch modulation depth
    pub amp_depth: f32,   // Amplitude modulation depth
    pub waveform: LFOWaveform,
    pub key_sync: bool, // Restart LFO on key press

    // Internal state
    phase: f32,         // Current phase (0.0 to 1.0)
    delay_counter: f32, // Delay countdown in seconds
    sample_rate: f32,
    last_sample_hold: f32, // For sample & hold waveform
    sh_phase_trigger: f32, // Trigger point for S&H
    is_delayed: bool,      // Whether LFO is still in delay phase

    // Cached values for performance
    cached_rate_hz: f32,
    last_rate: f32,
}

impl LFO {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            rate: 50.0,        // Medium rate
            delay: 0.0,        // No delay by default
            pitch_depth: 25.0, // Moderate pitch modulation for testing
            amp_depth: 15.0,   // Moderate amplitude modulation for testing
            waveform: LFOWaveform::Triangle,
            key_sync: false,

            phase: 0.0,
            delay_counter: 0.0,
            sample_rate,
            last_sample_hold: 0.0,
            sh_phase_trigger: 0.0,
            is_delayed: false,
            cached_rate_hz: 0.0,
            last_rate: -1.0, // Initialize to -1 to force first calculation
        }
    }

    /// Convert DX7 rate (0-99) to Hz via the ROM `LFO_FREQ_TABLE`. Fractional
    /// rates are linearly interpolated between adjacent table entries so the
    /// GUI slider is smooth even though the underlying parameter is integer.
    fn dx7_rate_to_hz(rate: f32) -> f32 {
        if rate <= 0.0 {
            return 0.0;
        }
        let r = rate.min(99.0);
        let lo_idx = r as usize;
        let hi_idx = (lo_idx + 1).min(99);
        let frac = r - lo_idx as f32;
        let lo = LFO_FREQ_TABLE[lo_idx];
        let hi = LFO_FREQ_TABLE[hi_idx];
        lo + (hi - lo) * frac
    }

    /// Convert DX7 delay (0-99) to seconds
    fn dx7_delay_to_seconds(delay: f32) -> f32 {
        if delay <= 0.0 {
            0.0
        } else {
            // Linear mapping: 0 to approximately 5 seconds
            delay / 99.0 * 5.0
        }
    }

    /// Trigger LFO (used for key sync)
    pub fn trigger(&mut self) {
        if self.key_sync {
            self.phase = 0.0;
            self.sh_phase_trigger = 0.0;
        }

        if self.delay > 0.0 {
            self.delay_counter = Self::dx7_delay_to_seconds(self.delay);
            self.is_delayed = true;
        } else {
            self.is_delayed = false;
        }
    }

    /// Generate waveform value for current phase (-1.0 to 1.0)
    fn generate_waveform(&mut self, phase: f32) -> f32 {
        match self.waveform {
            LFOWaveform::Sine => fast_sin(phase * 2.0 * PI),

            LFOWaveform::Triangle => {
                if phase < 0.5 {
                    4.0 * phase - 1.0 // Rising: -1 to +1
                } else {
                    3.0 - 4.0 * phase // Falling: +1 to -1
                }
            }

            LFOWaveform::Square => {
                if phase < 0.5 {
                    -1.0
                } else {
                    1.0
                }
            }

            LFOWaveform::SawUp => {
                2.0 * phase - 1.0 // Linear rise from -1 to +1
            }

            LFOWaveform::SawDown => {
                1.0 - 2.0 * phase // Linear fall from +1 to -1
            }

            LFOWaveform::SampleHold => {
                // Sample & hold: change value at specific phase points
                if phase >= self.sh_phase_trigger && phase < self.sh_phase_trigger + 0.01 {
                    // Generate new random value when crossing trigger point
                    self.last_sample_hold = (rand::random::<f32>() * 2.0) - 1.0;
                    self.sh_phase_trigger = if self.sh_phase_trigger < 0.5 {
                        0.5
                    } else {
                        0.0
                    };
                }
                self.last_sample_hold
            }
        }
    }

    /// Process one sample and return modulation values
    pub fn process(&mut self, mod_wheel: f32) -> (f32, f32) {
        // Handle delay phase
        if self.is_delayed {
            self.delay_counter -= 1.0 / self.sample_rate;
            if self.delay_counter <= 0.0 {
                self.is_delayed = false;
            } else {
                return (0.0, 0.0); // No modulation during delay
            }
        }

        // Calculate frequency and phase increment with caching
        let frequency_hz = if (self.rate - self.last_rate).abs() > 0.01 {
            self.last_rate = self.rate;
            self.cached_rate_hz = Self::dx7_rate_to_hz(self.rate);
            self.cached_rate_hz
        } else {
            self.cached_rate_hz
        };
        if frequency_hz <= 0.0 {
            return (0.0, 0.0); // No modulation if rate is 0
        }

        let phase_increment = frequency_hz / self.sample_rate;

        // Generate waveform
        let lfo_value = self.generate_waveform(self.phase);

        // Update phase for next sample
        self.phase += phase_increment;
        while self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Calculate modulation amounts
        // Mod wheel scales the depth (0.0 to 1.0)
        let depth_scale = mod_wheel;

        // Convert DX7 depth (0-99) to modulation percentage
        let pitch_mod = (self.pitch_depth / 99.0) * lfo_value * depth_scale;
        let amp_mod = (self.amp_depth / 99.0) * lfo_value * depth_scale;

        (pitch_mod, amp_mod)
    }

    /// Set LFO parameters with DX7 range validation
    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.0, 99.0);
    }

    pub fn set_delay(&mut self, delay: f32) {
        self.delay = delay.clamp(0.0, 99.0);
    }

    pub fn set_pitch_depth(&mut self, depth: f32) {
        self.pitch_depth = depth.clamp(0.0, 99.0);
    }

    pub fn set_amp_depth(&mut self, depth: f32) {
        self.amp_depth = depth.clamp(0.0, 99.0);
    }

    pub fn set_waveform(&mut self, waveform: LFOWaveform) {
        self.waveform = waveform;
        // Reset sample & hold state when changing waveform
        if waveform == LFOWaveform::SampleHold {
            self.sh_phase_trigger = 0.0;
            self.last_sample_hold = 0.0;
        }
    }

    pub fn set_key_sync(&mut self, key_sync: bool) {
        self.key_sync = key_sync;
    }

    /// Get current LFO frequency in Hz (for display purposes)
    pub fn get_frequency_hz(&self) -> f32 {
        Self::dx7_rate_to_hz(self.rate)
    }

    /// Get current delay time in seconds (for display purposes)
    pub fn get_delay_seconds(&self) -> f32 {
        Self::dx7_delay_to_seconds(self.delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44_100.0;

    // -----------------------------------------------------------------------
    // LFOWaveform metadata
    // -----------------------------------------------------------------------

    #[test]
    fn waveform_default_is_triangle() {
        assert_eq!(LFOWaveform::default(), LFOWaveform::Triangle);
    }

    #[test]
    fn waveform_all_returns_six_variants() {
        assert_eq!(LFOWaveform::all().len(), 6);
    }

    #[test]
    fn waveform_names_are_unique_and_nonempty() {
        let mut names = Vec::new();
        for w in LFOWaveform::all() {
            let n = w.name();
            assert!(!n.is_empty());
            names.push(n);
        }
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len, "names should be unique");
    }

    // -----------------------------------------------------------------------
    // Construction & setters
    // -----------------------------------------------------------------------

    #[test]
    fn new_lfo_has_default_state() {
        let lfo = LFO::new(SR);
        assert_eq!(lfo.rate, 50.0);
        assert_eq!(lfo.waveform, LFOWaveform::Triangle);
        assert!(!lfo.key_sync);
    }

    #[test]
    fn setters_clamp_to_valid_range() {
        let mut lfo = LFO::new(SR);
        lfo.set_rate(200.0);
        assert_eq!(lfo.rate, 99.0);
        lfo.set_rate(-10.0);
        assert_eq!(lfo.rate, 0.0);

        lfo.set_delay(150.0);
        assert_eq!(lfo.delay, 99.0);

        lfo.set_pitch_depth(150.0);
        assert_eq!(lfo.pitch_depth, 99.0);

        lfo.set_amp_depth(150.0);
        assert_eq!(lfo.amp_depth, 99.0);
    }

    #[test]
    fn set_waveform_updates_field() {
        let mut lfo = LFO::new(SR);
        lfo.set_waveform(LFOWaveform::Square);
        assert_eq!(lfo.waveform, LFOWaveform::Square);
    }

    #[test]
    fn set_waveform_to_sample_hold_resets_state() {
        let mut lfo = LFO::new(SR);
        lfo.set_waveform(LFOWaveform::SampleHold);
        assert_eq!(lfo.waveform, LFOWaveform::SampleHold);
    }

    #[test]
    fn set_key_sync_toggles_field() {
        let mut lfo = LFO::new(SR);
        lfo.set_key_sync(true);
        assert!(lfo.key_sync);
        lfo.set_key_sync(false);
        assert!(!lfo.key_sync);
    }

    // -----------------------------------------------------------------------
    // Rate / delay conversions
    // -----------------------------------------------------------------------

    #[test]
    fn rate_zero_yields_zero_hz() {
        let lfo = LFO {
            rate: 0.0,
            ..LFO::new(SR)
        };
        assert_eq!(lfo.get_frequency_hz(), 0.0);
    }

    #[test]
    fn rate_increases_frequency_monotonically() {
        let make = |r: f32| {
            let mut l = LFO::new(SR);
            l.rate = r;
            l
        };
        let f_low = make(10.0).get_frequency_hz();
        let f_mid = make(50.0).get_frequency_hz();
        let f_high = make(99.0).get_frequency_hz();
        assert!(f_low < f_mid);
        assert!(f_mid < f_high);
    }

    #[test]
    fn delay_zero_is_zero_seconds() {
        let lfo = LFO::new(SR);
        assert_eq!(lfo.get_delay_seconds(), 0.0);
    }

    #[test]
    fn delay_99_is_around_five_seconds() {
        let mut lfo = LFO::new(SR);
        lfo.delay = 99.0;
        let secs = lfo.get_delay_seconds();
        assert!((secs - 5.0).abs() < 0.1);
    }

    // -----------------------------------------------------------------------
    // Trigger / delay
    // -----------------------------------------------------------------------

    #[test]
    fn trigger_with_key_sync_resets_phase() {
        let mut lfo = LFO::new(SR);
        lfo.set_key_sync(true);
        // Run a bit so phase advances
        for _ in 0..1000 {
            lfo.process(1.0);
        }
        let before = lfo.phase;
        lfo.trigger();
        assert_eq!(lfo.phase, 0.0);
        assert!(before > 0.0);
    }

    #[test]
    fn trigger_starts_delay_when_delay_nonzero() {
        let mut lfo = LFO::new(SR);
        lfo.delay = 50.0;
        lfo.trigger();
        // During delay, no modulation
        let (p, a) = lfo.process(1.0);
        assert_eq!(p, 0.0);
        assert_eq!(a, 0.0);
    }

    #[test]
    fn delay_eventually_releases_modulation() {
        let mut lfo = LFO::new(SR);
        lfo.delay = 1.0; // very short delay
        lfo.pitch_depth = 99.0;
        lfo.trigger();
        // Run for ~100ms (4410 samples) which is much longer than 1/99*5 ≈ 50ms
        let mut got_mod = false;
        for _ in 0..10000 {
            let (p, _) = lfo.process(1.0);
            if p.abs() > 1e-6 {
                got_mod = true;
                break;
            }
        }
        assert!(got_mod, "after delay, modulation should fire");
    }

    // -----------------------------------------------------------------------
    // Process / waveforms
    // -----------------------------------------------------------------------

    #[test]
    fn rate_zero_outputs_no_modulation() {
        let mut lfo = LFO::new(SR);
        lfo.rate = 0.0;
        lfo.pitch_depth = 99.0;
        let (p, a) = lfo.process(1.0);
        assert_eq!(p, 0.0);
        assert_eq!(a, 0.0);
    }

    #[test]
    fn mod_wheel_scales_modulation() {
        let mut lfo = LFO::new(SR);
        lfo.rate = 50.0;
        lfo.pitch_depth = 99.0;
        lfo.amp_depth = 99.0;
        // Burn many samples to step through the LFO cycle
        let mut p_full = 0.0_f32;
        let mut p_zero = 0.0_f32;
        for _ in 0..2048 {
            let (p, _) = lfo.process(1.0);
            p_full = p_full.max(p.abs());
        }
        let mut lfo_off = LFO::new(SR);
        lfo_off.rate = 50.0;
        lfo_off.pitch_depth = 99.0;
        for _ in 0..2048 {
            let (p, _) = lfo_off.process(0.0);
            p_zero = p_zero.max(p.abs());
        }
        assert!(p_full > p_zero);
        assert_eq!(p_zero, 0.0);
    }

    #[test]
    fn each_waveform_produces_modulation_in_range() {
        for &waveform in LFOWaveform::all() {
            let mut lfo = LFO::new(SR);
            lfo.set_waveform(waveform);
            lfo.rate = 50.0;
            lfo.pitch_depth = 99.0;
            lfo.amp_depth = 99.0;
            for _ in 0..512 {
                let (p, a) = lfo.process(1.0);
                assert!(p.abs() <= 1.01, "{:?} pitch out of range: {}", waveform, p);
                assert!(a.abs() <= 1.01, "{:?} amp out of range: {}", waveform, a);
            }
        }
    }

    #[test]
    fn triangle_waveform_oscillates_in_minus_one_to_plus_one() {
        let mut lfo = LFO::new(SR);
        lfo.set_waveform(LFOWaveform::Triangle);
        lfo.rate = 99.0;
        lfo.pitch_depth = 99.0;
        let mut min = 1.0_f32;
        let mut max = -1.0_f32;
        for _ in 0..(SR as usize / 2) {
            let (p, _) = lfo.process(1.0);
            min = min.min(p);
            max = max.max(p);
        }
        assert!(max > 0.5);
        assert!(min < -0.5);
    }

    #[test]
    fn square_waveform_is_bipolar() {
        let mut lfo = LFO::new(SR);
        lfo.set_waveform(LFOWaveform::Square);
        lfo.rate = 99.0;
        lfo.pitch_depth = 99.0;
        let mut saw_pos = false;
        let mut saw_neg = false;
        for _ in 0..(SR as usize / 2) {
            let (p, _) = lfo.process(1.0);
            if p > 0.5 {
                saw_pos = true;
            }
            if p < -0.5 {
                saw_neg = true;
            }
        }
        assert!(saw_pos && saw_neg);
    }

    #[test]
    fn sample_hold_produces_constant_runs() {
        let mut lfo = LFO::new(SR);
        lfo.set_waveform(LFOWaveform::SampleHold);
        lfo.rate = 50.0;
        lfo.pitch_depth = 99.0;
        // Drive a number of samples and verify the value plateaus before changing.
        let mut history = Vec::new();
        for _ in 0..2048 {
            let (p, _) = lfo.process(1.0);
            history.push(p);
        }
        // S&H should hold the same value for many consecutive samples between transitions.
        let mut max_run = 1usize;
        let mut current_run = 1usize;
        for w in history.windows(2) {
            if (w[0] - w[1]).abs() < 1e-6 {
                current_run += 1;
                max_run = max_run.max(current_run);
            } else {
                current_run = 1;
            }
        }
        assert!(
            max_run > 50,
            "S&H should hold value for many samples, max_run={max_run}"
        );
    }
}
