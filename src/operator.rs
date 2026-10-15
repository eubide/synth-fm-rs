use crate::envelope::{scale_out_level, Envelope};
use crate::optimization::fast_sin;
use std::f32::consts::PI;

/// DX7 AMS (amplitude mod sensitivity) ROM lookup, indexed 0..3.
///
/// Values come from the 8-bit DX7 ROM (`{0, 66, 109, 255}`) normalised to f32.
/// Source: `ampmodsenstab[4]` in MSFA / Dexed `dx7note.cc`, where the table is
/// stored as Q24 (`{0, 4_342_338, 7_171_437, 16_777_216}`) and divided by `1<<24`.
///
/// Replaces an earlier `{0.0, 0.09, 0.37, 1.0}` approximation whose intermediate
/// values were ~3× too low — patches with AMS=1 or AMS=2 lost most of their
/// LFO amplitude character.
const AMS_SCALE_TABLE: [f32; 4] = [0.0, 0.258_820_65, 0.427_440_64, 1.0];

/// DX7 ROM lookup for the four exponential scaling curves, used by the
/// keyboard level scaling formula. Indexed by `group` (0..32 inclusive).
///
/// Source: `exp_scale_data[33]` in MSFA / Dexed `dx7note.cc`.
const EXP_SCALE_DATA: [u8; 33] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 14, 16, 19, 23, 27, 33, 39, 47, 56, 66, 80, 94, 110, 126,
    142, 158, 174, 190, 206, 222, 238, 250,
];

/// DX7 ROM velocity scaling table, indexed by `MIDI_velocity >> 1` (0..63).
///
/// Source: `velocity_data[64]` in MSFA / Dexed `dx7note.cc`. The integer offset
/// `-239` sets the neutral point at MIDI velocity ≈ 100; values below are
/// attenuations and above are boosts (same convention as the DX7 hardware).
const VELOCITY_DATA: [u8; 64] = [
    0, 70, 86, 97, 106, 114, 121, 126, 132, 138, 142, 148, 152, 156, 160, 163, 166, 170, 173, 174,
    178, 181, 184, 186, 189, 190, 194, 196, 198, 200, 202, 205, 206, 209, 211, 214, 216, 218, 220,
    222, 224, 225, 227, 229, 230, 232, 233, 235, 237, 238, 240, 241, 242, 243, 244, 246, 246, 248,
    249, 250, 251, 252, 253, 254,
];

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
    params_dirty: bool,
}

impl CachedValues {
    fn new() -> Self {
        CachedValues { params_dirty: true }
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

        self.envelope
            .trigger(self.outlevel(), self.rate_scaling(note));

        // OSC KEY SYNC: when ON the phase resets so every note starts identically;
        // when OFF the oscillator free-runs to mimic the analog/DX1 behaviour.
        if self.oscillator_key_sync {
            self.phase = 0.0;
        }
        self.last_output = 0.0;
        self.prev_output = 0.0;
        self.cached_values.params_dirty = false;
    }

    fn update_cached_values(&mut self) {
        if !self.cached_values.params_dirty {
            return;
        }
        self.envelope.set_outlevel(self.outlevel());
        self.cached_values.params_dirty = false;
    }

    /// Output level, keyboard level scaling and velocity combined in
    /// microsteps (32 per output-level step), as in Dexed `Dx7Note::init`.
    fn outlevel(&self) -> i32 {
        let level = self.output_level.round().clamp(0.0, 99.0) as i32;
        let scaled = (scale_out_level(level) + self.key_level_scaling()).min(127);
        ((scaled << 5) + self.velocity_scaling()).max(0)
    }

    /// DX7 ROM `ScaleVelocity`, in microsteps: `velocity_data[v >> 1] - 239`
    /// puts the neutral point near MIDI velocity 100; sensitivity 0 ignores
    /// velocity entirely.
    fn velocity_scaling(&self) -> i32 {
        let midi_velocity = (self.current_velocity * 127.0).round().clamp(0.0, 127.0) as i32;
        let vel_value = VELOCITY_DATA[(midi_velocity >> 1) as usize] as i32 - 239;
        let sens = self.velocity_sensitivity.round().clamp(0.0, 7.0) as i32;
        ((sens * vel_value + 7) >> 3) << 4
    }

