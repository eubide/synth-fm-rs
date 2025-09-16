use crate::algorithms;
use crate::lfo::LFO;
use crate::lock_free::LockFreeSynth;
use crate::operator::Operator;
use crate::optimization::OPTIMIZATION_TABLES;
use std::collections::HashMap;

const MAX_VOICES: usize = 16;

#[derive(Clone)]
pub struct Voice {
    pub operators: [Operator; 6],
    pub note: u8,
    pub frequency: f32,
    pub velocity: f32,
    pub active: bool,
    pub release_time: f32,
    pub current_frequency: f32, // For portamento
    pub target_frequency: f32,  // For portamento
    sample_rate: f32,
    // Voice stealing fade state
    fade_state: VoiceFadeState,
    fade_gain: f32, // 0.0 to 1.0 for fade in/out
    fade_rate: f32, // Rate per sample
}

#[derive(Clone, Debug, PartialEq)]
enum VoiceFadeState {
    Normal,  // Regular playing
    FadeOut, // Being stolen, fading out
    FadeIn,  // New note, fading in
}

impl Voice {
    pub fn new_with_sample_rate(sample_rate: f32) -> Self {
        let mut operators = [
            Operator::new(sample_rate),
            Operator::new(sample_rate),
            Operator::new(sample_rate),
            Operator::new(sample_rate),
            Operator::new(sample_rate),
            Operator::new(sample_rate),
        ];

        // Set up a DX7 E.Piano-like patch (Algorithm 5) with reduced levels
        let ratios = [1.0, 1.0, 7.0, 1.0, 14.0, 1.0]; // Classic E.Piano ratios
        let levels = [75.0, 65.0, 35.0, 45.0, 20.0, 55.0]; // Reduced levels to prevent overload

        for (i, op) in operators.iter_mut().enumerate() {
            op.frequency_ratio = ratios[i];
            op.output_level = levels[i];

            // Add feedback to operator 6 (index 5)
            if i == 5 {
                op.feedback = 3.0; // Moderate feedback for warmth
            }

            // Fast attack envelope
            op.envelope.rate1 = 99.0; // Fast attack
            op.envelope.rate2 = 85.0; // Medium decay
            op.envelope.rate3 = 70.0; // Medium sustain decay
            op.envelope.rate4 = 75.0; // Medium release
            op.envelope.level1 = 99.0; // Full attack level
            op.envelope.level2 = 85.0; // High decay level
            op.envelope.level3 = 60.0; // Medium sustain level
            op.envelope.level4 = 0.0; // Full release
        }

        Self {
            operators,
            note: 0,
            frequency: 0.0,
            velocity: 0.0,
            active: false,
            release_time: 0.0,
            current_frequency: 0.0,
            target_frequency: 0.0,
            sample_rate,
            fade_state: VoiceFadeState::Normal,
            fade_gain: 1.0,
            fade_rate: 0.001, // Fast fade: ~2ms @ 44.1kHz
        }
    }

    pub fn steal_voice(&mut self) {
        self.fade_state = VoiceFadeState::FadeOut;
        self.fade_rate = 1.0 / (self.sample_rate * 0.002); // 2ms fade-out
    }

    pub fn trigger(&mut self, note: u8, velocity: f32, master_tune: f32, portamento_enable: bool) {
        self.note = note;
        // Apply master_tune in cents (±150 cents = ±1.5 semitones)
        // Use optimized pre-calculated MIDI frequencies
        let base_frequency = OPTIMIZATION_TABLES.get_midi_frequency(note);
        let new_frequency = base_frequency * 2.0_f32.powf((master_tune / 100.0) / 12.0);

        // Check if we should use portamento: only if enabled, voice is active, and we have a valid current frequency
        let use_portamento = portamento_enable
            && self.active
            && self.current_frequency > 0.0
            && (self.current_frequency - new_frequency).abs() > 0.1; // Only if frequency actually changed

        // Always update frequencies
        self.frequency = new_frequency;

        if use_portamento {
            // Portamento: smooth transition from current to new frequency
            self.target_frequency = new_frequency;
            // Keep current_frequency for smooth portamento transition
        } else {
            // No portamento: immediate frequency change
            self.current_frequency = new_frequency;
            self.target_frequency = new_frequency;
        }

        self.velocity = velocity;
        self.active = true;

        // Set up graceful fade-in for new notes
        self.fade_state = VoiceFadeState::FadeIn;
        self.fade_gain = 0.0;
        self.fade_rate = 1.0 / (self.sample_rate * 0.005); // 5ms fade-in

        // Always re-trigger operators with new frequency for consistent sound
        for op in &mut self.operators {
            op.trigger(new_frequency, velocity, note);
        }
    }

