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
    pub frequency_ratio: f32,
    pub frequency_fixed: Option<f32>,
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
    sample_rate: f32,
    base_frequency: f32,         // Store base frequency for real-time updates
    current_velocity: f32,       // Store velocity for real-time updates
    current_note: u8,            // Store MIDI note for key scaling
    cached_values: CachedValues, // Cached calculations for performance
}

impl Operator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            frequency_ratio: 1.0,
            frequency_fixed: None,
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
        self.cached_values.params_dirty = true;
    }

    fn update_cached_values(&mut self) {
        if !self.cached_values.params_dirty {
            return;
        }

        // Cache level amplitude using optimized lookup
        self.cached_values.level_amplitude =
            OPTIMIZATION_TABLES.dx7_level_to_amplitude(self.output_level as u8);

        // Cache velocity factor (exponential curve for natural response)
        let vel_sens_factor = self.velocity_sensitivity / 7.0;
        let velocity_curve = 1.0 - vel_sens_factor + (vel_sens_factor * self.current_velocity);
        self.cached_values.velocity_factor = velocity_curve.clamp(0.0, 1.0);

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
        let actual_freq = self
            .frequency_fixed
            .unwrap_or(self.base_frequency * self.frequency_ratio);
        let detuned_freq = actual_freq * (1.0 + self.detune / 100.0);

        // Validate frequency range
        if detuned_freq.is_finite()
            && detuned_freq >= 0.1
            && detuned_freq <= 20000.0
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

    /// Get the previous output for feedback routing
    pub fn get_feedback_output(&self) -> f32 {
        self.last_output * self.feedback
    }

    pub fn process(&mut self, modulation: f32) -> f32 {
        self.update_cached_values();

        let env_value = self.envelope.process();
        if env_value == 0.0 {
            return 0.0;
        }

        // Apply feedback modulation (DX7: 0-7 maps to 0-π radians)
        let feedback_mod = if self.feedback > 0.0 {
            self.last_output * self.feedback * PI / 7.0
        } else {
            0.0
        };

        // Generate output with phase modulation
        let total_modulation = modulation + feedback_mod;
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

        self.last_output = output;

        // Apply gentle soft clipping for operators
        self.soft_clip_operator(output)
    }

    pub fn is_active(&self) -> bool {
        self.envelope.is_active()
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.last_output = 0.0;
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

    /// Gentle soft clipping for operator output
    fn soft_clip_operator(&self, sample: f32) -> f32 {
        const THRESHOLD: f32 = 0.9; // Higher threshold for operators
        const SOFTNESS: f32 = 0.1; // Gentle softening

        if sample.abs() <= THRESHOLD {
            sample
        } else {
            let sign = sample.signum();
            let abs_sample = sample.abs();

            // Gentle compression for operator clipping
            let excess = abs_sample - THRESHOLD;
            let softened = excess / (1.0 + excess / SOFTNESS);

            sign * (THRESHOLD + softened).min(1.0)
        }
    }
}
