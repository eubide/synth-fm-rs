use std::f32::consts::PI;

const MAX_DELAY_SAMPLES: usize = 88200; // 2 seconds at 44.1kHz

// ============================================================================
// CHORUS EFFECT
// ============================================================================

pub struct Chorus {
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos: usize,
    lfo_phase: f32,
    sample_rate: f32,

    // Parameters
    pub enabled: bool,
    pub rate: f32,     // LFO rate in Hz (0.1 - 5.0)
    pub depth: f32,    // Modulation depth in ms (0.0 - 10.0)
    pub mix: f32,      // Wet/dry mix (0.0 - 1.0)
    pub feedback: f32, // Feedback amount (0.0 - 0.7)
}

impl Chorus {
    pub fn new(sample_rate: f32) -> Self {
        let buffer_size = (sample_rate * 0.05) as usize; // 50ms buffer
        Self {
            buffer_l: vec![0.0; buffer_size],
            buffer_r: vec![0.0; buffer_size],
            write_pos: 0,
            lfo_phase: 0.0,
            sample_rate,
            enabled: false,
            rate: 1.5,
            depth: 3.0,
            mix: 0.5,
            feedback: 0.2,
        }
    }

    pub fn process(&mut self, input: f32) -> (f32, f32) {
        if !self.enabled {
            return (input, input);
        }

        let buffer_size = self.buffer_l.len();

        // LFO for modulation (sine wave)
        let lfo_l = (self.lfo_phase * 2.0 * PI).sin();
        let lfo_r = ((self.lfo_phase + 0.25) * 2.0 * PI).sin(); // 90 degree offset for stereo

        // Calculate delay times in samples (keep as float for interpolation)
        let base_delay_ms = 7.0; // Base delay
        let delay_l_ms = base_delay_ms + self.depth * lfo_l;
        let delay_r_ms = base_delay_ms + self.depth * lfo_r;

        let delay_l_samples = delay_l_ms * self.sample_rate / 1000.0;
        let delay_r_samples = delay_r_ms * self.sample_rate / 1000.0;

        // Read with linear interpolation (eliminates zipper noise)
        let delayed_l = self.read_interpolated(&self.buffer_l, delay_l_samples, buffer_size);
        let delayed_r = self.read_interpolated(&self.buffer_r, delay_r_samples, buffer_size);

        // Write to buffers with feedback
        self.buffer_l[self.write_pos] = input + delayed_l * self.feedback;
        self.buffer_r[self.write_pos] = input + delayed_r * self.feedback;

        // Advance write position
        self.write_pos = (self.write_pos + 1) % buffer_size;

        // Advance LFO
        self.lfo_phase += self.rate / self.sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        // Mix dry and wet
        let out_l = input * (1.0 - self.mix) + delayed_l * self.mix;
        let out_r = input * (1.0 - self.mix) + delayed_r * self.mix;

        (out_l, out_r)
    }

    /// Read from delay buffer with linear interpolation for smooth modulation
    fn read_interpolated(&self, buffer: &[f32], delay_samples: f32, buffer_size: usize) -> f32 {
        let delay_clamped = delay_samples.clamp(1.0, (buffer_size - 2) as f32);

        let delay_int = delay_clamped as usize;
        let frac = delay_clamped - delay_int as f32;

        let read_pos_0 = (self.write_pos + buffer_size - delay_int) % buffer_size;
        let read_pos_1 = (self.write_pos + buffer_size - delay_int - 1) % buffer_size;

        let sample_0 = buffer[read_pos_0];
        let sample_1 = buffer[read_pos_1];

        // Linear interpolation
        sample_0 + frac * (sample_1 - sample_0)
    }
}

// ============================================================================
// DELAY EFFECT
// ============================================================================

pub struct Delay {
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos: usize,
    sample_rate: f32,