    pub fn release(&mut self) {
        for op in &mut self.operators {
            op.release();
        }
        self.release_time = 0.0;
    }

    pub fn stop(&mut self) {
        self.active = false;
        for op in &mut self.operators {
            op.reset();
        }
    }

    pub fn process(
        &mut self,
        algorithm_number: u8,
        pitch_bend: f32,
        pitch_bend_range: f32,
        portamento_time: f32,
        lfo_pitch_mod: f32,
        lfo_amp_mod: f32,
    ) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Apply portamento smoothing with DX7-authentic curve
        if self.current_frequency != self.target_frequency {
            // DX7 portamento: 0-99 maps to ~5ms to ~5 seconds
            // Use exponential curve for more natural feel
            let portamento_rate = if portamento_time > 0.0 {
                // Convert 0-99 range to seconds (exponentially)
                let time_seconds = 0.005 + (portamento_time / 99.0).powf(2.0) * 2.0;
                // Convert to rate per sample
                let samples_for_transition = time_seconds * self.sample_rate;
                1.0 / samples_for_transition.max(1.0)
            } else {
                1.0 // Instant change when portamento is 0
            };

            // Exponential glide for more musical portamento
            let freq_ratio = self.target_frequency / self.current_frequency.max(0.001);
            let log_ratio = freq_ratio.ln();
            let step = log_ratio * portamento_rate;
            self.current_frequency *= (1.0 + step).min(2.0).max(0.5); // Limit rate of change

            // Snap to target when very close
            if (self.target_frequency - self.current_frequency).abs() < 0.1 {
                self.current_frequency = self.target_frequency;
            }
        }

        // Apply pitch bend and LFO
        let bend_semitones = pitch_bend * pitch_bend_range;
        let bent_frequency = self.current_frequency * 2.0_f32.powf(bend_semitones / 12.0);
        let lfo_pitch_semitones = lfo_pitch_mod * 0.5;
        let lfo_pitch_factor = 2.0_f32.powf(lfo_pitch_semitones / 12.0);
        let final_frequency = bent_frequency * lfo_pitch_factor;

        // Update operator frequencies without resetting phase
        for op in &mut self.operators {
            op.update_frequency_only(final_frequency);
        }

        // Process using direct hardcoded algorithms
        let output = algorithms::process_algorithm(algorithm_number, &mut self.operators);

        // Apply LFO amplitude modulation
        let lfo_amp_factor = 1.0 + (lfo_amp_mod * 0.5);
        let modulated_output = output * lfo_amp_factor;

        // Check if voice is still active
        let all_inactive = self.operators.iter().all(|op| !op.is_active());
        if all_inactive && self.fade_state != VoiceFadeState::FadeOut {
            self.active = false;
        }

        // Handle voice fade states (same as original)
        let final_output = match self.fade_state {
            VoiceFadeState::FadeIn => {
                self.fade_gain += self.fade_rate;
                if self.fade_gain >= 1.0 {
                    self.fade_gain = 1.0;
                    self.fade_state = VoiceFadeState::Normal;
                }
                modulated_output * self.fade_gain
            }
            VoiceFadeState::FadeOut => {
                self.fade_gain -= self.fade_rate;
                if self.fade_gain <= 0.0 {
                    self.fade_gain = 0.0;
                    self.active = false;
                }
                modulated_output * self.fade_gain
            }
            VoiceFadeState::Normal => modulated_output,
        };

        final_output * 0.8
    }
}

pub struct FmSynthesizer {
    pub voices: Vec<Voice>,
    pub held_notes: HashMap<u8, usize>,
    pub preset_name: String,
    pub lfo: LFO,
    pub lock_free_params: LockFreeSynth,
}

