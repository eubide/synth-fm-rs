use crate::envelope::Envelope;
use std::f32::consts::PI;

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
    base_frequency: f32,   // Store base frequency for real-time updates
    current_velocity: f32, // Store velocity for real-time updates
    current_note: u8,      // Store MIDI note for key scaling
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
    }

    pub fn release(&mut self) {
        self.envelope.release();
    }

    // Update frequency when parameters change
    pub fn update_frequency(&mut self) {
        let actual_freq = if let Some(fixed) = self.frequency_fixed {
            fixed
        } else {
            self.base_frequency * self.frequency_ratio
        };

        let detuned_freq = actual_freq * (1.0 + self.detune / 100.0);
        self.phase_increment = (2.0 * PI * detuned_freq) / self.sample_rate;
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
        let env_value = self.envelope.process();

        if env_value == 0.0 {
            return 0.0;
        }

        // Apply velocity sensitivity (0-7 range, where 7 = maximum sensitivity)
        let velocity_factor = if self.velocity_sensitivity > 0.0 {
            let sensitivity = self.velocity_sensitivity / 7.0; // Normalize to 0-1
            1.0 - sensitivity * (1.0 - self.current_velocity)
        } else {
            1.0
        };

        // Apply key scaling to level
        let key_scale_level_factor = if self.key_scale_level > 0.0 {
            let distance =
                (self.current_note as i32 - self.key_scale_breakpoint as i32).abs() as f32;
            let scaling = 1.0 - (distance * self.key_scale_level / 99.0 / 48.0); // 48 = 4 octaves
            scaling.max(0.0).min(1.0)
        } else {
            1.0
        };

        // Feedback: DX7 uses previous output as phase modulation
        // Feedback range 0-7 maps to 0-π radians of modulation
        let feedback_mod = if self.feedback > 0.0 {
            self.last_output * self.feedback * PI / 7.0
        } else {
            0.0
        };

        // Total phase modulation
        let total_modulation = modulation + feedback_mod;

        // Generate output with phase modulation and apply all scaling factors
        let output = (self.phase + total_modulation).sin()
            * env_value
            * (self.output_level / 99.0)
            * velocity_factor
            * key_scale_level_factor;

        // Update phase for next sample
        self.phase += self.phase_increment;
        while self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        // Store for feedback
        self.last_output = output;

        // Soft clipping for stability
        output.tanh()
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
}
