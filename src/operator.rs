use crate::envelope::Envelope;
use crate::optimization::OPTIMIZATION_TABLES;
use std::f32::consts::PI;

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
    pub key_scale_level: f32,      // 0-99, level scaling across keyboard
    pub key_scale_rate: f32,       // 0-7, envelope rate scaling
    pub key_scale_breakpoint: u8,  // Note number for scaling center (C3 = 60)
    pub envelope: Envelope,
    pub feedback: f32,

    // Internal state
    phase: f32,
    phase_increment: f32,
    last_output: f32,
    prev_output: f32, // DX7-authentic: two-sample average for feedback stability
    sample_rate: f32,
    base_frequency: f32,         // Store base frequency for real-time updates
    current_velocity: f32,       // Store velocity for real-time updates
    current_note: u8,            // Store MIDI note for key scaling
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
            key_scale_level: 0.0,
            key_scale_rate: 0.0,
            key_scale_breakpoint: 60, // C3
            envelope: Envelope::new(sample_rate),
            feedback: 0.0,

            phase: 0.0,
            phase_increment: 0.0,
            last_output: 0.0,
            prev_output: 0.0,
            sample_rate,
            base_frequency: 440.0,
            current_velocity: 1.0,
            current_note: 60,
            cached_values: CachedValues::new(),
        }
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

        self.phase = 0.0;
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

        // Cache key scaling factors
        let key_distance = self.current_note as f32 - self.key_scale_breakpoint as f32;
        let key_scale_normalized = key_distance / 24.0; // ±24 semitones range

        // Level key scaling
        self.cached_values.key_scale_level_factor = if self.key_scale_level > 0.0 {
            let level_scale_amount = self.key_scale_level / 99.0;
            1.0 + (key_scale_normalized * level_scale_amount).clamp(-0.5, 0.5)
        } else {
            1.0
        };

        // Rate key scaling
        self.cached_values.key_scale_rate_factor = if self.key_scale_rate > 0.0 {
            let rate_scale_amount = self.key_scale_rate / 7.0;
            1.0 + (key_scale_normalized * rate_scale_amount).clamp(-0.75, 0.75)
        } else {
            1.0
        };

        self.cached_values.params_dirty = false;
    }

    pub fn release(&mut self) {
        self.envelope.release();
    }

    pub fn update_frequency(&mut self) {
        let actual_freq = self.base_frequency * self.frequency_ratio;
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
        let output = sin_result
            * env_value
            * self.cached_values.level_amplitude
            * self.cached_values.velocity_factor
            * self.cached_values.key_scale_level_factor;

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
