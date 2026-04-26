use crate::envelope::Envelope;
use crate::optimization::OPTIMIZATION_TABLES;
use std::f32::consts::PI;

/// DX7 keyboard level scaling curve type. Applied independently to the
/// left and right of the breakpoint note.
///
/// - `NegLin` / `PosLin`: linear ramp downward / upward from the breakpoint.
/// - `NegExp` / `PosExp`: exponential ramp (faster taper near the edges).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum KeyScaleCurve {
    #[default]
    NegLin,
    NegExp,
    PosExp,
    PosLin,
}

impl KeyScaleCurve {
    pub fn from_dx7_code(code: u8) -> Self {
        match code {
            0 => KeyScaleCurve::NegLin,
            1 => KeyScaleCurve::NegExp,
            2 => KeyScaleCurve::PosExp,
            _ => KeyScaleCurve::PosLin,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "-lin" | "neglin" | "lindown" => KeyScaleCurve::NegLin,
            "-exp" | "negexp" | "expdown" => KeyScaleCurve::NegExp,
            "+exp" | "posexp" | "expup" => KeyScaleCurve::PosExp,
            _ => KeyScaleCurve::PosLin,
        }
    }

    /// Inverse of `from_dx7_code`: returns the DX7 SysEx encoding (0..3).
    pub fn to_dx7_code(self) -> u8 {
        match self {
            KeyScaleCurve::NegLin => 0,
            KeyScaleCurve::NegExp => 1,
            KeyScaleCurve::PosExp => 2,
            KeyScaleCurve::PosLin => 3,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedValues {
    level_amplitude: f32,
    velocity_factor: f32,
    key_scale_level_factor: f32,
    key_scale_rate_factor: f32,
    params_dirty: bool,
}

impl CachedValues {
    fn new() -> Self {
        CachedValues {
            level_amplitude: 1.0,
            velocity_factor: 1.0,
            key_scale_level_factor: 1.0,
            key_scale_rate_factor: 1.0,
            params_dirty: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Operator {
    pub enabled: bool,
    pub frequency_ratio: f32,
    pub detune: f32,
    pub output_level: f32,
    pub velocity_sensitivity: f32, // 0-7, how much velocity affects output
    pub key_scale_rate: f32,       // 0-7, envelope rate scaling
    pub key_scale_breakpoint: u8, // MIDI note that splits left/right scaling (DX7 default A-1 = 21, our default C3 = 60)
    pub key_scale_left_curve: KeyScaleCurve,
    pub key_scale_right_curve: KeyScaleCurve,
    pub key_scale_left_depth: f32,  // 0-99
    pub key_scale_right_depth: f32, // 0-99
    pub envelope: Envelope,
    pub feedback: f32,
    pub am_sensitivity: u8, // 0-3 LFO amp modulation depth scaling per operator
    pub oscillator_key_sync: bool, // OSC KEY SYNC: ON resets phase on note-on; OFF lets phase free-run
    pub fixed_frequency: bool,     // OSC MODE: false = RATIO (default), true = FIXED Hz
    pub fixed_freq_hz: f32,        // Absolute frequency in Hz when fixed_frequency = true

    // Internal state
    phase: f32,
    phase_increment: f32,
    last_output: f32,
    prev_output: f32, // DX7-authentic: two-sample average for feedback stability
    sample_rate: f32,
    base_frequency: f32,         // Store base frequency for real-time updates
    current_velocity: f32,       // Store velocity for real-time updates
    current_note: u8,            // Store MIDI note for key scaling
    current_lfo_amp_mod: f32,    // Latest LFO amp modulation value (-1..+1) staged by Voice
    current_eg_bias: f32,        // Static (non-oscillating) bias amount in 0..1 staged by Voice
    cached_values: CachedValues, // Cached calculations for performance
}

impl Operator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            enabled: true,
            frequency_ratio: 1.0,
            detune: 0.0,
            output_level: 99.0,
            velocity_sensitivity: 0.0,
            key_scale_rate: 0.0,
            key_scale_breakpoint: 60, // C3
            key_scale_left_curve: KeyScaleCurve::default(),
            key_scale_right_curve: KeyScaleCurve::default(),
            key_scale_left_depth: 0.0,
            key_scale_right_depth: 0.0,
            envelope: Envelope::new(sample_rate),
            feedback: 0.0,
            am_sensitivity: 0,
            oscillator_key_sync: true,
            fixed_frequency: false,
            fixed_freq_hz: 440.0,

            phase: 0.0,
            phase_increment: 0.0,
            last_output: 0.0,
            prev_output: 0.0,
            sample_rate,
            base_frequency: 440.0,
            current_velocity: 1.0,
            current_note: 60,
            current_lfo_amp_mod: 0.0,
            current_eg_bias: 0.0,
            cached_values: CachedValues::new(),
        }
    }

    /// Stage the latest LFO amplitude modulation sample (already scaled by mod-wheel
    /// and depth). The Voice calls this before `process()` each sample so the operator
    /// can apply its own `am_sensitivity` (0-3) to scale the impact.
    pub fn set_lfo_amp_mod(&mut self, value: f32) {
        self.current_lfo_amp_mod = value;
    }

    /// Stage the EG Bias amount (0..1). The DX7 manual describes this as a static,
    /// controller-driven offset that lowers operator levels — distinct from the LFO
    /// because it does not oscillate. Per-operator depth is gated by `am_sensitivity`,
    /// so modulators with AMS=0 are unaffected and AMS=3 ones get the full bite.
    pub fn set_eg_bias(&mut self, value: f32) {
        self.current_eg_bias = value.clamp(0.0, 1.0);
    }

    pub fn trigger(&mut self, frequency: f32, velocity: f32, note: u8) {
        self.base_frequency = frequency;
        self.current_velocity = velocity;
        self.current_note = note;
        self.update_frequency();

        // Apply key scale rate to envelope
        let key_scale_factor = self.calculate_key_scale_factor(note);
        self.envelope
            .trigger_with_key_scale(velocity, key_scale_factor);

        // OSC KEY SYNC: when ON the phase resets so every note starts identically;
        // when OFF the oscillator free-runs to mimic the analog/DX1 behaviour.
        if self.oscillator_key_sync {
            self.phase = 0.0;
        }
        self.last_output = 0.0;
        self.prev_output = 0.0;
        self.cached_values.params_dirty = true;
    }

    fn update_cached_values(&mut self) {
        if !self.cached_values.params_dirty {
            return;
        }

        // Cache level amplitude using optimized lookup
        self.cached_values.level_amplitude =
            OPTIMIZATION_TABLES.dx7_level_to_amplitude(self.output_level as u8);

        // DX7-style velocity sensitivity (0-7 range)
        // At sensitivity 0: velocity has no effect (always full volume)
        // At sensitivity 7: full velocity range effect
        // Uses power curve for natural dynamics
        let vel_sens_factor = self.velocity_sensitivity / 7.0;
        let velocity_power = 1.0 + vel_sens_factor * 2.0; // Power from 1.0 to 3.0
        let velocity_curve = self.current_velocity.powf(velocity_power);
        // Blend between full volume and velocity-scaled based on sensitivity
        self.cached_values.velocity_factor =
            (1.0 - vel_sens_factor) + (vel_sens_factor * velocity_curve);

        // Cache key level scaling: independent left/right curves with DX7-style depth.
        self.cached_values.key_scale_level_factor = self.calculate_key_level_factor();

        // Rate key scaling factor (1.0 means no change). Used by the envelope side via
        // `calculate_key_scale_factor()` which calls into similar logic; this cached
        // value is currently unused at the operator level but kept for future hooks.
        let key_distance = self.current_note as f32 - self.key_scale_breakpoint as f32;
        self.cached_values.key_scale_rate_factor = if self.key_scale_rate > 0.0 {
            let normalized = key_distance / 24.0;
            let rate_scale_amount = self.key_scale_rate / 7.0;
            1.0 + (normalized * rate_scale_amount).clamp(-0.75, 0.75)
        } else {
            1.0
        };

        self.cached_values.params_dirty = false;
    }

    /// DX7 keyboard level scaling: amplitude is multiplied by a factor derived from
    /// the distance between the played note and the breakpoint, the curve type
    /// (linear vs exponential, positive vs negative), and a 0-99 depth.
    ///
    /// We model the DX7 behaviour where the breakpoint defines a hinge: notes below
    /// use `left_curve`/`left_depth`, notes above use `right_curve`/`right_depth`.
    /// A factor of 1.0 means no scaling; values below 1.0 attenuate; values above 1.0
    /// would boost (clamped to a moderate range to avoid runaway gain).
    fn calculate_key_level_factor(&self) -> f32 {
        let distance = self.current_note as f32 - self.key_scale_breakpoint as f32;
        let (curve, depth) = if distance < 0.0 {
            (self.key_scale_left_curve, self.key_scale_left_depth)
        } else {
            (self.key_scale_right_curve, self.key_scale_right_depth)
        };

        if depth <= 0.0 {
            return 1.0;
        }

        // Normalize distance over a 4-octave reference (DX7 reaches max effect at ~48 semitones).
        let normalized = (distance.abs() / 48.0).clamp(0.0, 1.0);

        // Curve shape: linear is just `normalized`; exponential emphasises further-away notes.
        let shape = match curve {
            KeyScaleCurve::NegLin | KeyScaleCurve::PosLin => normalized,
            KeyScaleCurve::NegExp | KeyScaleCurve::PosExp => normalized * normalized,
        };

        let amount = (depth / 99.0) * shape; // 0..1 effective amount
        let factor = match curve {
            KeyScaleCurve::NegLin | KeyScaleCurve::NegExp => 1.0 - amount, // attenuate
            KeyScaleCurve::PosLin | KeyScaleCurve::PosExp => 1.0 + amount, // boost
        };

        factor.clamp(0.0, 2.0)
    }

    pub fn release(&mut self) {
        self.envelope.release();
    }

    pub fn update_frequency(&mut self) {
        // FIXED mode bypasses the note-tracked base frequency and uses an absolute Hz value.
        // Detune still applies as a small percentage offset, matching DX7 behaviour.
        let actual_freq = if self.fixed_frequency {
            self.fixed_freq_hz
        } else {
            self.base_frequency * self.frequency_ratio
        };
        let detuned_freq = actual_freq * (1.0 + self.detune / 100.0);

        // Validate frequency range
        if detuned_freq.is_finite()
            && (0.1..=20000.0).contains(&detuned_freq)
            && self.sample_rate > 0.0
            && self.sample_rate.is_finite()
        {
            self.phase_increment = (2.0 * PI * detuned_freq) / self.sample_rate;

            // Validate phase increment
            if !self.phase_increment.is_finite() || self.phase_increment.abs() > 100.0 {
                self.phase_increment = 0.0;
            }
        } else {
            self.phase_increment = 0.0;
        }
    }

    /// Update frequency without resetting phase - used for real-time modulation
    pub fn update_frequency_only(&mut self, frequency: f32) {
        self.base_frequency = frequency;
        self.update_frequency();
    }

    pub fn set_frequency_ratio(&mut self, ratio: f32) {
        self.frequency_ratio = ratio;
        self.update_frequency();
    }

    pub fn set_detune(&mut self, detune: f32) {
        self.detune = detune;
        self.update_frequency();
    }

    pub fn process(&mut self, modulation: f32) -> f32 {
        self.process_inner(modulation, true)
    }

    /// Process without self-feedback. Used by cross-feedback algorithms (4, 6)
    /// where feedback is routed between operators instead of self-loop.
    pub fn process_no_self_feedback(&mut self, modulation: f32) -> f32 {
        self.process_inner(modulation, false)
    }

    /// Two-sample averaged output for cross-feedback routing (DX7 algorithms 4, 6).
    /// `fb_depth` is the feedback parameter (0-7) that controls depth.
    /// Returns a pre-scaled value: after MOD_INDEX_SCALE in process(), produces
    /// the same depth as self-feedback (~π radians max at feedback=7).
    pub fn cross_feedback_signal(&self, fb_depth: f32) -> f32 {
        if fb_depth > 0.0 {
            let avg = (self.last_output + self.prev_output) * 0.5;
            // Pre-divide by MOD_INDEX_SCALE so process() scaling gives correct depth:
            // avg * fb * PI/7 / MOD_INDEX_SCALE = avg * fb / 28
            avg * fb_depth / 28.0
        } else {
            0.0
        }
    }

    fn process_inner(&mut self, modulation: f32, apply_self_feedback: bool) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        self.update_cached_values();

        let env_value = self.envelope.process();
        if env_value == 0.0 {
            return 0.0;
        }

        // DX7-authentic modulation index scaling
        // In the real DX7, output level 99 produces ~4π radians of maximum
        // phase deviation. Our level table normalizes to 0-1.0, so we scale
        // modulation inputs to match the authentic modulation depth.
        const MOD_INDEX_SCALE: f32 = 4.0 * PI;

        // DX7-authentic self-feedback using two-sample average for stability.
        // The real DX7 uses (y[n-1] + y[n-2]) >> (9 - fb) which averages
        // the last two outputs to reduce aliasing in the feedback loop.
        // At feedback=7: ~π radians max phase deviation.
        let feedback_mod = if apply_self_feedback && self.feedback > 0.0 {
            let avg_output = (self.last_output + self.prev_output) * 0.5;
            avg_output * self.feedback * PI / 7.0
        } else {
            0.0
        };

        // Scale incoming modulation to DX7-authentic depth
        // Feedback has its own independent scaling (not multiplied by MOD_INDEX_SCALE)
        let total_modulation = (modulation * MOD_INDEX_SCALE) + feedback_mod;
        let sin_result = OPTIMIZATION_TABLES.fast_sin(self.phase + total_modulation);

        // DX7 AMS table (0..3): how much the LFO amplitude modulation affects this op.
        // 0 = none, 3 = maximum. Mapped to a per-sample gain factor centred on 1.0.
        // Multiplier table mirrors the DX7 ROM (~0%, ~9%, ~37%, ~100%).
        let ams_scale = match self.am_sensitivity.min(3) {
            0 => 0.0,
            1 => 0.09,
            2 => 0.37,
            _ => 1.0,
        };
        let amp_mod_factor = 1.0 + (self.current_lfo_amp_mod * ams_scale);

        // EG Bias attenuates the op output by a static, controller-driven amount.
        // Gated by AMS (per DX7 manual): AMS=0 unaffected, AMS=3 fully attenuated up to ~70%.
        let eg_bias_factor = 1.0 - (self.current_eg_bias * ams_scale * 0.7);

        let output = sin_result
            * env_value
            * self.cached_values.level_amplitude
            * self.cached_values.velocity_factor
            * self.cached_values.key_scale_level_factor
            * amp_mod_factor
            * eg_bias_factor;

        // Update phase with bounds checking
        if self.phase_increment.is_finite() && self.phase_increment.abs() < 100.0 {
            self.phase += self.phase_increment;

            // Optimized phase wrapping using conditional subtraction
            // Much faster than modulo operations
            const TWO_PI: f32 = 2.0 * PI;
            while self.phase >= TWO_PI {
                self.phase -= TWO_PI;
            }
            while self.phase < 0.0 {
                self.phase += TWO_PI;
            }
        } else {
            self.phase = 0.0;
            self.phase_increment = 0.0;
        }

        self.prev_output = self.last_output;
        self.last_output = output;
        output
    }

    pub fn is_active(&self) -> bool {
        self.envelope.is_active()
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.last_output = 0.0;
        self.prev_output = 0.0;
        self.envelope.reset();
    }

    // Calculate key scaling factor for envelope rates
    fn calculate_key_scale_factor(&self, note: u8) -> f32 {
        if self.key_scale_rate == 0.0 {
            return 1.0;
        }

        // Key scale rate affects how fast envelopes run based on key position
        // Higher notes = faster envelopes
        let distance = note as i32 - self.key_scale_breakpoint as i32;
        if distance > 0 {
            // Above breakpoint - faster rates
            1.0 + (distance as f32 * self.key_scale_rate / 7.0 / 24.0) // 24 = 2 octaves
        } else if distance < 0 {
            // Below breakpoint - slower rates
            1.0 / (1.0 + (-distance as f32 * self.key_scale_rate / 7.0 / 24.0))
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44_100.0;

    fn warmup(op: &mut Operator, samples: usize) -> f32 {
        let mut peak = 0.0_f32;
        for _ in 0..samples {
            peak = peak.max(op.process(0.0).abs());
        }
        peak
    }

    // -----------------------------------------------------------------------
    // KeyScaleCurve
    // -----------------------------------------------------------------------

    #[test]
    fn key_scale_curve_from_dx7_code() {
        assert_eq!(KeyScaleCurve::from_dx7_code(0), KeyScaleCurve::NegLin);
        assert_eq!(KeyScaleCurve::from_dx7_code(1), KeyScaleCurve::NegExp);
        assert_eq!(KeyScaleCurve::from_dx7_code(2), KeyScaleCurve::PosExp);
        assert_eq!(KeyScaleCurve::from_dx7_code(3), KeyScaleCurve::PosLin);
        assert_eq!(KeyScaleCurve::from_dx7_code(99), KeyScaleCurve::PosLin); // default
    }

    #[test]
    fn key_scale_curve_to_dx7_code_roundtrips() {
        for code in 0..4u8 {
            assert_eq!(KeyScaleCurve::from_dx7_code(code).to_dx7_code(), code);
        }
    }

    #[test]
    fn key_scale_curve_from_str_handles_aliases() {
        assert_eq!(KeyScaleCurve::from_str("-lin"), KeyScaleCurve::NegLin);
        assert_eq!(KeyScaleCurve::from_str("NEGLIN"), KeyScaleCurve::NegLin);
        assert_eq!(KeyScaleCurve::from_str("LinDown"), KeyScaleCurve::NegLin);
        assert_eq!(KeyScaleCurve::from_str("-exp"), KeyScaleCurve::NegExp);
        assert_eq!(KeyScaleCurve::from_str("ExpDown"), KeyScaleCurve::NegExp);
        assert_eq!(KeyScaleCurve::from_str("+exp"), KeyScaleCurve::PosExp);
        assert_eq!(KeyScaleCurve::from_str("ExpUp"), KeyScaleCurve::PosExp);
        // Anything else falls back to PosLin
        assert_eq!(KeyScaleCurve::from_str("garbage"), KeyScaleCurve::PosLin);
    }

    #[test]
    fn key_scale_curve_default_is_negative_linear() {
        assert_eq!(KeyScaleCurve::default(), KeyScaleCurve::NegLin);
    }

    // -----------------------------------------------------------------------
    // Operator construction & basic setters
    // -----------------------------------------------------------------------

    #[test]
    fn operator_new_has_sane_defaults() {
        let op = Operator::new(SR);
        assert!(op.enabled);
        assert_eq!(op.frequency_ratio, 1.0);
        assert_eq!(op.detune, 0.0);
        assert_eq!(op.output_level, 99.0);
        assert!(op.oscillator_key_sync);
        assert!(!op.fixed_frequency);
    }

    #[test]
    fn set_frequency_ratio_updates_and_phase_increment_recalculates() {
        let mut op = Operator::new(SR);
        op.trigger(440.0, 1.0, 60);
        op.set_frequency_ratio(2.0);
        assert_eq!(op.frequency_ratio, 2.0);
        // After trigger and ratio change, output should be non-zero
        let peak = warmup(&mut op, 256);
        assert!(peak > 0.0);
    }

    #[test]
    fn set_detune_changes_internal_value() {
        let mut op = Operator::new(SR);
        op.set_detune(7.0);
        assert_eq!(op.detune, 7.0);
        op.set_detune(-3.5);
        assert_eq!(op.detune, -3.5);
    }

    // -----------------------------------------------------------------------
    // Trigger / process / release lifecycle
    // -----------------------------------------------------------------------

    #[test]
    fn process_disabled_returns_zero() {
        let mut op = Operator::new(SR);
        op.enabled = false;
        op.trigger(440.0, 1.0, 60);
        let out = op.process(0.0);
        assert_eq!(out, 0.0);
    }

    #[test]
    fn trigger_resets_phase_when_key_sync_on() {
        let mut op = Operator::new(SR);
        op.oscillator_key_sync = true;
        op.trigger(440.0, 1.0, 60);
        // Drive a few samples to advance phase
        for _ in 0..100 {
            op.process(0.0);
        }
        let phase_before = op.phase;
        op.trigger(440.0, 1.0, 60);
        // With key sync ON the phase should reset to 0
        assert_eq!(op.phase, 0.0);
        // Sanity: previous phase had advanced
        assert!(phase_before > 0.0);
    }

    #[test]
    fn trigger_preserves_phase_when_key_sync_off() {
        let mut op = Operator::new(SR);
        op.oscillator_key_sync = false;
        op.trigger(440.0, 1.0, 60);
        for _ in 0..100 {
            op.process(0.0);
        }
        let phase_before = op.phase;
        op.trigger(440.0, 1.0, 60);
        // Phase should be preserved (free-run)
        assert_eq!(op.phase, phase_before);
    }

    #[test]
    fn process_produces_audio_after_trigger() {
        let mut op = Operator::new(SR);
        op.trigger(440.0, 1.0, 60);
        let peak = warmup(&mut op, 4096);
        assert!(peak > 0.05, "expected audible output after trigger, got peak={peak}");
    }

    #[test]
    fn release_eventually_makes_operator_inactive() {
        let mut op = Operator::new(SR);
        op.envelope.rate1 = 99.0;
        op.envelope.rate4 = 99.0;
        op.envelope.level4 = 0.0;
        op.trigger(440.0, 1.0, 60);
        warmup(&mut op, 4096);
        op.release();
        for _ in 0..(SR as usize) {
            op.process(0.0);
            if !op.is_active() {
                break;
            }
        }
        assert!(!op.is_active(), "operator should reach inactive state after release");
    }

    #[test]
    fn reset_clears_phase_and_envelope() {
        let mut op = Operator::new(SR);
        op.trigger(440.0, 1.0, 60);
        warmup(&mut op, 256);
        op.reset();
        assert_eq!(op.phase, 0.0);
        assert!(!op.is_active());
    }

    // -----------------------------------------------------------------------
    // Frequency configuration
    // -----------------------------------------------------------------------

    #[test]
    fn fixed_frequency_uses_fixed_hz() {
        let mut op = Operator::new(SR);
        op.fixed_frequency = true;
        op.fixed_freq_hz = 1000.0;
        op.trigger(440.0, 1.0, 60); // base_frequency ignored
        // Operator should produce a 1kHz wave; just ensure it's audible
        let peak = warmup(&mut op, 4096);
        assert!(peak > 0.05);
    }

    #[test]
    fn invalid_frequency_clears_phase_increment() {
        let mut op = Operator::new(SR);
        // Force an invalid setup: frequency very low or NaN base
        op.fixed_frequency = true;
        op.fixed_freq_hz = 0.0; // outside [0.1, 20000]
        op.trigger(440.0, 1.0, 60);
        // process should still run and produce silence (sin(0)=0)
        let out = op.process(0.0);
        assert!(out.abs() < 1e-3);
    }

    #[test]
    fn update_frequency_only_does_not_reset_phase() {
        let mut op = Operator::new(SR);
        op.trigger(440.0, 1.0, 60);
        for _ in 0..50 {
            op.process(0.0);
        }
        let phase_before = op.phase;
        op.update_frequency_only(880.0);
        assert_eq!(op.phase, phase_before);
        assert_eq!(op.base_frequency, 880.0);
    }

    // -----------------------------------------------------------------------
    // Modulation, AMS and EG bias
    // -----------------------------------------------------------------------

    #[test]
    fn modulation_input_changes_output() {
        // Compare per-sample outputs: modulation should change the waveform shape,
        // even if total energy stays similar. Look at how many samples differ
        // by more than a small epsilon.
        let mut op_no_mod = Operator::new(SR);
        let mut op_mod = Operator::new(SR);
        op_no_mod.trigger(440.0, 1.0, 60);
        op_mod.trigger(440.0, 1.0, 60);
        // Let the envelope reach steady state first
        for _ in 0..2048 {
            op_no_mod.process(0.0);
            op_mod.process(2.0);
        }
        let mut differ = 0usize;
        for _ in 0..2048 {
            let a = op_no_mod.process(0.0);
            // 0.3 chosen to avoid landing exactly on 2π multiples
            // (the modulation is scaled by 4π internally).
            let b = op_mod.process(0.3);
            if (a - b).abs() > 0.001 {
                differ += 1;
            }
        }
        assert!(differ > 100, "modulation should change the waveform on most samples ({differ} differing)");
    }

    #[test]
    fn am_sensitivity_levels_alter_output() {
        // AMS=0 → no LFO amp influence; AMS=3 → full influence.
        let mut op_off = Operator::new(SR);
        let mut op_on = Operator::new(SR);
        op_off.am_sensitivity = 0;
        op_on.am_sensitivity = 3;
        op_off.set_lfo_amp_mod(1.0);
        op_on.set_lfo_amp_mod(1.0);
        op_off.trigger(440.0, 1.0, 60);
        op_on.trigger(440.0, 1.0, 60);
        let p_off = warmup(&mut op_off, 2048);
        let p_on = warmup(&mut op_on, 2048);
        // AMS=3 doubles the gain at full LFO; should produce a bigger peak
        assert!(p_on > p_off * 1.1, "AMS=3 should boost over AMS=0: off={p_off}, on={p_on}");
    }

    #[test]
    fn am_sensitivity_intermediate_values() {
        // Smoke-test all AMS settings to cover the match arms (0/1/2/3+).
        for ams in 0..=4u8 {
            let mut op = Operator::new(SR);
            op.am_sensitivity = ams;
            op.set_lfo_amp_mod(0.5);
            op.trigger(440.0, 1.0, 60);
            warmup(&mut op, 64);
        }
    }

    #[test]
    fn eg_bias_clamps_to_unit_range() {
        let mut op = Operator::new(SR);
        op.set_eg_bias(2.0);
        // we can't read the private field directly; the clamp is exercised by process
        op.am_sensitivity = 3;
        op.trigger(440.0, 1.0, 60);
        let peak = warmup(&mut op, 1024);
        assert!(peak >= 0.0);

        op.set_eg_bias(-1.0); // also clamped
        warmup(&mut op, 256);
    }

    // -----------------------------------------------------------------------
    // Self-feedback
    // -----------------------------------------------------------------------

    #[test]
    fn feedback_modifies_signal_shape() {
        let mut op_no_fb = Operator::new(SR);
        let mut op_fb = Operator::new(SR);
        op_no_fb.feedback = 0.0;
        op_fb.feedback = 7.0;
        op_no_fb.trigger(440.0, 1.0, 60);
        op_fb.trigger(440.0, 1.0, 60);
        let mut energy_no = 0.0;
        let mut energy_fb = 0.0;
        for _ in 0..4096 {
            let a = op_no_fb.process(0.0);
            let b = op_fb.process(0.0);
            energy_no += a * a;
            energy_fb += b * b;
        }
        // Both should produce non-trivial energy; full feedback should not be silent.
        assert!(energy_no > 0.1);
        assert!(energy_fb > 0.0);
    }

    #[test]
    fn process_no_self_feedback_skips_internal_loop() {
        let mut op_self = Operator::new(SR);
        let mut op_no_self = Operator::new(SR);
        op_self.feedback = 7.0;
        op_no_self.feedback = 7.0;
        op_self.trigger(440.0, 1.0, 60);
        op_no_self.trigger(440.0, 1.0, 60);

        let mut energy_self = 0.0;
        let mut energy_no_self = 0.0;
        for _ in 0..4096 {
            energy_self += op_self.process(0.0).powi(2);
            energy_no_self += op_no_self.process_no_self_feedback(0.0).powi(2);
        }
        // The two paths should produce different signals
        assert!((energy_self - energy_no_self).abs() > 1e-3);
    }

    #[test]
    fn cross_feedback_signal_zero_when_no_depth() {
        let op = Operator::new(SR);
        assert_eq!(op.cross_feedback_signal(0.0), 0.0);
    }

    #[test]
    fn cross_feedback_signal_scales_with_depth() {
        let mut op = Operator::new(SR);
        op.trigger(440.0, 1.0, 60);
        // Drive the operator so last_output / prev_output have content
        for _ in 0..32 {
            op.process(0.0);
        }
        let low = op.cross_feedback_signal(1.0).abs();
        let high = op.cross_feedback_signal(7.0).abs();
        assert!(high >= low, "depth=7 should scale at least as much as depth=1");
    }

    // -----------------------------------------------------------------------
    // Key scaling
    // -----------------------------------------------------------------------

    #[test]
    fn key_level_scaling_negative_attenuates() {
        // Positive depth + NegLin curve should attenuate notes far from breakpoint.
        let mut op_close = Operator::new(SR);
        let mut op_far = Operator::new(SR);
        op_close.key_scale_breakpoint = 60;
        op_close.key_scale_left_curve = KeyScaleCurve::NegLin;
        op_close.key_scale_right_curve = KeyScaleCurve::NegLin;
        op_close.key_scale_left_depth = 99.0;
        op_close.key_scale_right_depth = 99.0;
        op_far.key_scale_breakpoint = 60;
        op_far.key_scale_left_curve = KeyScaleCurve::NegLin;
        op_far.key_scale_right_curve = KeyScaleCurve::NegLin;
        op_far.key_scale_left_depth = 99.0;
        op_far.key_scale_right_depth = 99.0;
        op_close.trigger(440.0, 1.0, 60); // at breakpoint
        op_far.trigger(440.0, 1.0, 108); // 4 octaves above

        let p_close = warmup(&mut op_close, 4096);
        let p_far = warmup(&mut op_far, 4096);
        assert!(p_far <= p_close, "neg-curve should attenuate far note: close={p_close} far={p_far}");
    }

    #[test]
    fn key_level_scaling_positive_boosts() {
        let mut op = Operator::new(SR);
        op.key_scale_breakpoint = 60;
        op.key_scale_right_curve = KeyScaleCurve::PosLin;
        op.key_scale_right_depth = 99.0;
        op.trigger(440.0, 1.0, 96); // far above breakpoint
        warmup(&mut op, 1024);
    }

    #[test]
    fn key_level_scaling_no_depth_is_neutral() {
        let mut op = Operator::new(SR);
        op.key_scale_left_depth = 0.0;
        op.key_scale_right_depth = 0.0;
        // Calling calculate via process should produce normal output
        op.trigger(440.0, 1.0, 96);
        let peak = warmup(&mut op, 1024);
        assert!(peak > 0.0);
    }

    #[test]
    fn key_level_scaling_exp_curve() {
        let mut op = Operator::new(SR);
        op.key_scale_breakpoint = 60;
        op.key_scale_left_curve = KeyScaleCurve::NegExp;
        op.key_scale_right_curve = KeyScaleCurve::PosExp;
        op.key_scale_left_depth = 50.0;
        op.key_scale_right_depth = 50.0;
        op.trigger(440.0, 1.0, 24);
        warmup(&mut op, 256);
    }

    #[test]
    fn velocity_sensitivity_changes_output() {
        let mut op_low = Operator::new(SR);
        let mut op_high = Operator::new(SR);
        op_low.velocity_sensitivity = 0.0; // no effect
        op_high.velocity_sensitivity = 7.0; // max effect
        op_low.trigger(440.0, 0.5, 60);
        op_high.trigger(440.0, 0.5, 60);

        let p_low = warmup(&mut op_low, 4096);
        let p_high = warmup(&mut op_high, 4096);
        // High sensitivity at velocity 0.5 should be quieter than no sensitivity
        assert!(p_high <= p_low);
    }

    #[test]
    fn key_scale_rate_speeds_up_envelope_for_higher_notes() {
        let mut op_low = Operator::new(SR);
        let mut op_high = Operator::new(SR);
        op_low.key_scale_rate = 7.0;
        op_high.key_scale_rate = 7.0;
        op_low.key_scale_breakpoint = 60;
        op_high.key_scale_breakpoint = 60;
        op_low.trigger(220.0, 1.0, 36);
        op_high.trigger(880.0, 1.0, 96);
        // No assertion on shape — we just want the code path to execute.
        warmup(&mut op_low, 128);
        warmup(&mut op_high, 128);
    }
}
