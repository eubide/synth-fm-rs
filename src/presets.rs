use crate::fm_synth::SynthEngine;

#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub struct Dx7Preset {
    pub name: &'static str,
    pub algorithm: u8,
    pub operators: [(f32, f32, f32, f32); 6], // (ratio, level, detune, feedback)
    pub envelopes: [(f32, f32, f32, f32, f32, f32, f32, f32); 6], // (r1-r4, l1-l4)
    // Function Mode parameters (reserved for future use)
    pub master_tune: Option<f32>,        // ±150 cents
    pub mono_mode: Option<bool>,         // false = poly, true = mono
    pub pitch_bend_range: Option<f32>,   // 0-12 semitones
    pub portamento_enable: Option<bool>, // portamento on/off
    pub portamento_time: Option<f32>,    // 0-99
}

impl Dx7Preset {
    pub fn apply_to_synth(&self, synth: &mut SynthEngine) {
        synth.set_algorithm(self.algorithm);
        synth.set_preset_name(self.name.to_string());

        // Apply operator settings to all voices
        for voice in synth.voices_mut() {
            for (i, op) in voice.operators.iter_mut().enumerate() {
                let (ratio, level, detune, feedback) = self.operators[i];
                op.frequency_ratio = ratio;
                op.output_level = level;
                op.detune = detune;
                op.feedback = feedback;

                // Apply envelope
                let (r1, r2, r3, r4, l1, l2, l3, l4) = self.envelopes[i];
                op.envelope.rate1 = r1;
                op.envelope.rate2 = r2;
                op.envelope.rate3 = r3;
                op.envelope.rate4 = r4;
                op.envelope.level1 = l1;
                op.envelope.level2 = l2;
                op.envelope.level3 = l3;
                op.envelope.level4 = l4;
            }
        }
    }
}

