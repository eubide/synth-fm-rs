use std::f32::consts::PI;

// DX7-style lookup tables for performance optimization

pub struct OptimizationTables {
    sine_table: [f32; 4096],
    exp_table: [f32; 256],
    midi_frequencies: [f32; 128],
}

impl OptimizationTables {
    pub fn new() -> Self {
        let mut tables = OptimizationTables {
            sine_table: [0.0; 4096],
            exp_table: [0.0; 256],
            midi_frequencies: [0.0; 128],
        };
        
        tables.init_sine_table();
        tables.init_exp_table();
        tables.init_midi_frequencies();
        
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
    
    // Fast sine lookup with linear interpolation for better accuracy
    pub fn fast_sin(&self, phase: f32) -> f32 {
        let normalized = (phase / (2.0 * PI)).fract();
        let normalized = if normalized < 0.0 { normalized + 1.0 } else { normalized };
        
        let index_f = normalized * 4096.0;
        let index = index_f as usize;
        let frac = index_f - index as f32;
        
        let val0 = self.sine_table[index & 4095];
        let val1 = self.sine_table[(index + 1) & 4095];
        
        // Linear interpolation for smoother result
        val0 + frac * (val1 - val0)
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
    
    // Convert DX7 level (0-99) to linear amplitude with exponential curve
    pub fn dx7_level_to_amplitude(&self, level: u8) -> f32 {
        if level == 0 {
            0.0
        } else {
            // DX7 uses exponential level scaling
            let normalized = level as f32 / 99.0;
            self.fast_exp(normalized)
        }
    }
    
    // Convert DX7 rate (0-99) to time multiplier
    pub fn dx7_rate_to_multiplier(&self, rate: u8) -> f32 {
        if rate == 0 {
            0.0001 // Very slow
        } else {
            // Exponential rate scaling: higher rate = faster envelope
            let normalized = rate as f32 / 99.0;
            0.001 + self.fast_exp(normalized) * 10.0 // 0.001 to ~10.0 range
        }
    }
}

// Global optimization tables instance
lazy_static::lazy_static! {
    pub static ref OPTIMIZATION_TABLES: OptimizationTables = OptimizationTables::new();
}