use crate::optimization::OPTIMIZATION_TABLES;
use std::f32::consts::PI;

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

    /// Convert DX7 rate (0-99) to Hz using optimized exponential lookup
    fn dx7_rate_to_hz(rate: f32) -> f32 {
        if rate <= 0.0 {
            0.0
        } else {
            // Use optimized exponential lookup table
            // Map rate to exponential curve: approximately 0.062Hz to 20Hz
            let normalized = (rate / 99.0).clamp(0.0, 1.0);
            // Scale normalized value for exponential range (6.0 gives ~20Hz max)
            let exp_input = normalized; // Already 0-1 for fast_exp
            0.062 * (1.0 + OPTIMIZATION_TABLES.fast_exp(exp_input) * 320.0)
        }
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
            LFOWaveform::Sine => OPTIMIZATION_TABLES.fast_sin(phase * 2.0 * PI),

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