    /// DX7 keyboard level scaling in output-level steps (0.75 dB each). Port
    /// of `ScaleLevel` / `ScaleCurve` in MSFA `dx7note.cc`. The breakpoint
    /// defines a hinge: notes below use `left_curve`/`left_depth`, above use
    /// `right_curve`/`right_depth`. The hardware groups notes in 3-semitone
    /// blocks around four semitones below the breakpoint note.
    fn key_level_scaling(&self) -> i32 {
        // Dexed takes the breakpoint as the DX7 parameter byte (0 = A-1 = MIDI 21).
        let breakpoint_byte = self.key_scale_breakpoint as i32 - 21;
        let offset = self.current_note as i32 - breakpoint_byte - 17;
        let (group, depth, curve) = if offset >= 0 {
            (
                (offset + 1) / 3,
                self.key_scale_right_depth,
                self.key_scale_right_curve,
            )
        } else {
            (
                (1 - offset) / 3,
                self.key_scale_left_depth,
                self.key_scale_left_curve,
            )
        };

        if depth <= 0.0 {
            return 0;
        }
        let depth_int = depth.round().clamp(0.0, 99.0) as i32;
        let group = group.max(0);

        let magnitude = match curve {
            KeyScaleCurve::NegLin | KeyScaleCurve::PosLin => (group * depth_int * 329) >> 12,
            KeyScaleCurve::NegExp | KeyScaleCurve::PosExp => {
                let g = (group as usize).min(32);
                (EXP_SCALE_DATA[g] as i32 * depth_int * 329) >> 15
            }
        };

        match curve {
            KeyScaleCurve::NegLin | KeyScaleCurve::NegExp => -magnitude,
            KeyScaleCurve::PosLin | KeyScaleCurve::PosExp => magnitude,
        }
    }

    pub fn release(&mut self) {
        self.envelope.release();
    }

    /// DX7 detune in octaves, as measured on hardware for Dexed `osc_freq`:
    /// a step is ~1 cent at A4, wider in the bass and narrower in the treble.
    /// Fixed mode only honours detune above centre.
    fn detune_octaves(&self) -> f32 {
        if self.fixed_frequency {
            return self.detune.max(0.0) * 13457.0 / (1 << 24) as f32;
        }
        let octaves = self.base_frequency.max(1.0).log2();
        0.0209 * (-0.396 * octaves).exp() / 7.0 * octaves * self.detune
    }

