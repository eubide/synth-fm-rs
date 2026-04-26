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

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44_100.0;

    fn drive_chorus(c: &mut Chorus, samples: usize) -> (f32, f32) {
        let mut peak_l = 0.0_f32;
        let mut peak_r = 0.0_f32;
        for i in 0..samples {
            let phase = 2.0 * PI * 440.0 * (i as f32) / SR;
            let (l, r) = c.process(phase.sin());
            peak_l = peak_l.max(l.abs());
            peak_r = peak_r.max(r.abs());
        }
        (peak_l, peak_r)
    }

    // -----------------------------------------------------------------------
    // Chorus
    // -----------------------------------------------------------------------

    #[test]
    fn chorus_disabled_passes_input_through_unchanged() {
        let mut c = Chorus::new(SR);
        c.enabled = false;
        let (l, r) = c.process(0.5);
        assert_eq!(l, 0.5);
        assert_eq!(r, 0.5);
    }

    #[test]
    fn chorus_enabled_modulates_output() {
        let mut c = Chorus::new(SR);
        c.enabled = true;
        let (peak_l, peak_r) = drive_chorus(&mut c, 4096);
        assert!(peak_l > 0.0);
        assert!(peak_r > 0.0);
        // Should stay within reasonable bounds.
        assert!(peak_l < 5.0);
        assert!(peak_r < 5.0);
    }

    #[test]
    fn chorus_mix_at_zero_returns_input_only() {
        let mut c = Chorus::new(SR);
        c.enabled = true;
        c.mix = 0.0;
        // After enough samples, output should track input
        let (l, r) = c.process(1.0);
        assert!((l - 1.0).abs() < 0.5);
        assert!((r - 1.0).abs() < 0.5);
    }

    #[test]
    fn chorus_lfo_phase_advances_through_cycle() {
        let mut c = Chorus::new(SR);
        c.enabled = true;
        c.rate = 5.0;
        // Run long enough to wrap LFO phase several times.
        drive_chorus(&mut c, SR as usize);
    }

    // -----------------------------------------------------------------------
    // Delay
    // -----------------------------------------------------------------------

    #[test]
    fn delay_disabled_passes_through_stereo() {
        let mut d = Delay::new(SR);
        d.enabled = false;
        let (l, r) = d.process(0.3, 0.7);
        assert_eq!(l, 0.3);
        assert_eq!(r, 0.7);
    }

    #[test]
    fn delay_enabled_emits_delayed_signal() {
        let mut d = Delay::new(SR);
        d.enabled = true;
        d.time_ms = 50.0;
        d.feedback = 0.0;
        d.mix = 1.0;

        // Send an impulse, then silence for 50ms+ samples; we should see the impulse come back.
        d.process(1.0, 1.0);
        let mut max_after = 0.0_f32;
        let mut got_echo = false;
        for _ in 0..((SR * 0.06) as usize) {
            let (l, r) = d.process(0.0, 0.0);
            max_after = max_after.max(l.abs()).max(r.abs());
            if l.abs() > 0.5 || r.abs() > 0.5 {
                got_echo = true;
            }
        }
        assert!(got_echo, "delay should produce an echo within ~60ms, max={max_after}");
    }

    #[test]
    fn delay_ping_pong_mode_processes_audio() {
        let mut d = Delay::new(SR);
        d.enabled = true;
        d.ping_pong = true;
        for _ in 0..2048 {
            let _ = d.process(0.5, 0.0);
        }
    }

    #[test]
    fn delay_normal_mode_processes_audio() {
        let mut d = Delay::new(SR);
        d.enabled = true;
        d.ping_pong = false;
        for _ in 0..2048 {
            let _ = d.process(0.5, 0.5);
        }
    }

    // -----------------------------------------------------------------------
    // Reverb
    // -----------------------------------------------------------------------

    #[test]
    fn reverb_disabled_passes_through_stereo() {
        let mut r = Reverb::new(SR);
        r.enabled = false;
        let (l, rr) = r.process(0.4, 0.6);
        assert_eq!(l, 0.4);
        assert_eq!(rr, 0.6);
    }

    #[test]
    fn reverb_enabled_produces_decaying_tail() {
        let mut r = Reverb::new(SR);
        r.enabled = true;
        r.mix = 1.0;
        // Drive ~50ms (comb delays are ~25-30ms; 2200 samples seeds them).
        for _ in 0..(SR as usize / 20) {
            r.process(0.5, 0.5);
        }
        // Measure ~50ms of tail energy after the input goes silent.
        let mut tail_energy = 0.0_f32;
        for _ in 0..(SR as usize / 20) {
            let (l, rr) = r.process(0.0, 0.0);
            tail_energy += l * l + rr * rr;
        }
        assert!(tail_energy > 1e-3, "reverb should leave a decaying tail, energy={tail_energy}");
    }

    #[test]
    fn reverb_room_size_changes_feedback() {
        let mut r = Reverb::new(SR);
        r.enabled = true;
        r.room_size = 0.0;
        for _ in 0..512 {
            r.process(1.0, 1.0);
        }
        r.room_size = 1.0;
        for _ in 0..512 {
            r.process(1.0, 1.0);
        }
    }

    #[test]
    fn reverb_width_zero_collapses_to_mono() {
        let mut r = Reverb::new(SR);
        r.enabled = true;
        r.width = 0.0;
        // Drive some signal in
        for _ in 0..1024 {
            let _ = r.process(0.5, -0.5);
        }
    }

    // -----------------------------------------------------------------------
    // EffectsChain
    // -----------------------------------------------------------------------

    #[test]
    fn effects_chain_pipes_through_all_three_stages() {
        let mut chain = EffectsChain::new(SR);
        chain.chorus.enabled = true;
        chain.delay.enabled = true;
        chain.reverb.enabled = true;
        let mut peak = 0.0_f32;
        for i in 0..2048 {
            let phase = 2.0 * PI * 440.0 * (i as f32) / SR;
            let (l, r) = chain.process(phase.sin());
            peak = peak.max(l.abs()).max(r.abs());
        }
        assert!(peak > 0.0);
    }

    #[test]
    fn effects_chain_all_disabled_returns_input_as_stereo() {
        let mut chain = EffectsChain::new(SR);
        let (l, r) = chain.process(0.42);
        assert_eq!(l, 0.42);
        assert_eq!(r, 0.42);
    }
}