impl FmSynthesizer {
    pub fn new_with_sample_rate(sample_rate: f32) -> Self {
        let mut voices = Vec::with_capacity(MAX_VOICES);
        for _ in 0..MAX_VOICES {
            // Create voices with actual sample rate instead of hardcoded
            let voice = Voice::new_with_sample_rate(sample_rate);
            voices.push(voice);
        }

        let lock_free_params = LockFreeSynth::new();

        Self {
            voices,
            held_notes: HashMap::new(),
            preset_name: "Init Voice".to_string(),
            lfo: LFO::new(sample_rate),
            lock_free_params,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        let velocity_f = velocity as f32 / 127.0;
        let params = self.lock_free_params.get_global_params();

        // Trigger LFO if key sync is enabled
        self.lfo.trigger();

        if params.mono_mode {
            // Mono mode: use portamento for smooth transition or immediate change
            self.held_notes.clear();

            // In mono mode, always trigger the first voice
            // Portamento will be handled inside trigger() based on portamento settings
            self.voices[0].trigger(
                note,
                velocity_f,
                params.master_tune,
                params.portamento_enable,
            );
            self.held_notes.insert(note, 0);
        } else {
            // Poly mode: original logic
            if let Some(&voice_idx) = self.held_notes.get(&note) {
                self.voices[voice_idx].trigger(
                    note,
                    velocity_f,
                    params.master_tune,
                    params.portamento_enable && params.mono_mode,
                );
                return;
            }

            for (i, voice) in self.voices.iter_mut().enumerate() {
                if !voice.active {
                    voice.trigger(note, velocity_f, params.master_tune, false); // No portamento in poly mode
                    self.held_notes.insert(note, i);
                    return;
                }
            }

            let oldest_voice = self
                .voices
                .iter()
                .enumerate()
                .max_by(|a, b| {
                    a.1.release_time
                        .partial_cmp(&b.1.release_time)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0);

            // Voice stealing with fade-out
            self.voices[oldest_voice].steal_voice();
            self.voices[oldest_voice].trigger(note, velocity_f, params.master_tune, false);

            self.held_notes.retain(|_, &mut v| v != oldest_voice);
            self.held_notes.insert(note, oldest_voice);
        }
    }

    pub fn note_off(&mut self, note: u8) {
        // Debug: println!("Note OFF: {}", note);
        if let Some(&voice_idx) = self.held_notes.get(&note) {
            if !self.lock_free_params.get_sustain_pedal() {
                self.voices[voice_idx].release();
                self.held_notes.remove(&note);
            }
        }
    }

    pub fn process(&mut self) -> f32 {
        let mut output = 0.0;
        let mut active_voice_count = 0;

        // Get lock-free parameters (real-time safe)
        let params = self.lock_free_params.get_global_params();

        // Check for panic request
        if self.lock_free_params.check_panic_request() {
            for voice in &mut self.voices {
                voice.stop();
            }
            self.held_notes.clear();
            return 0.0;
        }

        // Generate global LFO modulation values
        let (lfo_pitch_mod, lfo_amp_mod) = self.lfo.process(params.mod_wheel);

        // Process voices using direct hardcoded algorithms
        for voice in &mut self.voices {
            if voice.active {
                let voice_output = voice.process(
                    params.algorithm,
                    params.pitch_bend,
                    params.pitch_bend_range,
                    params.portamento_time,
                    lfo_pitch_mod,
                    lfo_amp_mod,
                );
                output += voice_output;
                active_voice_count += 1;
            }
        }

        // Apply DX7-authentic polyphonic scaling to preserve clarity
        let voice_scaling = if active_voice_count > 0 {
            // More aggressive scaling like the original DX7 to prevent muddiness
            // DX7 had significant headroom and clear voice separation
            // Use pre-computed voice scaling table for better performance
            OPTIMIZATION_TABLES.get_voice_scale(active_voice_count)
        } else {
            1.0
        };

        let scaled_output = output * voice_scaling * params.master_volume;

        // Apply final soft limiting
        self.soft_limit(scaled_output)
    }

    pub fn set_algorithm(&mut self, algorithm: u8) {
        if (1..=32).contains(&algorithm) {
            // Update lock-free parameters
            let mut params = self.lock_free_params.get_global_params().clone();
            params.algorithm = algorithm;
            self.lock_free_params.set_global_param(params);
        }
    }

    pub fn set_operator_param(&mut self, op_index: usize, param: &str, value: f32) {
        if op_index >= 6 {
            return;
        }

        for voice in &mut self.voices {
            match param {
                "ratio" => voice.operators[op_index].set_frequency_ratio(value),
                "level" => voice.operators[op_index].output_level = value,
                "detune" => voice.operators[op_index].set_detune(value),
                "feedback" => voice.operators[op_index].feedback = value,
                "vel_sens" => voice.operators[op_index].velocity_sensitivity = value,
                "key_scale_level" => voice.operators[op_index].key_scale_level = value,
                "key_scale_rate" => voice.operators[op_index].key_scale_rate = value,
                _ => {}
            }
        }
    }

    pub fn set_envelope_param(&mut self, op_index: usize, param: &str, value: f32) {
        if op_index >= 6 {
            return;
        }

        for voice in &mut self.voices {
            match param {
                "rate1" => voice.operators[op_index].envelope.rate1 = value,
                "rate2" => voice.operators[op_index].envelope.rate2 = value,
                "rate3" => voice.operators[op_index].envelope.rate3 = value,
                "rate4" => voice.operators[op_index].envelope.rate4 = value,
                "level1" => voice.operators[op_index].envelope.level1 = value,
                "level2" => voice.operators[op_index].envelope.level2 = value,
                "level3" => voice.operators[op_index].envelope.level3 = value,
                "level4" => voice.operators[op_index].envelope.level4 = value,
                _ => {}
            }
        }
    }

    pub fn panic(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
            for op in &mut voice.operators {
                op.reset();
            }
        }
        self.held_notes.clear();
    }

    pub fn control_change(&mut self, controller: u8, value: u8) {
        match controller {
            1 => {
                let mut params = self.lock_free_params.get_global_params().clone();
                params.mod_wheel = value as f32 / 127.0;
                self.lock_free_params.set_global_param(params);
            }
            64 => {
                self.lock_free_params.set_sustain_pedal(value >= 64);
            }
            123 => {
                self.lock_free_params.request_panic();
            }
            _ => {}
        }
    }

    pub fn pitch_bend(&mut self, value: i16) {
        let mut params = self.lock_free_params.get_global_params().clone();
        params.pitch_bend = value as f32 / 8192.0;
        self.lock_free_params.set_global_param(params);
    }

    pub fn set_master_tune(&mut self, cents: f32) {
        let mut params = self.lock_free_params.get_global_params().clone();
        params.master_tune = cents.clamp(-150.0, 150.0);
        self.lock_free_params.set_global_param(params);
    }

    pub fn set_mono_mode(&mut self, mono: bool) {
        let mut params = self.lock_free_params.get_global_params().clone();
        params.mono_mode = mono;
        self.lock_free_params.set_global_param(params);

        // If switching to mono, stop all voices except the first active one
        if mono {
            let mut first_active_found = false;
            for voice in &mut self.voices {
                if voice.active {
                    if first_active_found {
                        voice.stop();
                    } else {
                        first_active_found = true;
                    }
                }
            }
        }
    }

    pub fn set_pitch_bend_range(&mut self, range: f32) {
        let mut params = self.lock_free_params.get_global_params().clone();
        params.pitch_bend_range = range.clamp(0.0, 12.0);
        self.lock_free_params.set_global_param(params);
    }

    pub fn set_portamento_enable(&mut self, enable: bool) {
        let mut params = self.lock_free_params.get_global_params().clone();
        params.portamento_enable = enable;
        self.lock_free_params.set_global_param(params);
    }

    pub fn set_portamento_time(&mut self, time: f32) {
        let mut params = self.lock_free_params.get_global_params().clone();
        params.portamento_time = time.clamp(0.0, 99.0);
        self.lock_free_params.set_global_param(params);
    }

    pub fn voice_initialize(&mut self) {
        // Reset all voices to basic DX7 init voice settings
        self.preset_name = "Init Voice".to_string();

        // Set algorithm to 1 using lock-free parameters
        let mut params = self.lock_free_params.get_global_params().clone();
        params.algorithm = 1; // Basic algorithm
        self.lock_free_params.set_global_param(params);

        // Stop all playing voices
        for voice in &mut self.voices {
            voice.stop();
        }
        self.held_notes.clear();

        // Initialize all voice operators to basic settings
        for voice in &mut self.voices {
            for (i, op) in voice.operators.iter_mut().enumerate() {
                // Basic operator settings
                op.frequency_ratio = 1.0;
                op.output_level = if i == 0 { 99.0 } else { 0.0 }; // Only OP1 audible
                op.detune = 0.0;
                op.feedback = 0.0;
                op.velocity_sensitivity = 0.0;
                op.key_scale_level = 0.0;
                op.key_scale_rate = 0.0;

                // Basic envelope (fast attack, sustain)
                op.envelope.rate1 = 95.0; // Attack
                op.envelope.rate2 = 25.0; // Decay
                op.envelope.rate3 = 25.0; // Sustain
                op.envelope.rate4 = 67.0; // Release
                op.envelope.level1 = 99.0; // Attack level
                op.envelope.level2 = 75.0; // Decay level
                op.envelope.level3 = 50.0; // Sustain level (must be > 0 for sustained sound)
                op.envelope.level4 = 0.0; // Release level
            }
        }
    }

    // LFO control methods
    pub fn set_lfo_rate(&mut self, rate: f32) {
        self.lfo.set_rate(rate);
    }

    pub fn set_lfo_delay(&mut self, delay: f32) {
        self.lfo.set_delay(delay);
    }

    pub fn set_lfo_pitch_depth(&mut self, depth: f32) {
        self.lfo.set_pitch_depth(depth);
    }

    pub fn set_lfo_amp_depth(&mut self, depth: f32) {
        self.lfo.set_amp_depth(depth);
    }

    pub fn set_lfo_waveform(&mut self, waveform: crate::lfo::LFOWaveform) {
        self.lfo.set_waveform(waveform);
    }

    pub fn set_lfo_key_sync(&mut self, key_sync: bool) {
        self.lfo.set_key_sync(key_sync);
    }

    // LFO getters for GUI display
    pub fn get_lfo_rate(&self) -> f32 {
        self.lfo.rate
    }

    pub fn get_lfo_delay(&self) -> f32 {
        self.lfo.delay
    }

    pub fn get_lfo_pitch_depth(&self) -> f32 {
        self.lfo.pitch_depth
    }

    pub fn get_lfo_amp_depth(&self) -> f32 {
        self.lfo.amp_depth
    }

    pub fn get_lfo_waveform(&self) -> crate::lfo::LFOWaveform {
        self.lfo.waveform
    }

    pub fn get_lfo_key_sync(&self) -> bool {
        self.lfo.key_sync
    }

    pub fn get_lfo_frequency_hz(&self) -> f32 {
        self.lfo.get_frequency_hz()
    }

    pub fn get_lfo_delay_seconds(&self) -> f32 {
        self.lfo.get_delay_seconds()
    }

    // Lock-free parameter getters for GUI
    pub fn get_algorithm(&self) -> u8 {
        self.lock_free_params.get_global_params().algorithm
    }

    pub fn get_master_volume(&self) -> f32 {
        self.lock_free_params.get_global_params().master_volume
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        let mut params = self.lock_free_params.get_global_params().clone();
        params.master_volume = volume.clamp(0.0, 1.0);
        self.lock_free_params.set_global_param(params);
    }

    pub fn get_mod_wheel(&self) -> f32 {
        self.lock_free_params.get_global_params().mod_wheel
    }

    pub fn get_master_tune(&self) -> f32 {
        self.lock_free_params.get_global_params().master_tune
    }

    pub fn get_mono_mode(&self) -> bool {
        self.lock_free_params.get_global_params().mono_mode
    }

    pub fn get_pitch_bend_range(&self) -> f32 {
        self.lock_free_params.get_global_params().pitch_bend_range
    }

    pub fn get_portamento_enable(&self) -> bool {
        self.lock_free_params.get_global_params().portamento_enable
    }

    pub fn get_portamento_time(&self) -> f32 {
        self.lock_free_params.get_global_params().portamento_time
    }

    /// Soft limiting for final output
    fn soft_limit(&self, sample: f32) -> f32 {
        const THRESHOLD: f32 = 0.7; // Lower threshold for synth output
        const KNEE: f32 = 0.15; // Gentler knee

        if sample.abs() <= THRESHOLD {
            sample
        } else {
            let sign = sample.signum();
            let abs_sample = sample.abs();

            // Smooth compression above threshold
            let excess = abs_sample - THRESHOLD;
            let compressed_excess = excess / (1.0 + excess / KNEE);
            let limited = THRESHOLD + compressed_excess;

            // Final hard limit with more headroom
            sign * limited.min(0.85)
        }
    }
}