    pub fn update_frequency(&mut self) {
        // FIXED mode bypasses the note-tracked base frequency and uses an absolute Hz value.
        let actual_freq = if self.fixed_frequency {
            self.fixed_freq_hz
        } else {
            self.base_frequency * self.frequency_ratio
        };
        let detuned_freq = if self.detune == 0.0 {
            actual_freq
        } else {
            actual_freq * self.detune_octaves().exp2()
        };

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

    /// Mark the cached values stale. Call after any bulk write to operator
    /// fields that bypasses the typed setters (preset apply, SysEx load).
    pub fn invalidate_cache(&mut self) {
        self.cached_values.params_dirty = true;
    }

    /// Setters that clamp to DX7 range and invalidate the cache. Use these
    /// from any path that writes during a sustained note.
    pub fn set_output_level(&mut self, level: f32) {
        self.output_level = level.clamp(0.0, 99.0);
        self.cached_values.params_dirty = true;
    }

    pub fn set_velocity_sensitivity(&mut self, sens: f32) {
        self.velocity_sensitivity = sens.clamp(0.0, 7.0);
        self.cached_values.params_dirty = true;
    }

    pub fn set_key_scale_breakpoint(&mut self, note: u8) {
        self.key_scale_breakpoint = note.min(127);
        self.cached_values.params_dirty = true;
    }

    pub fn set_key_scale_left_depth(&mut self, depth: f32) {
        self.key_scale_left_depth = depth.clamp(0.0, 99.0);
        self.cached_values.params_dirty = true;
    }

    pub fn set_key_scale_right_depth(&mut self, depth: f32) {
        self.key_scale_right_depth = depth.clamp(0.0, 99.0);
        self.cached_values.params_dirty = true;
    }

    pub fn set_key_scale_left_curve(&mut self, curve: KeyScaleCurve) {
        self.key_scale_left_curve = curve;
        self.cached_values.params_dirty = true;
    }

    pub fn set_key_scale_right_curve(&mut self, curve: KeyScaleCurve) {
        self.key_scale_right_curve = curve;
        self.cached_values.params_dirty = true;
    }

    /// Setters for fields read directly each sample (no cache).
    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 7.0);
    }

    pub fn set_key_scale_rate(&mut self, rate: f32) {
        self.key_scale_rate = rate.clamp(0.0, 7.0);
    }

    pub fn set_am_sensitivity(&mut self, sens: u8) {
        self.am_sensitivity = sens.min(3);
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
        let sin_result = fast_sin(self.phase + total_modulation);

        // DX7 AMS table (0..3): how much the LFO amplitude modulation affects this op.
        // 0 = none, 3 = maximum. Values come straight from the DX7 ROM via
        // `AMS_SCALE_TABLE` ({0, 66, 109, 255}/255).
        let ams_scale = AMS_SCALE_TABLE[self.am_sensitivity.min(3) as usize];
        let amp_mod_factor = 1.0 + (self.current_lfo_amp_mod * ams_scale);

        // EG Bias attenuates the op output by a static, controller-driven amount.
        // Gated by AMS (per DX7 manual): AMS=0 unaffected, AMS=3 fully attenuated up to ~70%.
        let eg_bias_factor = 1.0 - (self.current_eg_bias * ams_scale * 0.7);

        let output = sin_result * env_value * amp_mod_factor * eg_bias_factor;

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

    /// DX7 Key Rate Scaling — port of `ScaleRate` in MSFA `dx7note.cc`.
    ///
    /// Reference is fixed at MIDI 21 (A-1) and is **independent** of the
    /// per-operator level breakpoint (a previous version of this function
    /// reused `key_scale_breakpoint`, which is wrong: the DX7 KRS uses a
    /// hardware-implicit reference, not the patch's level scaling hinge).
    ///
    /// Integer ROM math:
    ///     x          = clamp(midinote / 3 - 7, 0, 31)
    ///     qratedelta = (sensitivity * x) >> 3
    /// `qratedelta` is in quarter-rate-step units; 4 quarter-steps double the
    /// envelope speed, so the multiplicative factor is `2^(qratedelta / 4)`.
    /// DX7 keyboard rate scaling (`ScaleRate`), in qrate units: every 4
    /// doubles the envelope speed.
    fn rate_scaling(&self, note: u8) -> i32 {
        let x = ((note as i32) / 3 - 7).clamp(0, 31);
        let sens = self.key_scale_rate.round().clamp(0.0, 7.0) as i32;
        (sens * x) >> 3
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
        assert!(
            peak > 0.05,
            "expected audible output after trigger, got peak={peak}"
        );
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
        assert!(
            !op.is_active(),
            "operator should reach inactive state after release"
        );
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

    fn detune_cents(base_hz: f32, detune: f32, fixed: bool) -> f32 {
        let mut op = Operator::new(SR);
        op.fixed_frequency = fixed;
        op.fixed_freq_hz = base_hz;
        op.detune = detune;
        op.trigger(base_hz, 1.0, 60);
        cents_offset(frequency_from_phase_increment(&op), base_hz)
    }

    #[test]
    fn detune_step_narrows_with_pitch_like_dexed() {
        // 0.0209 * e^(-0.396 * log2 f) / 7 * log2 f octaves per step.
        assert!((detune_cents(110.0, 7.0, false) - 11.60).abs() < 0.05);
        assert!((detune_cents(440.0, 7.0, false) - 6.80).abs() < 0.05);
        assert!((detune_cents(440.0, -7.0, false) + 6.80).abs() < 0.05);
    }

    #[test]
    fn fixed_mode_only_detunes_upward() {
        // Dexed `osc_freq` mode 1: 13457 / 2^24 octaves per step above centre.
        assert!(detune_cents(1000.0, -7.0, true).abs() < 1e-3);
        assert!((detune_cents(1000.0, 7.0, true) - 6.74).abs() < 0.05);
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

    /// Recover the operator's tuned frequency from its phase increment so we can
    /// assert on cents-level deviations regardless of internal representation.
    fn frequency_from_phase_increment(op: &Operator) -> f32 {
        op.phase_increment * op.sample_rate / (2.0 * PI)
    }

    fn cents_offset(actual_hz: f32, reference_hz: f32) -> f32 {
        1200.0 * (actual_hz / reference_hz).log2()
    }

    #[test]
    fn detune_zero_keeps_frequency_exact() {
        let mut op = Operator::new(SR);
        op.set_detune(0.0);
        op.trigger(440.0, 1.0, 60);
        let f = frequency_from_phase_increment(&op);
        assert!(
            (f - 440.0).abs() < 0.01,
            "detune=0 must be exact, got {f} Hz"
        );
    }

    #[test]
    fn detune_plus_seven_is_about_seven_cents_sharp() {
        // DX7 detune ±7 should be a *fine* offset (≈ ±7 cents), not a wild
        // percentage shift. Regression test for the bug that scaled detune as
        // `1 + detune/100`, producing ~+117 cents at detune=+7.
        let mut op = Operator::new(SR);
        op.set_detune(7.0);
        op.trigger(440.0, 1.0, 60);
        let f = frequency_from_phase_increment(&op);
        let cents = cents_offset(f, 440.0);
        assert!(
            (cents - 7.0).abs() < 0.5,
            "detune=+7 should produce ~+7 cents, got {cents:.2} cents (freq {f:.2} Hz)"
        );
    }

    #[test]
    fn detune_minus_seven_is_about_seven_cents_flat() {
        let mut op = Operator::new(SR);
        op.set_detune(-7.0);
        op.trigger(440.0, 1.0, 60);
        let f = frequency_from_phase_increment(&op);
        let cents = cents_offset(f, 440.0);
        assert!(
            (cents + 7.0).abs() < 0.5,
            "detune=-7 should produce ~-7 cents, got {cents:.2} cents (freq {f:.2} Hz)"
        );
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
        assert!(
            differ > 100,
            "modulation should change the waveform on most samples ({differ} differing)"
        );
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
        assert!(
            p_on > p_off * 1.1,
            "AMS=3 should boost over AMS=0: off={p_off}, on={p_on}"
        );
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
        assert!(
            high >= low,
            "depth=7 should scale at least as much as depth=1"
        );
    }

    #[test]
    fn velocity_is_ignored_at_sensitivity_zero() {
        let peak = |velocity: f32| {
            let mut op = Operator::new(SR);
            op.trigger(440.0, velocity, 60);
            warmup(&mut op, 2048)
        };
        assert!((peak(1.0) - peak(0.3)).abs() < 1e-3);
    }

    #[test]
    fn key_level_scaling_attenuates_in_output_level_steps() {
        // Note 86 is ten 3-semitone groups above a C3 breakpoint at depth 99,
        // -LIN: (10 * 99 * 329) >> 12 = 79 output-level steps = 79 * 32 microsteps.
        let peak = |depth: f32| {
            let mut op = Operator::new(SR);
            op.key_scale_breakpoint = 60;
            op.key_scale_right_curve = KeyScaleCurve::NegLin;
            op.key_scale_right_depth = depth;
            op.trigger(440.0, 1.0, 86);
            warmup(&mut op, 2048)
        };
        let expected = 2.0_f32.powf(-(79.0 * 32.0) / 256.0);
        let ratio = peak(99.0) / peak(0.0);
        assert!(
            (ratio / expected - 1.0).abs() < 0.03,
            "ratio {ratio}, expected {expected}"
        );
    }

    // -----------------------------------------------------------------------
    // Key scaling
    // -----------------------------------------------------------------------

    #[test]
    fn key_level_scaling_matches_dexed_scale_level() {
        // Dexed `ScaleLevel(note, 39, 99, 99, -LIN, -EXP)`; DX7 breakpoint
        // byte 39 is C3 = MIDI 60.
        let expected = [
            (21, -95),
            (36, -55),
            (48, -23),
            (50, -15),
            (53, -7),
            (54, -7),
            (55, 0),
            (56, 0),
            (57, 0),
            (58, 0),
            (59, 0),
            (60, 0),
            (72, -4),
            (84, -8),
            (96, -18),
            (108, -38),
        ];
        for (note, steps) in expected {
            let mut op = Operator::new(SR);
            op.key_scale_breakpoint = 60;
            op.key_scale_left_curve = KeyScaleCurve::NegLin;
            op.key_scale_right_curve = KeyScaleCurve::NegExp;
            op.key_scale_left_depth = 99.0;
            op.key_scale_right_depth = 99.0;
            op.trigger(440.0, 1.0, note);
            assert_eq!(op.key_level_scaling(), steps, "note {note}");
        }
    }

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
        assert!(
            p_far <= p_close,
            "neg-curve should attenuate far note: close={p_close} far={p_far}"
        );
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

    // -----------------------------------------------------------------------
    // Regression: live-tweak setters must invalidate the cache
    // -----------------------------------------------------------------------

    #[test]
    fn set_output_level_takes_effect_mid_note() {
        // Reproduces the params_dirty bug where direct field writes to
        // output_level were ignored until the next note-on.
        let mut op = Operator::new(SR);
        op.set_output_level(99.0);
        op.trigger(440.0, 1.0, 60);
        let peak_loud = warmup(&mut op, 4096);

        op.set_output_level(20.0);
        let peak_quiet = warmup(&mut op, 4096);

        assert!(
            peak_quiet < peak_loud * 0.5,
            "output_level change mid-note should quiet the operator: loud={peak_loud}, quiet={peak_quiet}"
        );
    }

    #[test]
    fn set_velocity_sensitivity_takes_effect_mid_note() {
        let mut op_a = Operator::new(SR);
        let mut op_b = Operator::new(SR);
        op_a.trigger(440.0, 0.5, 60);
        op_b.trigger(440.0, 0.5, 60);
        let peak_no_sens = warmup(&mut op_a, 4096);

        // Same low velocity but now with high sensitivity → should attenuate.
        op_b.set_velocity_sensitivity(7.0);
        let peak_sens = warmup(&mut op_b, 4096);

        assert!(
            peak_sens < peak_no_sens,
            "velocity sensitivity change should quiet the op at v=0.5: no_sens={peak_no_sens}, sens={peak_sens}"
        );
    }

    // -----------------------------------------------------------------------
    // Regression: setters clamp parameters to DX7 range
    // -----------------------------------------------------------------------

    #[test]
    fn set_feedback_clamps_above_seven() {
        let mut op = Operator::new(SR);
        op.set_feedback(99.0);
        assert_eq!(op.feedback, 7.0);
        op.set_feedback(-3.0);
        assert_eq!(op.feedback, 0.0);
    }

    #[test]
    fn set_output_level_clamps() {
        let mut op = Operator::new(SR);
        op.set_output_level(200.0);
        assert_eq!(op.output_level, 99.0);
        op.set_output_level(-5.0);
        assert_eq!(op.output_level, 0.0);
    }

    #[test]
    fn set_velocity_sensitivity_clamps() {
        let mut op = Operator::new(SR);
        op.set_velocity_sensitivity(50.0);
        assert_eq!(op.velocity_sensitivity, 7.0);
    }

    #[test]
    fn set_key_scale_rate_clamps() {
        let mut op = Operator::new(SR);
        op.set_key_scale_rate(99.0);
        assert_eq!(op.key_scale_rate, 7.0);
    }

    // -----------------------------------------------------------------------
    // Regression: Key Rate Scaling reference is MIDI 21 (A-1), not the
    // operator's level breakpoint.
    // -----------------------------------------------------------------------

    #[test]
    fn key_rate_scaling_is_independent_of_level_breakpoint() {
        // Two operators with the same KRS sensitivity playing the same note
        // but with very different level breakpoints. The KRS factor must be
        // identical because the DX7 reference is fixed at A-1, not the per-op
        // breakpoint.
        let mut op_low_bp = Operator::new(SR);
        let mut op_high_bp = Operator::new(SR);
        op_low_bp.key_scale_rate = 7.0;
        op_high_bp.key_scale_rate = 7.0;
        op_low_bp.key_scale_breakpoint = 24; // C0
        op_high_bp.key_scale_breakpoint = 96; // C6
        assert_eq!(op_low_bp.rate_scaling(60), op_high_bp.rate_scaling(60));
    }

    #[test]
    fn key_rate_scaling_at_a_minus_1_is_zero() {
        // ROM formula: x = clamp(midinote/3 - 7, 0, 31) → at midinote=21, x=0.
        let mut op = Operator::new(SR);
        op.key_scale_rate = 7.0;
        assert_eq!(op.rate_scaling(21), 0);
    }

    #[test]
    fn key_rate_scaling_matches_dexed_rom_formula() {
        // At C3 (midinote=60) with sens=7: x=13, qratedelta=(7*13)>>3=11.
        let mut op = Operator::new(SR);
        op.key_scale_rate = 7.0;
        assert_eq!(op.rate_scaling(60), 11);
    }
}