    // Parameters
    pub enabled: bool,
    pub time_ms: f32,    // Delay time in ms (0 - 1000)
    pub feedback: f32,   // Feedback amount (0.0 - 0.9)
    pub mix: f32,        // Wet/dry mix (0.0 - 1.0)
    pub ping_pong: bool, // Ping-pong stereo mode
}

impl Delay {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buffer_l: vec![0.0; MAX_DELAY_SAMPLES],
            buffer_r: vec![0.0; MAX_DELAY_SAMPLES],
            write_pos: 0,
            sample_rate,
            enabled: false,
            time_ms: 300.0,
            feedback: 0.4,
            mix: 0.3,
            ping_pong: true,
        }
    }

    pub fn process(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        if !self.enabled {
            return (input_l, input_r);
        }

        let delay_samples =
            ((self.time_ms * self.sample_rate / 1000.0) as usize).min(MAX_DELAY_SAMPLES - 1);
        let read_pos = (self.write_pos + MAX_DELAY_SAMPLES - delay_samples) % MAX_DELAY_SAMPLES;

        let delayed_l = self.buffer_l[read_pos];
        let delayed_r = self.buffer_r[read_pos];

        // Write to buffers
        if self.ping_pong {
            // Ping-pong: left feeds right, right feeds left
            self.buffer_l[self.write_pos] = input_l + delayed_r * self.feedback;
            self.buffer_r[self.write_pos] = input_r + delayed_l * self.feedback;
        } else {
            // Normal stereo delay
            self.buffer_l[self.write_pos] = input_l + delayed_l * self.feedback;
            self.buffer_r[self.write_pos] = input_r + delayed_r * self.feedback;
        }

        self.write_pos = (self.write_pos + 1) % MAX_DELAY_SAMPLES;

        // Mix
        let out_l = input_l * (1.0 - self.mix) + delayed_l * self.mix;
        let out_r = input_r * (1.0 - self.mix) + delayed_r * self.mix;

        (out_l, out_r)
    }
}

// ============================================================================
// REVERB EFFECT (Schroeder-style)
// ============================================================================

struct CombFilter {
    buffer: Vec<f32>,
    write_pos: usize,
    feedback: f32,
    damp: f32,
    damp_state: f32,
}

impl CombFilter {
    fn new(size: usize, feedback: f32, damp: f32) -> Self {
        Self {
            buffer: vec![0.0; size],
            write_pos: 0,
            feedback,
            damp,
            damp_state: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.write_pos];

        // Low-pass filter in feedback loop for natural decay
        self.damp_state = output * (1.0 - self.damp) + self.damp_state * self.damp;

        self.buffer[self.write_pos] = input + self.damp_state * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();

        output
    }
}

struct AllPassFilter {
    buffer: Vec<f32>,
    write_pos: usize,
    feedback: f32,
}

impl AllPassFilter {
    fn new(size: usize, feedback: f32) -> Self {
        Self {
            buffer: vec![0.0; size],
            write_pos: 0,
            feedback,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.write_pos];
        let output = -input + delayed;

        self.buffer[self.write_pos] = input + delayed * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();

        output
    }
}

pub struct Reverb {
    // 8 parallel comb filters (4 per channel for stereo)
    combs_l: [CombFilter; 4],
    combs_r: [CombFilter; 4],
    // 2 series allpass filters per channel
    allpasses_l: [AllPassFilter; 2],
    allpasses_r: [AllPassFilter; 2],

    // Parameters
    pub enabled: bool,
    pub room_size: f32, // 0.0 - 1.0
    pub damping: f32,   // 0.0 - 1.0
    pub mix: f32,       // Wet/dry mix (0.0 - 1.0)
    pub width: f32,     // Stereo width (0.0 - 1.0)
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        // Comb filter delay times (in samples at 44.1kHz, scaled for actual sample rate)
        let scale = sample_rate / 44100.0;
        let comb_sizes_l: [usize; 4] = [
            (1116.0 * scale) as usize,
            (1188.0 * scale) as usize,
            (1277.0 * scale) as usize,
            (1356.0 * scale) as usize,
        ];
        let comb_sizes_r: [usize; 4] = [
            (1139.0 * scale) as usize, // Slightly different for stereo
            (1211.0 * scale) as usize,
            (1300.0 * scale) as usize,
            (1379.0 * scale) as usize,
        ];