pub fn get_dx7_presets() -> Vec<Dx7Preset> {
    vec![
        // E.PIANO 1 - Authentic DX7 Classic Electric Piano
        Dx7Preset {
            name: "E.PIANO 1",
            algorithm: 5,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Carrier - fundamental (max level)
                (1.0, 99.0, 0.0, 0.0),  // Op2: Modulator -> Op1 (brightness)
                (1.0, 99.0, 0.0, 0.0),  // Op3: Carrier - body
                (1.0, 99.0, 0.0, 0.0),  // Op4: Carrier - depth
                (14.0, 50.0, 0.0, 0.0), // Op5: High harmonic (bell tone, authentic DX7 ratio)
                (1.0, 80.0, 0.0, 7.0),  // Op6: Modulator -> Op2 + max feedback (brilliance)
            ],
            envelopes: [
                (99.0, 83.0, 83.0, 45.0, 99.0, 85.0, 0.0, 0.0), // Op1: Instant attack, no sustain
                (99.0, 83.0, 83.0, 45.0, 99.0, 85.0, 0.0, 0.0), // Op2: Instant attack, no sustain
                (99.0, 80.0, 80.0, 45.0, 99.0, 87.0, 0.0, 0.0), // Op3: Instant attack, no sustain
                (99.0, 75.0, 75.0, 52.0, 99.0, 92.0, 0.0, 0.0), // Op4: Instant attack, no sustain
                (99.0, 95.0, 95.0, 95.0, 99.0, 75.0, 0.0, 0.0), // Op5: Bell harmonic, fast decay
                (99.0, 85.0, 85.0, 45.0, 99.0, 90.0, 0.0, 0.0), // Op6: Modulator brightness
            ],
            master_tune: None,
            mono_mode: Some(false), // POLY mode
            pitch_bend_range: None,
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // BASS 1 - Crystalline Distorted Bass (DX7-authentic levels)
        Dx7Preset {
            name: "BASS 1",
            algorithm: 1,
            operators: [
                (1.0, 75.0, 0.0, 0.0), // Op1: Carrier - fundamental bass (reduced from 99)
                (2.0, 50.0, 0.0, 0.0), // Op2: Modulator -> Op1 (reduced from 65)
                (1.0, 68.0, 4.2, 0.0), // Op3: Carrier - detuned bass body (reduced from 90)
                (2.0, 38.0, 0.0, 0.0), // Op4: Modulator -> Op3 (reduced from 50)
                (3.0, 30.0, 0.0, 0.0), // Op5: Modulator -> Op4 (reduced from 40)
                (1.0, 53.0, 0.0, 6.5), // Op6: Modulator -> Op5 + feedback (reduced from 70)
            ],
            envelopes: [
                (99.0, 75.0, 40.0, 70.0, 99.0, 80.0, 70.0, 0.0), // Op1
                (99.0, 99.0, 20.0, 85.0, 99.0, 60.0, 40.0, 0.0), // Op2: Sharp attack
                (99.0, 85.0, 30.0, 80.0, 99.0, 60.0, 30.0, 0.0), // Op3
                (99.0, 99.0, 15.0, 90.0, 99.0, 30.0, 15.0, 0.0), // Op4: Aggressive envelope
                (99.0, 99.0, 10.0, 95.0, 99.0, 20.0, 10.0, 0.0), // Op5: Very aggressive
                (99.0, 75.0, 40.0, 70.0, 99.0, 80.0, 70.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(true), // Bass sounds better in mono
            pitch_bend_range: None,
            portamento_enable: Some(true), // Glide for bass
            portamento_time: Some(20.0),
        },
        // TUBULAR BELL - Bright Metallic Bells
        Dx7Preset {
            name: "TUB BELLS",
            algorithm: 7,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Main carrier
                (1.0, 95.0, 7.0, 0.0),  // Op2: Detuned carrier (chorus)
                (3.5, 70.0, 0.0, 0.0),  // Op3: Bell harmonic modulator
                (1.0, 99.0, -7.0, 0.0), // Op4: Detuned carrier (chorus)
                (7.0, 50.0, 0.0, 0.0),  // Op5: High bell harmonic
                (14.0, 35.0, 0.0, 3.0), // Op6: Very high shimmer + feedback
            ],
            envelopes: [
                (99.0, 50.0, 50.0, 40.0, 99.0, 90.0, 0.0, 0.0), // Op1: Long decay, no sustain
                (99.0, 50.0, 50.0, 40.0, 99.0, 90.0, 0.0, 0.0), // Op2: Long decay
                (99.0, 60.0, 60.0, 50.0, 99.0, 70.0, 0.0, 0.0), // Op3: Medium decay
                (99.0, 50.0, 50.0, 40.0, 99.0, 90.0, 0.0, 0.0), // Op4: Long decay
                (99.0, 70.0, 70.0, 60.0, 99.0, 50.0, 0.0, 0.0), // Op5: Quick decay
                (99.0, 80.0, 80.0, 70.0, 99.0, 30.0, 0.0, 0.0), // Op6: Very quick shimmer
            ],
            master_tune: None,
            mono_mode: Some(false), // POLY mode
            pitch_bend_range: Some(2.0),
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // BRASS 1 - Crystalline Screaming Brass (DX7-authentic levels)
        Dx7Preset {
            name: "BRASS 1",
            algorithm: 16,
            operators: [
                (1.0, 75.0, 0.0, 0.0),  // Op1: Carrier - main brass sound (reduced from 99)
                (1.0, 56.0, 11.2, 0.0), // Op2: Modulator -> Op1 (reduced from 75)
                (2.0, 49.0, -8.7, 0.0), // Op3: Modulator -> Op1 (reduced from 65)
                (3.0, 41.0, 0.0, 0.0),  // Op4: Modulator -> Op3 (reduced from 55)
                (4.0, 53.0, 0.0, 0.0),  // Op5: Modulator -> Op1 (reduced from 70)
                (1.0, 45.0, 0.0, 7.0),  // Op6: Modulator -> Op5 + feedback (reduced from 60)
            ],
            envelopes: [
                (85.0, 80.0, 60.0, 70.0, 99.0, 90.0, 80.0, 0.0), // Op1: More aggressive
                (90.0, 85.0, 55.0, 75.0, 99.0, 85.0, 75.0, 0.0), // Op2: Sharp attack
                (95.0, 90.0, 50.0, 80.0, 99.0, 80.0, 70.0, 0.0), // Op3: Very aggressive
                (99.0, 95.0, 45.0, 85.0, 99.0, 70.0, 50.0, 0.0), // Op4: Extreme attack
                (99.0, 90.0, 40.0, 80.0, 99.0, 60.0, 40.0, 0.0), // Op5: Sharp harmonic
                (85.0, 80.0, 60.0, 70.0, 99.0, 90.0, 80.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(true),       // MONO mode for brass
            pitch_bend_range: Some(3.0), // Good pitch bend for brass
            portamento_enable: Some(true),
            portamento_time: None,
        },
        // STRINGS - Smooth String Ensemble
        Dx7Preset {
            name: "STRINGS",
            algorithm: 32, // All carriers for smooth ensemble
            operators: [
                (1.0, 99.0, 0.0, 0.0),   // Op1: Carrier - main voice
                (1.003, 95.0, 0.0, 0.0), // Op2: Carrier - subtle detune up
                (0.997, 90.0, 0.0, 0.0), // Op3: Carrier - subtle detune down
                (1.006, 85.0, 0.0, 0.0), // Op4: Carrier - more detune up
                (0.994, 80.0, 0.0, 0.0), // Op5: Carrier - more detune down
                (2.0, 70.0, 0.0, 0.0),   // Op6: Carrier - octave for body
            ],
            envelopes: [
                (35.0, 55.0, 65.0, 45.0, 99.0, 95.0, 90.0, 0.0), // Op1: Slow smooth attack, high sustain
                (37.0, 57.0, 63.0, 47.0, 99.0, 93.0, 88.0, 0.0), // Op2: Slightly different timing
                (33.0, 53.0, 67.0, 43.0, 99.0, 91.0, 86.0, 0.0), // Op3: Different timing
                (40.0, 60.0, 60.0, 50.0, 99.0, 89.0, 84.0, 0.0), // Op4: Even slower
                (32.0, 52.0, 68.0, 42.0, 99.0, 87.0, 82.0, 0.0), // Op5: Fastest
                (45.0, 65.0, 55.0, 55.0, 99.0, 85.0, 80.0, 0.0), // Op6: Slowest for warmth
            ],
            master_tune: None,
            mono_mode: Some(false),        // POLY mode
            pitch_bend_range: Some(2.0),   // Standard pitch bend
            portamento_enable: Some(true), // Smooth string glides
            portamento_time: Some(35.0),
        },
        // ORGAN - Crystalline Drawbar Organ (DX7-authentic levels)
        Dx7Preset {
            name: "ORGAN 1",
            algorithm: 32,
            operators: [
                (1.0, 75.0, 0.0, 0.0), // Op1: Fundamental (reduced from 99)
                (2.0, 53.0, 0.0, 0.0), // Op2: 2nd harmonic (reduced from 70)
                (3.0, 38.0, 0.0, 0.0), // Op3: 3rd harmonic (reduced from 50)
                (4.0, 30.0, 0.0, 0.0), // Op4: 4th harmonic (reduced from 40)
                (5.0, 23.0, 0.0, 0.0), // Op5: 5th harmonic (reduced from 30)
                (6.0, 19.0, 0.0, 0.0), // Op6: 6th harmonic (reduced from 25)
            ],
            envelopes: [
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op1
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op2
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op3
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op4
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op5
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(false), // POLY mode
            pitch_bend_range: None, // Organs typically don't have pitch bend
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // CLAV - Funky Clavinet
        Dx7Preset {
            name: "CLAV",
            algorithm: 3,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Carrier fundamental
                (1.0, 99.0, 0.0, 0.0), // Op2: Carrier
                (3.0, 50.0, 0.0, 0.0), // Op3: Harmonic modulator
                (1.0, 99.0, 0.0, 0.0), // Op4: Carrier
                (7.0, 40.0, 0.0, 0.0), // Op5: High harmonic
                (5.0, 45.0, 0.0, 5.0), // Op6: Modulator with feedback
            ],
            envelopes: [
                (99.0, 99.0, 99.0, 70.0, 99.0, 50.0, 0.0, 0.0), // Op1: Sharp percussive, no sustain
                (99.0, 99.0, 99.0, 70.0, 99.0, 50.0, 0.0, 0.0), // Op2: Sharp percussive
                (99.0, 99.0, 99.0, 75.0, 99.0, 35.0, 0.0, 0.0), // Op3: Sharp modulation
                (99.0, 99.0, 99.0, 70.0, 99.0, 50.0, 0.0, 0.0), // Op4: Sharp percussive
                (99.0, 99.0, 99.0, 75.0, 99.0, 30.0, 0.0, 0.0), // Op5: Sharp harmonic
                (99.0, 99.0, 99.0, 75.0, 99.0, 40.0, 0.0, 0.0), // Op6: Sharp modulation
            ],
            master_tune: None,
            mono_mode: Some(false), // POLY mode
            pitch_bend_range: Some(2.0),
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // FLUTE - Crystalline Wooden Flute (DX7-authentic levels)
        Dx7Preset {
            name: "FLUTE",
            algorithm: 19,
            operators: [
                (1.0, 75.0, 0.0, 0.0), // Op1: Carrier - main flute tone (reduced from 99)
                (1.0, 19.0, 0.0, 0.0), // Op2: Modulator -> Op1 (reduced from 25)
                (2.0, 15.0, 0.0, 3.0), // Op3: Modulator -> Op1 + feedback (reduced from 20)
                (1.0, 64.0, 0.0, 0.0), // Op4: Carrier - flute body (reduced from 85)
                (1.0, 56.0, 0.0, 0.0), // Op5: Carrier - flute harmonic (reduced from 75)
                (1.0, 26.0, 0.0, 0.0), // Op6: Modulator -> Op5 (reduced from 35)
            ],
            envelopes: [
                (70.0, 60.0, 60.0, 60.0, 99.0, 95.0, 90.0, 0.0), // Op1
                (70.0, 60.0, 60.0, 60.0, 99.0, 95.0, 90.0, 0.0), // Op2
                (75.0, 65.0, 55.0, 65.0, 99.0, 80.0, 70.0, 0.0), // Op3
                (80.0, 70.0, 50.0, 70.0, 99.0, 70.0, 60.0, 0.0), // Op4
                (75.0, 65.0, 55.0, 65.0, 99.0, 80.0, 70.0, 0.0), // Op5
                (90.0, 99.0, 40.0, 80.0, 60.0, 10.0, 5.0, 0.0),  // Op6
            ],
            master_tune: None,
            mono_mode: Some(true), // Flute is monophonic
            pitch_bend_range: Some(2.0),
            portamento_enable: Some(true), // Natural breath transitions
            portamento_time: Some(15.0),
        },
        // GUITAR - Plucked Guitar
        Dx7Preset {
            name: "GUITAR",
            algorithm: 18,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Carrier - main guitar sound (max level)
                (2.0, 40.0, 0.0, 0.0), // Op2: Modulator -> Op1
                (3.0, 45.0, 0.0, 4.0), // Op3: Modulator -> Op1 + feedback
                (1.0, 50.0, 0.0, 0.0), // Op4: Modulator -> Op1
                (7.0, 30.0, 0.0, 0.0), // Op5: Modulator -> Op4
                (1.0, 35.0, 0.0, 0.0), // Op6: Modulator -> Op5
            ],
            envelopes: [
                (99.0, 80.0, 55.0, 60.0, 99.0, 80.0, 0.0, 0.0), // Op1: Plucked, no sustain
                (99.0, 80.0, 55.0, 60.0, 99.0, 80.0, 0.0, 0.0), // Op2: Plucked
                (99.0, 85.0, 50.0, 65.0, 99.0, 70.0, 0.0, 0.0), // Op3: Sharp modulation
                (99.0, 80.0, 55.0, 60.0, 99.0, 80.0, 0.0, 0.0), // Op4: Plucked
                (99.0, 90.0, 45.0, 70.0, 99.0, 60.0, 0.0, 0.0), // Op5: Sharp harmonic decay
                (99.0, 80.0, 55.0, 60.0, 99.0, 80.0, 0.0, 0.0), // Op6: Plucked
            ],
            master_tune: None,
            mono_mode: Some(false),      // POLY mode
            pitch_bend_range: Some(3.0), // Guitar pitch bend
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // SYNTH BASS - Crystalline DX7 Synth Bass (DX7-authentic levels)
        Dx7Preset {
            name: "SYN BASS",
            algorithm: 6,
            operators: [
                (1.0, 75.0, 0.0, 0.0), // Op1: Carrier - main bass fundamental (reduced from 99)
                (2.0, 26.0, 0.0, 0.0), // Op2: Modulator -> Op1 (reduced from 35)
                (1.0, 64.0, 0.0, 0.0), // Op3: Carrier - bass body (reduced from 85)
                (1.0, 56.0, 0.0, 0.0), // Op4: Carrier - bass harmonic (reduced from 75)
                (2.0, 19.0, 0.0, 0.0), // Op5: Modulator -> Op3 (reduced from 25)
                (1.0, 30.0, 0.0, 2.0), // Op6: Modulator -> Op2 + feedback (reduced from 40)
            ],
            envelopes: [
                (99.0, 80.0, 45.0, 70.0, 99.0, 75.0, 60.0, 0.0), // Op1
                (99.0, 80.0, 45.0, 70.0, 99.0, 75.0, 60.0, 0.0), // Op2
                (99.0, 90.0, 35.0, 80.0, 99.0, 55.0, 40.0, 0.0), // Op3
                (99.0, 80.0, 45.0, 70.0, 99.0, 75.0, 60.0, 0.0), // Op4
                (99.0, 95.0, 30.0, 85.0, 99.0, 45.0, 30.0, 0.0), // Op5
                (99.0, 80.0, 45.0, 70.0, 99.0, 75.0, 60.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(true), // Synth bass in mono
            pitch_bend_range: Some(3.0),
            portamento_enable: Some(true), // Bass glide
            portamento_time: Some(25.0),
        },
        // SAX - Crystalline Saxophone Sound (DX7-authentic levels)
        Dx7Preset {
            name: "SAX",
            algorithm: 11,
            operators: [
                (1.0, 75.0, 0.0, 0.0),  // Op1: Carrier - main sax tone (reduced from 99)
                (1.0, 26.0, 5.0, 0.0),  // Op2: Modulator -> Op1 (reduced from 35)
                (3.0, 19.0, 0.0, 0.0),  // Op3: Modulator -> Op2 (reduced from 25)
                (1.0, 64.0, -5.0, 0.0), // Op4: Carrier - sax body resonance (reduced from 85)
                (7.0, 23.0, 0.0, 0.0),  // Op5: Modulator -> Op4 (reduced from 30)
                (1.0, 30.0, 0.0, 4.0),  // Op6: Modulator -> Op4 + feedback (reduced from 40)
            ],
            envelopes: [
                (70.0, 65.0, 55.0, 60.0, 99.0, 90.0, 80.0, 0.0), // Op1
                (70.0, 65.0, 55.0, 60.0, 99.0, 90.0, 80.0, 0.0), // Op2
                (75.0, 70.0, 50.0, 65.0, 99.0, 75.0, 65.0, 0.0), // Op3
                (70.0, 65.0, 55.0, 60.0, 99.0, 90.0, 80.0, 0.0), // Op4
                (80.0, 75.0, 45.0, 70.0, 99.0, 65.0, 55.0, 0.0), // Op5
                (85.0, 90.0, 40.0, 75.0, 70.0, 30.0, 20.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(true),         // Sax is monophonic
            pitch_bend_range: Some(3.0),   // Good for sax expression
            portamento_enable: Some(true), // Natural sax glissando
            portamento_time: Some(18.0),
        },
        // VIBRAPHONE - Metallic Mallets
        Dx7Preset {
            name: "VIBES",
            algorithm: 9,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Carrier fundamental
                (1.0, 40.0, 7.0, 3.0),  // Op2: Modulator + feedback (shimmer)
                (3.5, 90.0, 0.0, 0.0),  // Op3: Carrier bright metallic
                (7.0, 30.0, 0.0, 0.0),  // Op4: Modulator
                (1.0, 35.0, -7.0, 0.0), // Op5: Modulator (chorus)
                (14.0, 20.0, 0.0, 0.0), // Op6: High shimmer
            ],
            envelopes: [
                (99.0, 45.0, 45.0, 35.0, 99.0, 85.0, 0.0, 0.0), // Op1: Long decay, no sustain
                (99.0, 45.0, 45.0, 35.0, 99.0, 85.0, 0.0, 0.0), // Op2: Long decay
                (99.0, 55.0, 55.0, 45.0, 99.0, 65.0, 0.0, 0.0), // Op3: Medium decay
                (99.0, 65.0, 65.0, 55.0, 99.0, 45.0, 0.0, 0.0), // Op4: Quick decay
                (99.0, 45.0, 45.0, 35.0, 99.0, 85.0, 0.0, 0.0), // Op5: Long decay
                (99.0, 75.0, 75.0, 65.0, 99.0, 25.0, 0.0, 0.0), // Op6: Very quick
            ],
            master_tune: None,
            mono_mode: Some(false),
            pitch_bend_range: Some(2.0),
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // MARIMBA - Percussive Wooden Mallets
        Dx7Preset {
            name: "MARIMBA",
            algorithm: 15,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Fundamental
                (2.0, 80.0, 0.0, 0.0),  // Op2: Octave
                (3.0, 60.0, 0.0, 0.0),  // Op3: Third harmonic
                (4.0, 45.0, 0.0, 0.0),  // Op4: Fourth harmonic
                (7.0, 30.0, 0.0, 0.0),  // Op5: Woody overtone
                (11.0, 20.0, 0.0, 0.0), // Op6: High overtone
            ],
            envelopes: [
                (99.0, 85.0, 85.0, 55.0, 99.0, 70.0, 0.0, 0.0), // Op1: Fast percussive decay
                (99.0, 90.0, 90.0, 60.0, 99.0, 55.0, 0.0, 0.0), // Op2: Faster decay
                (99.0, 95.0, 95.0, 65.0, 99.0, 45.0, 0.0, 0.0), // Op3: Very fast
                (99.0, 99.0, 99.0, 70.0, 99.0, 35.0, 0.0, 0.0), // Op4: Instant decay
                (99.0, 99.0, 99.0, 75.0, 99.0, 25.0, 0.0, 0.0), // Op5: Sharp attack
                (99.0, 99.0, 99.0, 80.0, 99.0, 15.0, 0.0, 0.0), // Op6: Very sharp
            ],
            master_tune: None,
            mono_mode: Some(false),
            pitch_bend_range: Some(2.0),
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // HARPSICHORD - Bright Harpsichord
        Dx7Preset {
            name: "HARPSI",
            algorithm: 4,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Fundamental
                (2.0, 70.0, 0.0, 0.0), // Op2: Octave
                (4.0, 50.0, 0.0, 0.0), // Op3: Two octaves
                (8.0, 35.0, 0.0, 0.0), // Op4: Three octaves
                (1.0, 85.0, 5.0, 0.0), // Op5: Detuned fundamental (chorus)
                (3.0, 40.0, 0.0, 3.0), // Op6: Pluck attack + feedback
            ],
            envelopes: [
                (99.0, 85.0, 85.0, 75.0, 99.0, 50.0, 0.0, 0.0), // Op1: Percussive
                (99.0, 90.0, 90.0, 80.0, 99.0, 40.0, 0.0, 0.0), // Op2: Quick
                (99.0, 95.0, 95.0, 85.0, 99.0, 30.0, 0.0, 0.0), // Op3: Quicker
                (99.0, 99.0, 99.0, 90.0, 99.0, 20.0, 0.0, 0.0), // Op4: Very quick
                (99.0, 85.0, 85.0, 75.0, 99.0, 50.0, 0.0, 0.0), // Op5: Percussive
                (99.0, 99.0, 99.0, 95.0, 99.0, 15.0, 0.0, 0.0), // Op6: Sharp pluck
            ],
            master_tune: None,
            mono_mode: Some(false),
            pitch_bend_range: Some(2.0),
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // WOODBLOCK - Sharp Percussion
        Dx7Preset {
            name: "WOODBLOK",
            algorithm: 12,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Fundamental
                (3.5, 75.0, 0.0, 0.0),  // Op2: Woody overtone
                (7.2, 65.0, 0.0, 0.0),  // Op3: Higher woody tone
                (11.3, 55.0, 0.0, 0.0), // Op4: Even higher
                (1.0, 85.0, 0.0, 0.0),  // Op5: Fundamental support
                (2.1, 45.0, 0.0, 7.0),  // Op6: Noise with max feedback
            ],
            envelopes: [
                (99.0, 99.0, 99.0, 99.0, 99.0, 20.0, 0.0, 0.0), // Op1: Very short
                (99.0, 99.0, 99.0, 99.0, 99.0, 15.0, 0.0, 0.0), // Op2: Very short
                (99.0, 99.0, 99.0, 99.0, 99.0, 10.0, 0.0, 0.0), // Op3: Ultra short
                (99.0, 99.0, 99.0, 99.0, 99.0, 8.0, 0.0, 0.0),  // Op4: Ultra short
                (99.0, 99.0, 99.0, 99.0, 99.0, 20.0, 0.0, 0.0), // Op5: Very short
                (99.0, 99.0, 99.0, 99.0, 60.0, 5.0, 0.0, 0.0),  // Op6: Noise burst
            ],
            master_tune: None,
            mono_mode: Some(false),
            pitch_bend_range: Some(1.0),
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // XYLOPHONE - Bright Xylophone
        Dx7Preset {
            name: "XYLO",
            algorithm: 1,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Fundamental
                (3.0, 80.0, 0.0, 0.0),  // Op2: Third harmonic
                (5.0, 65.0, 0.0, 0.0),  // Op3: Fifth harmonic
                (7.0, 50.0, 0.0, 0.0),  // Op4: Seventh harmonic
                (9.0, 40.0, 0.0, 0.0),  // Op5: Ninth harmonic
                (11.0, 30.0, 0.0, 2.0), // Op6: Eleventh harmonic + feedback
            ],
            envelopes: [
                (99.0, 65.0, 65.0, 55.0, 99.0, 70.0, 0.0, 0.0), // Op1: Percussive
                (99.0, 70.0, 70.0, 60.0, 99.0, 60.0, 0.0, 0.0), // Op2: Quick
                (99.0, 75.0, 75.0, 65.0, 99.0, 50.0, 0.0, 0.0), // Op3: Quicker
                (99.0, 80.0, 80.0, 70.0, 99.0, 40.0, 0.0, 0.0), // Op4: Very quick
                (99.0, 85.0, 85.0, 75.0, 99.0, 30.0, 0.0, 0.0), // Op5: Sharp
                (99.0, 90.0, 90.0, 80.0, 99.0, 20.0, 0.0, 0.0), // Op6: Very sharp
            ],
            master_tune: None,
            mono_mode: Some(false),
            pitch_bend_range: Some(2.0),
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // CLARINET - Crystalline Clarinet (DX7-authentic levels)
        Dx7Preset {
            name: "CLARINET",
            algorithm: 19,
            operators: [
                (1.0, 75.0, 0.0, 0.0), // Op1: Fundamental (reduced from 99)
                (3.0, 45.0, 0.0, 0.0), // Op2: Third harmonic (reduced from 60)
                (5.0, 30.0, 0.0, 0.0), // Op3: Fifth harmonic (reduced from 40)
                (7.0, 19.0, 0.0, 0.0), // Op4: Seventh harmonic (reduced from 25)
                (1.0, 64.0, 0.0, 0.0), // Op5: Fundamental support (reduced from 85)
                (2.0, 23.0, 0.0, 4.0), // Op6: Breath noise (reduced from 30)
            ],
            envelopes: [
                (65.0, 60.0, 65.0, 55.0, 99.0, 95.0, 90.0, 0.0), // Op1
                (70.0, 65.0, 60.0, 60.0, 99.0, 80.0, 70.0, 0.0), // Op2
                (75.0, 70.0, 55.0, 65.0, 99.0, 65.0, 55.0, 0.0), // Op3
                (80.0, 75.0, 50.0, 70.0, 99.0, 50.0, 40.0, 0.0), // Op4
                (65.0, 60.0, 65.0, 55.0, 99.0, 95.0, 90.0, 0.0), // Op5
                (90.0, 95.0, 45.0, 80.0, 50.0, 15.0, 10.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(true),         // Clarinet is monophonic
            pitch_bend_range: Some(2.0),   // Standard wind instrument pitch bend
            portamento_enable: Some(true), // Natural legato transitions
            portamento_time: Some(20.0),
        },
        // OBOE - Crystalline Oboe (DX7-authentic levels)
        Dx7Preset {
            name: "OBOE",
            algorithm: 8,
            operators: [
                (1.0, 75.0, 0.0, 0.0), // Op1: Fundamental (reduced from 99)
                (2.0, 56.0, 0.0, 0.0), // Op2: Octave (reduced from 75)
                (3.0, 49.0, 0.0, 0.0), // Op3: Third harmonic (reduced from 65)
                (4.0, 41.0, 0.0, 0.0), // Op4: Fourth harmonic (reduced from 55)
                (5.0, 34.0, 0.0, 0.0), // Op5: Fifth harmonic (reduced from 45)
                (1.0, 45.0, 0.0, 5.0), // Op6: Reed noise (reduced from 60)
            ],
            envelopes: [
                (70.0, 65.0, 60.0, 60.0, 99.0, 90.0, 85.0, 0.0), // Op1
                (75.0, 70.0, 55.0, 65.0, 99.0, 80.0, 70.0, 0.0), // Op2
                (80.0, 75.0, 50.0, 70.0, 99.0, 70.0, 60.0, 0.0), // Op3
                (85.0, 80.0, 45.0, 75.0, 99.0, 60.0, 50.0, 0.0), // Op4
                (90.0, 85.0, 40.0, 80.0, 99.0, 50.0, 40.0, 0.0), // Op5
                (95.0, 99.0, 35.0, 85.0, 60.0, 20.0, 10.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(true),         // Oboe is monophonic
            pitch_bend_range: Some(2.0),   // Standard wind instrument pitch bend
            portamento_enable: Some(true), // Natural legato transitions
            portamento_time: Some(25.0),
        },
        // TRUMPET - Crystalline Trumpet (DX7-authentic levels)
        Dx7Preset {
            name: "TRUMPET",
            algorithm: 22,
            operators: [
                (1.0, 75.0, 0.0, 0.0), // Op1: Carrier - trumpet fundamental (reduced from 99)
                (2.0, 23.0, 0.0, 0.0), // Op2: Modulator -> Op1 (reduced from 30)
                (2.0, 64.0, 0.0, 0.0), // Op3: Carrier - second harmonic (reduced from 85)
                (3.0, 56.0, 0.0, 0.0), // Op4: Carrier - third harmonic (reduced from 75)
                (4.0, 53.0, 0.0, 0.0), // Op5: Carrier - fourth harmonic (reduced from 70)
                (1.0, 26.0, 0.0, 2.0), // Op6: Modulator -> Op3,4,5 + feedback (reduced from 35)
            ],
            envelopes: [
                (75.0, 70.0, 60.0, 65.0, 99.0, 85.0, 80.0, 0.0), // Op1
                (80.0, 75.0, 55.0, 70.0, 99.0, 80.0, 70.0, 0.0), // Op2
                (85.0, 80.0, 50.0, 75.0, 99.0, 75.0, 65.0, 0.0), // Op3
                (90.0, 85.0, 45.0, 80.0, 99.0, 70.0, 60.0, 0.0), // Op4
                (95.0, 90.0, 40.0, 85.0, 99.0, 65.0, 55.0, 0.0), // Op5
                (99.0, 95.0, 35.0, 90.0, 99.0, 60.0, 50.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(true),         // Trumpet is monophonic
            pitch_bend_range: Some(3.0),   // Good pitch bend range for brass
            portamento_enable: Some(true), // Natural brass glide
            portamento_time: Some(15.0),
        },
        // TUBA - Crystalline Tuba (DX7-authentic levels)
        Dx7Preset {
            name: "TUBA",
            algorithm: 1,
            operators: [
                (1.0, 75.0, 0.0, 0.0), // Op1: Fundamental (reduced from 99)
                (0.5, 64.0, 0.0, 0.0), // Op2: Sub octave (reduced from 85)
                (2.0, 45.0, 0.0, 0.0), // Op3: Octave (reduced from 60)
                (3.0, 34.0, 0.0, 0.0), // Op4: Third harmonic (reduced from 45)
                (4.0, 23.0, 0.0, 0.0), // Op5: Fourth harmonic (reduced from 30)
                (1.0, 53.0, 0.0, 2.0), // Op6: Breath with feedback (reduced from 70)
            ],
            envelopes: [
                (60.0, 55.0, 60.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op1
                (60.0, 55.0, 60.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op2
                (65.0, 60.0, 55.0, 55.0, 99.0, 80.0, 70.0, 0.0), // Op3
                (70.0, 65.0, 50.0, 60.0, 99.0, 70.0, 60.0, 0.0), // Op4
                (75.0, 70.0, 45.0, 65.0, 99.0, 60.0, 50.0, 0.0), // Op5
                (80.0, 85.0, 40.0, 70.0, 70.0, 30.0, 20.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(true),         // Tuba is monophonic
            pitch_bend_range: Some(2.0),   // Conservative pitch bend for low brass
            portamento_enable: Some(true), // Natural brass glide
            portamento_time: Some(30.0),
        },
        // SPACE VOICE - Ethereal Pad
        Dx7Preset {
            name: "SPACE",
            algorithm: 28,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Fundamental
                (1.01, 95.0, 0.0, 0.0), // Op2: Slightly detuned
                (0.99, 90.0, 0.0, 0.0), // Op3: Slightly detuned
                (2.0, 60.0, 0.0, 0.0),  // Op4: Octave
                (3.0, 40.0, 0.0, 0.0),  // Op5: Third harmonic
                (1.5, 50.0, 0.0, 2.0),  // Op6: Perfect fifth with feedback
            ],
            envelopes: [
                (30.0, 40.0, 60.0, 35.0, 99.0, 95.0, 90.0, 0.0), // Op1
                (30.0, 40.0, 60.0, 35.0, 99.0, 95.0, 90.0, 0.0), // Op2
                (30.0, 40.0, 60.0, 35.0, 99.0, 95.0, 90.0, 0.0), // Op3
                (35.0, 45.0, 55.0, 40.0, 99.0, 85.0, 80.0, 0.0), // Op4
                (40.0, 50.0, 50.0, 45.0, 99.0, 75.0, 70.0, 0.0), // Op5
                (45.0, 55.0, 45.0, 50.0, 99.0, 65.0, 60.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(false),        // POLY mode
            pitch_bend_range: Some(4.0),   // Wide pitch bend for ethereal effects
            portamento_enable: Some(true), // Smooth pad transitions
            portamento_time: Some(40.0),
        },
        // GAMELAN - Metallic Percussion
        Dx7Preset {
            name: "GAMELAN",
            algorithm: 13,
            operators: [
                (1.0, 99.0, 0.0, 0.0),                  // Op1: Fundamental
                (std::f32::consts::PI, 75.0, 0.0, 0.0), // Op2: Inharmonic
                (5.67, 65.0, 0.0, 0.0),                 // Op3: Inharmonic
                (8.23, 55.0, 0.0, 0.0),                 // Op4: Inharmonic
                (11.41, 45.0, 0.0, 0.0),                // Op5: Inharmonic
                (1.0, 85.0, 0.0, 1.0),                  // Op6: Fundamental with feedback
            ],
            envelopes: [
                (99.0, 40.0, 25.0, 30.0, 99.0, 80.0, 70.0, 0.0), // Op1
                (99.0, 45.0, 20.0, 35.0, 99.0, 70.0, 60.0, 0.0), // Op2
                (99.0, 50.0, 15.0, 40.0, 99.0, 60.0, 50.0, 0.0), // Op3
                (99.0, 55.0, 10.0, 45.0, 99.0, 50.0, 40.0, 0.0), // Op4
                (99.0, 60.0, 8.0, 50.0, 99.0, 40.0, 30.0, 0.0),  // Op5
                (99.0, 40.0, 25.0, 30.0, 99.0, 80.0, 70.0, 0.0), // Op6
            ],
            master_tune: None,
            mono_mode: Some(false),      // POLY mode
            pitch_bend_range: Some(1.0), // Very small pitch bend for metallic percussion
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // CHAOS LEAD - Extreme FM with maximum chaos
        Dx7Preset {
            name: "CHAOS",
            algorithm: 32, // All operators as carriers for maximum chaos
            operators: [
                (1.0, 99.0, 0.0, 7.0),                   // Op1: Fundamental with max feedback
                (1.618, 95.0, 15.7, 7.0), // Op2: Golden ratio with max feedback + extreme detune
                (2.414, 90.0, -23.1, 7.0), // Op3: Square root of 6 with max feedback
                (std::f32::consts::PI, 85.0, 31.4, 7.0), // Op4: Pi ratio with max feedback
                (5.196, 80.0, -18.9, 7.0), // Op5: Fibonacci ratio with max feedback
                (7.777, 75.0, 42.0, 7.0), // Op6: High inharmonic ratio with max feedback
            ],
            envelopes: [
                (99.0, 99.0, 5.0, 99.0, 99.0, 90.0, 80.0, 0.0), // Op1: Sharp attack, harsh decay
                (99.0, 95.0, 8.0, 95.0, 99.0, 85.0, 70.0, 0.0), // Op2: Slightly softer
                (99.0, 90.0, 12.0, 90.0, 99.0, 80.0, 60.0, 0.0), // Op3
                (99.0, 85.0, 15.0, 85.0, 99.0, 75.0, 50.0, 0.0), // Op4
                (99.0, 80.0, 20.0, 80.0, 99.0, 70.0, 40.0, 0.0), // Op5
                (99.0, 75.0, 25.0, 75.0, 99.0, 65.0, 30.0, 0.0), // Op6
            ],
            master_tune: Some(7.3),       // Slightly detuned for extra chaos
            mono_mode: Some(false),       // POLY for maximum chaos
            pitch_bend_range: Some(12.0), // Full octave bend
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // WOBBLE BASS - Extreme modulation bass
        Dx7Preset {
            name: "WOBBLE",
            algorithm: 4, // Complex feedback algorithm
            operators: [
                (1.0, 99.0, 0.0, 0.0),   // Op1: Fundamental carrier
                (0.25, 99.0, 0.0, 6.5),  // Op2: Sub-bass with high feedback (wobble generator)
                (2.0, 80.0, 7.3, 0.0),   // Op3: Octave modulator with detune
                (1.0, 85.0, -11.7, 0.0), // Op4: Detuned carrier
                (4.0, 70.0, 0.0, 0.0),   // Op5: High harmonic modulator
                (8.0, 60.0, 0.0, 5.5),   // Op6: Very high modulator with feedback
            ],
            envelopes: [
                (99.0, 60.0, 80.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op1: Sustained bass
                (99.0, 30.0, 90.0, 40.0, 99.0, 99.0, 95.0, 0.0), // Op2: Slow wobble envelope
                (99.0, 80.0, 60.0, 70.0, 99.0, 70.0, 50.0, 0.0), // Op3: Quick modulation
                (99.0, 60.0, 80.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op4: Match fundamental
                (99.0, 90.0, 40.0, 80.0, 99.0, 50.0, 30.0, 0.0), // Op5: Sharp harmonic
                (99.0, 95.0, 30.0, 85.0, 99.0, 40.0, 20.0, 0.0), // Op6: Very sharp
            ],
            master_tune: None,
            mono_mode: Some(true), // MONO for bass
            pitch_bend_range: Some(5.0),
            portamento_enable: Some(true),
            portamento_time: Some(15.0),
        },
        // METALLIC STAB - Aggressive Stab
        Dx7Preset {
            name: "STAB",
            algorithm: 23,
            operators: [
                (1.0, 99.0, 0.0, 0.0),                 // Op1: Fundamental
                (11.73, 85.0, 0.0, 0.0),               // Op2: High inharmonic
                (17.32, 75.0, 13.5, 0.0),              // Op3: Very high inharmonic
                (23.89, 65.0, -19.2, 0.0),             // Op4: Extreme inharmonic
                (1.0, 90.0, 0.0, 7.0),                 // Op5: Fundamental with max feedback
                (std::f32::consts::E, 80.0, 0.0, 7.0), // Op6: E ratio with max feedback
            ],
            envelopes: [
                (99.0, 99.0, 99.0, 99.0, 99.0, 30.0, 0.0, 0.0), // Op1: Ultra short
                (99.0, 99.0, 99.0, 99.0, 99.0, 25.0, 0.0, 0.0), // Op2: Ultra short
                (99.0, 99.0, 99.0, 99.0, 99.0, 20.0, 0.0, 0.0), // Op3: Ultra short
                (99.0, 99.0, 99.0, 99.0, 99.0, 15.0, 0.0, 0.0), // Op4: Ultra short
                (99.0, 99.0, 99.0, 99.0, 99.0, 30.0, 0.0, 0.0), // Op5: Ultra short
                (99.0, 99.0, 99.0, 99.0, 99.0, 35.0, 0.0, 0.0), // Op6: Slightly longer
            ],
            master_tune: None,
            mono_mode: Some(false),
            pitch_bend_range: Some(1.0),
            portamento_enable: Some(false),
            portamento_time: None,
        },
        // SPACE CHOIR - Ethereal pad
        Dx7Preset {
            name: "CHOIR",
            algorithm: 28, // All carriers algorithm
            operators: [
                (1.0, 99.0, 0.0, 2.0),   // Op1: Fundamental with moderate feedback
                (1.004, 95.0, 0.0, 0.0), // Op2: Slightly detuned (beat frequency)
                (0.996, 90.0, 0.0, 0.0), // Op3: Slightly detuned opposite
                (2.0, 75.0, 0.0, 1.5),   // Op4: Octave with moderate feedback
                (3.0, 60.0, 3.0, 0.0),   // Op5: Third harmonic with moderate detune
                (1.5, 70.0, -5.0, 2.5),  // Op6: Perfect fifth with moderate detune and feedback
            ],
            envelopes: [
                (20.0, 30.0, 75.0, 25.0, 99.0, 90.0, 75.0, 0.0), // Op1: Slow ethereal attack
                (23.0, 33.0, 70.0, 28.0, 99.0, 88.0, 72.0, 0.0), // Op2: Slightly different timing
                (25.0, 35.0, 65.0, 30.0, 99.0, 86.0, 70.0, 0.0), // Op3: More different timing
                (30.0, 40.0, 60.0, 35.0, 99.0, 80.0, 65.0, 0.0), // Op4: Slower octave entry
                (35.0, 45.0, 55.0, 40.0, 99.0, 70.0, 60.0, 0.0), // Op5: Even slower harmonic
                (18.0, 28.0, 80.0, 23.0, 99.0, 92.0, 77.0, 0.0), // Op6: Fastest ethereal entry
            ],
            master_tune: Some(-1.5),       // Slightly flat for warmth
            mono_mode: Some(false),        // POLY for chord pads
            pitch_bend_range: Some(2.0),   // Standard pitch bend
            portamento_enable: Some(true), // Smooth pad glides
            portamento_time: Some(30.0),   // Moderate glide
        },
    ]
}
