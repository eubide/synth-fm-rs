use std::f32::consts::PI;

// DX7-style lookup tables for performance optimization

pub struct OptimizationTables {
    sine_table: [f32; 4096],
    exp_table: [f32; 256],
    midi_frequencies: [f32; 128],
    voice_scale_table: [f32; 17], // For voice counts 0-16
    dx7_level_table: [f32; 100],  // Authentic DX7 level-to-amplitude
    dx7_rate_table: [f32; 100],   // Authentic DX7 rate-to-time
}

impl OptimizationTables {
    pub fn new() -> Self {
        let mut tables = OptimizationTables {
            sine_table: [0.0; 4096],
            exp_table: [0.0; 256],
            midi_frequencies: [0.0; 128],
            voice_scale_table: [0.0; 17],
            dx7_level_table: [0.0; 100],
            dx7_rate_table: [0.0; 100],
        };

        tables.init_sine_table();
        tables.init_exp_table();
        tables.init_midi_frequencies();
        tables.init_voice_scale_table();
        tables.init_dx7_level_table();
        tables.init_dx7_rate_table();

        tables
    }

    // Initialize sine table with 4096 entries (12-bit precision like original DX7)
    fn init_sine_table(&mut self) {
        for i in 0..4096 {
            let phase = (i as f32 / 4096.0) * 2.0 * PI;
            self.sine_table[i] = phase.sin();
        }
    }

    // Initialize exponential table for envelope scaling
    fn init_exp_table(&mut self) {
        for i in 0..256 {
            // DX7-style exponential curve: 0 to 1 range with exponential mapping
            let normalized = i as f32 / 255.0;
            // Exponential curve: e^(ln(0.001) * (1 - x)) gives range from 0.001 to 1.0
            self.exp_table[i] = if normalized == 0.0 {
                0.0
            } else {
                (-6.907755 * (1.0 - normalized)).exp() // ln(0.001) ≈ -6.907755
            };
        }
    }

    // Pre-calculate MIDI note frequencies (A4 = 440Hz)
    fn init_midi_frequencies(&mut self) {
        for midi_note in 0..128 {
            // MIDI note 69 = A4 = 440Hz
            // Formula: f = 440 * 2^((note - 69) / 12)
            let frequency = 440.0 * 2.0_f32.powf((midi_note as f32 - 69.0) / 12.0);
            self.midi_frequencies[midi_note] = frequency;
        }
    }

    // Pre-calculate voice scaling factors for polyphony
    fn init_voice_scale_table(&mut self) {
        self.voice_scale_table[0] = 1.0; // 0 voices = 1.0 (no scaling)
        for i in 1..=16 {
            // DX7-authentic scaling: preserve more headroom for crystalline sound
            let voice_count_f = i as f32;
            self.voice_scale_table[i] = (1.0 / voice_count_f.sqrt()).min(1.0);
        }
    }

    // Authentic DX7 level-to-amplitude table
    // DX7 uses ~0.75 dB per level step, with level 99 = 0 dB (max)
    fn init_dx7_level_table(&mut self) {
        for level in 0..100 {
            if level == 0 {
                self.dx7_level_table[level] = 0.0;
            } else {
                // DX7 formula: each level step = ~0.75 dB
                // amplitude = 10^((level - 99) * 0.75 / 20)
                // Simplified: amplitude = 2^((level - 99) / 8)
                let db = (level as f32 - 99.0) * 0.75;
                self.dx7_level_table[level] = 10.0_f32.powf(db / 20.0);
            }
        }
    }

    // Authentic DX7 rate-to-time table
    // Rate 99 = fastest (~8ms), Rate 0 = slowest (~41 seconds)
    // The relationship is exponential
    fn init_dx7_rate_table(&mut self) {
        for rate in 0..100 {
            if rate == 0 {
                // Rate 0: very slow, approximately 41 seconds for full transition
                self.dx7_rate_table[rate] = 41.0;
            } else {
                // Exponential relationship: rate 99 = ~8ms, rate 0 = ~41s
                // time_ms = 41000 * 2^(-rate * 12 / 99)
                // This gives approximately 12 doublings of speed across 99 levels
                let exponent = -(rate as f32) * 12.0 / 99.0;
                let time_seconds = 41.0 * 2.0_f32.powf(exponent);
                self.dx7_rate_table[rate] = time_seconds.max(0.008); // Min 8ms
            }
        }
    }

    // Optimized sine lookup with cubic interpolation for smoother audio
    pub fn fast_sin(&self, phase: f32) -> f32 {
        // Use multiplication instead of division for better performance
        const INV_TWO_PI: f32 = 1.0 / (2.0 * std::f32::consts::PI);
        let normalized = (phase * INV_TWO_PI).fract();
        let normalized = if normalized < 0.0 {
            normalized + 1.0
        } else {
            normalized
        };

        let index_f = normalized * 4096.0;
        let index = index_f as usize;
        let frac = index_f - index as f32;

        // Get 4 points for cubic interpolation
        let i0 = (index + 4095) & 4095; // index - 1
        let i1 = index & 4095;
        let i2 = (index + 1) & 4095;
        let i3 = (index + 2) & 4095;

        let y0 = self.sine_table[i0];
        let y1 = self.sine_table[i1];
        let y2 = self.sine_table[i2];
        let y3 = self.sine_table[i3];

        // Cubic interpolation (Catmull-Rom spline)
        let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
        let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c = -0.5 * y0 + 0.5 * y2;
        let d = y1;

        ((a * frac + b) * frac + c) * frac + d
    }

    // Fast exponential lookup for envelope values
    pub fn fast_exp(&self, value: f32) -> f32 {
        let clamped = value.clamp(0.0, 1.0);
        let index = (clamped * 255.0) as usize;
        self.exp_table[index.min(255)]
    }

    // Get pre-calculated MIDI frequency
    pub fn get_midi_frequency(&self, midi_note: u8) -> f32 {
        if midi_note < 128 {
            self.midi_frequencies[midi_note as usize]
        } else {
            440.0 // Fallback to A4
        }
    }

    // Convert DX7 level (0-99) to linear amplitude using authentic table
    pub fn dx7_level_to_amplitude(&self, level: u8) -> f32 {
        let idx = (level as usize).min(99);
        self.dx7_level_table[idx]
    }

    // Convert DX7 rate (0-99) to time in seconds using authentic table
    pub fn dx7_rate_to_time(&self, rate: u8) -> f32 {
        let idx = (rate as usize).min(99);
        self.dx7_rate_table[idx]
    }

    // Convert DX7 rate (0-99) to rate multiplier for envelope processing
    // Higher rate = faster envelope = higher multiplier
    pub fn dx7_rate_to_multiplier(&self, rate: u8) -> f32 {
        let time = self.dx7_rate_to_time(rate);
        if time > 0.0 {
            1.0 / time // Convert time to rate: faster time = higher rate
        } else {
            100.0 // Max rate
        }
    }

    // Get pre-calculated voice scaling factor for polyphony
    pub fn get_voice_scale(&self, voice_count: usize) -> f32 {
        if voice_count <= 16 {
            self.voice_scale_table[voice_count]
        } else {
            // Fallback for > 16 voices (shouldn't happen in DX7 emulation)
            (1.0 / (voice_count as f32).sqrt()).min(1.0) * 0.7
        }
    }
}

// Global optimization tables instance
lazy_static::lazy_static! {
    pub static ref OPTIMIZATION_TABLES: OptimizationTables = OptimizationTables::new();
}