        // Allpass filter delay times
        let allpass_sizes: [usize; 2] = [(556.0 * scale) as usize, (441.0 * scale) as usize];

        let feedback = 0.84;
        let damp = 0.2;
        let allpass_feedback = 0.5;

        Self {
            combs_l: [
                CombFilter::new(comb_sizes_l[0], feedback, damp),
                CombFilter::new(comb_sizes_l[1], feedback, damp),
                CombFilter::new(comb_sizes_l[2], feedback, damp),
                CombFilter::new(comb_sizes_l[3], feedback, damp),
            ],
            combs_r: [
                CombFilter::new(comb_sizes_r[0], feedback, damp),
                CombFilter::new(comb_sizes_r[1], feedback, damp),
                CombFilter::new(comb_sizes_r[2], feedback, damp),
                CombFilter::new(comb_sizes_r[3], feedback, damp),
            ],
            allpasses_l: [
                AllPassFilter::new(allpass_sizes[0], allpass_feedback),
                AllPassFilter::new(allpass_sizes[1], allpass_feedback),
            ],
            allpasses_r: [
                AllPassFilter::new(allpass_sizes[0] + 23, allpass_feedback),
                AllPassFilter::new(allpass_sizes[1] + 17, allpass_feedback),
            ],
            enabled: false,
            room_size: 0.7,
            damping: 0.5,
            mix: 0.25,
            width: 1.0,
        }
    }

    pub fn process(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        if !self.enabled {
            return (input_l, input_r);
        }

        // Update comb filter parameters based on room size and damping
        let feedback = 0.7 + self.room_size * 0.28; // 0.7 to 0.98
        let damp = self.damping * 0.4; // 0 to 0.4

        // Process through parallel comb filters
        let input_mono = (input_l + input_r) * 0.5;
        let mut wet_l = 0.0;
        let mut wet_r = 0.0;

        for comb in &mut self.combs_l {
            comb.feedback = feedback;
            comb.damp = damp;
            wet_l += comb.process(input_mono);
        }

        for comb in &mut self.combs_r {
            comb.feedback = feedback;
            comb.damp = damp;
            wet_r += comb.process(input_mono);
        }

        // Scale comb output
        wet_l *= 0.25;
        wet_r *= 0.25;

        // Process through series allpass filters
        for allpass in &mut self.allpasses_l {
            wet_l = allpass.process(wet_l);
        }
        for allpass in &mut self.allpasses_r {
            wet_r = allpass.process(wet_r);
        }

        // Apply stereo width
        let wet_mono = (wet_l + wet_r) * 0.5;
        wet_l = wet_mono + (wet_l - wet_mono) * self.width;
        wet_r = wet_mono + (wet_r - wet_mono) * self.width;

        // Mix dry and wet
        let out_l = input_l * (1.0 - self.mix) + wet_l * self.mix;
        let out_r = input_r * (1.0 - self.mix) + wet_r * self.mix;

        (out_l, out_r)
    }
}

// ============================================================================
// EFFECTS CHAIN
// ============================================================================

pub struct EffectsChain {
    pub chorus: Chorus,
    pub delay: Delay,
    pub reverb: Reverb,
}

impl EffectsChain {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            chorus: Chorus::new(sample_rate),
            delay: Delay::new(sample_rate),
            reverb: Reverb::new(sample_rate),
        }
    }

    pub fn process(&mut self, input: f32) -> (f32, f32) {
        // Chorus first (mono to stereo)
        let (l, r) = self.chorus.process(input);

        // Then delay (stereo)
        let (l, r) = self.delay.process(l, r);

        // Finally reverb (stereo)
        self.reverb.process(l, r)
    }
}
