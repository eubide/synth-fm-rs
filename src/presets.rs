use crate::fm_synth::FmSynthesizer;

pub struct Dx7Preset {
    pub name: &'static str,
    pub algorithm: u8,
    pub operators: [(f32, f32, f32, f32); 6], // (ratio, level, detune, feedback)
    pub envelopes: [(f32, f32, f32, f32, f32, f32, f32, f32); 6], // (r1-r4, l1-l4)
    // Function Mode parameters (optional, using DX7 defaults if not specified)
    pub master_tune: Option<f32>,        // ±150 cents
    pub mono_mode: Option<bool>,         // false = poly, true = mono
    pub pitch_bend_range: Option<f32>,   // 0-12 semitones
    pub portamento_enable: Option<bool>, // portamento on/off
    pub portamento_time: Option<f32>,    // 0-99
}

impl Dx7Preset {
    pub fn apply_to_synth(&self, synth: &mut FmSynthesizer) {
        synth.algorithm = self.algorithm;
        synth.preset_name = self.name.to_string();

        // Apply operator settings to all voices
        for voice in &mut synth.voices {
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
        // E.PIANO 1 - Classic DX7 Electric Piano
        Dx7Preset {
            name: "E.PIANO 1",
            algorithm: 5,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Carrier
                (1.0, 85.0, 2.0, 0.0),  // Op2: Carrier
                (7.0, 45.0, 0.0, 0.0),  // Op3: Modulator (bell tone)
                (1.0, 60.0, -1.0, 0.0), // Op4: Carrier
                (14.0, 25.0, 0.0, 0.0), // Op5: Modulator (brightness)
                (1.0, 70.0, 0.0, 3.0),  // Op6: Modulator with feedback
            ],
            envelopes: [
                (99.0, 85.0, 70.0, 75.0, 99.0, 85.0, 60.0, 0.0), // Op1
                (99.0, 85.0, 70.0, 75.0, 99.0, 85.0, 60.0, 0.0), // Op2
                (99.0, 95.0, 50.0, 99.0, 99.0, 50.0, 0.0, 0.0),  // Op3
                (99.0, 85.0, 70.0, 75.0, 99.0, 85.0, 60.0, 0.0), // Op4
                (99.0, 95.0, 50.0, 99.0, 99.0, 50.0, 0.0, 0.0),  // Op5
                (99.0, 85.0, 70.0, 75.0, 99.0, 85.0, 60.0, 0.0), // Op6
            ],
        },
        // BASS 1 - Solid Bass
        Dx7Preset {
            name: "BASS 1",
            algorithm: 1,
            operators: [
                (0.5, 99.0, 0.0, 0.0), // Op1: Sub bass carrier
                (1.0, 80.0, 0.0, 0.0), // Op2: Fundamental
                (2.0, 50.0, 0.0, 0.0), // Op3: First harmonic
                (3.0, 30.0, 0.0, 0.0), // Op4: Second harmonic
                (4.0, 20.0, 0.0, 0.0), // Op5: Third harmonic
                (0.5, 60.0, 0.0, 2.0), // Op6: Modulator with feedback
            ],
            envelopes: [
                (99.0, 75.0, 40.0, 70.0, 99.0, 80.0, 70.0, 0.0), // Op1
                (99.0, 75.0, 40.0, 70.0, 99.0, 80.0, 70.0, 0.0), // Op2
                (99.0, 85.0, 30.0, 80.0, 99.0, 60.0, 30.0, 0.0), // Op3
                (99.0, 90.0, 20.0, 85.0, 99.0, 40.0, 20.0, 0.0), // Op4
                (99.0, 95.0, 10.0, 90.0, 99.0, 20.0, 10.0, 0.0), // Op5
                (99.0, 75.0, 40.0, 70.0, 99.0, 80.0, 70.0, 0.0), // Op6
            ],
        },
        // TUBULAR BELL
        Dx7Preset {
            name: "TUB BELLS",
            algorithm: 7,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Main carrier
                (1.0, 85.0, 7.0, 0.0),  // Op2: Detuned carrier
                (3.5, 70.0, 0.0, 0.0),  // Op3: Modulator
                (1.0, 75.0, -7.0, 0.0), // Op4: Detuned carrier
                (7.0, 50.0, 0.0, 0.0),  // Op5: High modulator
                (14.0, 30.0, 0.0, 1.0), // Op6: Very high modulator
            ],
            envelopes: [
                (99.0, 50.0, 35.0, 40.0, 99.0, 90.0, 80.0, 0.0), // Op1
                (99.0, 50.0, 35.0, 40.0, 99.0, 90.0, 80.0, 0.0), // Op2
                (99.0, 60.0, 30.0, 50.0, 99.0, 70.0, 40.0, 0.0), // Op3
                (99.0, 50.0, 35.0, 40.0, 99.0, 90.0, 80.0, 0.0), // Op4
                (99.0, 70.0, 25.0, 60.0, 99.0, 50.0, 20.0, 0.0), // Op5
                (99.0, 80.0, 20.0, 70.0, 99.0, 30.0, 10.0, 0.0), // Op6
            ],
        },
        // BRASS
        Dx7Preset {
            name: "BRASS 1",
            algorithm: 16,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Carrier
                (1.0, 90.0, 3.0, 0.0),  // Op2: Carrier
                (1.0, 85.0, -3.0, 0.0), // Op3: Carrier
                (2.0, 60.0, 0.0, 0.0),  // Op4: Modulator
                (3.0, 40.0, 0.0, 0.0),  // Op5: Modulator
                (1.0, 70.0, 0.0, 4.0),  // Op6: Modulator with feedback
            ],
            envelopes: [
                (75.0, 70.0, 50.0, 60.0, 99.0, 85.0, 75.0, 0.0), // Op1
                (75.0, 70.0, 50.0, 60.0, 99.0, 85.0, 75.0, 0.0), // Op2
                (75.0, 70.0, 50.0, 60.0, 99.0, 85.0, 75.0, 0.0), // Op3
                (80.0, 75.0, 45.0, 65.0, 99.0, 70.0, 50.0, 0.0), // Op4
                (85.0, 80.0, 40.0, 70.0, 99.0, 60.0, 40.0, 0.0), // Op5
                (75.0, 70.0, 50.0, 60.0, 99.0, 85.0, 75.0, 0.0), // Op6
            ],
        },
        // STRINGS
        Dx7Preset {
            name: "STRINGS",
            algorithm: 14,
            operators: [
                (1.0, 99.0, 0.0, 0.0),   // Op1: Main carrier
                (1.0, 95.0, 7.0, 0.0),   // Op2: Detuned carrier
                (0.99, 90.0, -7.0, 0.0), // Op3: Slightly detuned
                (1.01, 85.0, 0.0, 0.0),  // Op4: Slightly sharp
                (2.0, 30.0, 0.0, 0.0),   // Op5: Modulator
                (3.0, 25.0, 0.0, 1.0),   // Op6: Modulator
            ],
            envelopes: [
                (50.0, 60.0, 50.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op1
                (50.0, 60.0, 50.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op2
                (50.0, 60.0, 50.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op3
                (50.0, 60.0, 50.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op4
                (60.0, 70.0, 40.0, 60.0, 99.0, 70.0, 50.0, 0.0), // Op5
                (60.0, 70.0, 40.0, 60.0, 99.0, 70.0, 50.0, 0.0), // Op6
            ],
        },
        // ORGAN
        Dx7Preset {
            name: "ORGAN 1",
            algorithm: 32,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Fundamental
                (2.0, 70.0, 0.0, 0.0), // Op2: 2nd harmonic
                (3.0, 50.0, 0.0, 0.0), // Op3: 3rd harmonic
                (4.0, 40.0, 0.0, 0.0), // Op4: 4th harmonic
                (5.0, 30.0, 0.0, 0.0), // Op5: 5th harmonic
                (6.0, 25.0, 0.0, 0.0), // Op6: 6th harmonic
            ],
            envelopes: [
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op1
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op2
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op3
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op4
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op5
                (99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0), // Op6
            ],
        },
        // CLAV
        Dx7Preset {
            name: "CLAV",
            algorithm: 3,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Carrier
                (1.0, 85.0, 0.0, 0.0), // Op2: Carrier
                (3.0, 60.0, 0.0, 0.0), // Op3: Modulator
                (1.0, 75.0, 0.0, 0.0), // Op4: Carrier
                (7.0, 40.0, 0.0, 0.0), // Op5: Modulator
                (5.0, 50.0, 0.0, 5.0), // Op6: Modulator with feedback
            ],
            envelopes: [
                (99.0, 95.0, 30.0, 85.0, 99.0, 40.0, 20.0, 0.0), // Op1
                (99.0, 95.0, 30.0, 85.0, 99.0, 40.0, 20.0, 0.0), // Op2
                (99.0, 99.0, 20.0, 90.0, 99.0, 20.0, 0.0, 0.0),  // Op3
                (99.0, 95.0, 30.0, 85.0, 99.0, 40.0, 20.0, 0.0), // Op4
                (99.0, 99.0, 20.0, 90.0, 99.0, 20.0, 0.0, 0.0),  // Op5
                (99.0, 99.0, 20.0, 90.0, 99.0, 20.0, 0.0, 0.0),  // Op6
            ],
        },
        // FLUTE
        Dx7Preset {
            name: "FLUTE",
            algorithm: 2,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Main carrier
                (1.0, 30.0, 0.0, 0.0), // Op2: Soft carrier
                (2.0, 20.0, 0.0, 0.0), // Op3: Modulator
                (3.0, 15.0, 0.0, 0.0), // Op4: Modulator
                (1.0, 25.0, 0.0, 0.0), // Op5: Modulator
                (1.0, 40.0, 0.0, 6.0), // Op6: Breath noise
            ],
            envelopes: [
                (70.0, 60.0, 60.0, 60.0, 99.0, 95.0, 90.0, 0.0), // Op1
                (70.0, 60.0, 60.0, 60.0, 99.0, 95.0, 90.0, 0.0), // Op2
                (75.0, 65.0, 55.0, 65.0, 99.0, 80.0, 70.0, 0.0), // Op3
                (80.0, 70.0, 50.0, 70.0, 99.0, 70.0, 60.0, 0.0), // Op4
                (75.0, 65.0, 55.0, 65.0, 99.0, 80.0, 70.0, 0.0), // Op5
                (90.0, 99.0, 40.0, 80.0, 60.0, 10.0, 5.0, 0.0),  // Op6
            ],
        },
        // GUITAR - Iconic DX7 Guitar Sound
        Dx7Preset {
            name: "GUITAR",
            algorithm: 4,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Main carrier
                (2.0, 75.0, 0.0, 0.0), // Op2: Carrier
                (3.0, 55.0, 0.0, 0.0), // Op3: Modulator
                (1.0, 85.0, 0.0, 0.0), // Op4: Carrier
                (7.0, 45.0, 0.0, 0.0), // Op5: Modulator
                (1.0, 65.0, 0.0, 4.0), // Op6: Feedback for grit
            ],
            envelopes: [
                (99.0, 75.0, 40.0, 65.0, 99.0, 70.0, 50.0, 0.0), // Op1
                (99.0, 75.0, 40.0, 65.0, 99.0, 70.0, 50.0, 0.0), // Op2
                (99.0, 85.0, 30.0, 75.0, 99.0, 50.0, 30.0, 0.0), // Op3
                (99.0, 75.0, 40.0, 65.0, 99.0, 70.0, 50.0, 0.0), // Op4
                (99.0, 90.0, 25.0, 80.0, 99.0, 40.0, 20.0, 0.0), // Op5
                (99.0, 75.0, 40.0, 65.0, 99.0, 70.0, 50.0, 0.0), // Op6
            ],
        },
        // SYNTH BASS - Classic DX7 Synth Bass
        Dx7Preset {
            name: "SYN BASS",
            algorithm: 6,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Main bass
                (0.5, 85.0, 0.0, 0.0), // Op2: Sub bass
                (2.0, 60.0, 0.0, 0.0), // Op3: Modulator
                (1.0, 75.0, 0.0, 0.0), // Op4: Carrier
                (3.0, 40.0, 0.0, 0.0), // Op5: Modulator
                (1.0, 55.0, 0.0, 3.0), // Op6: Feedback warmth
            ],
            envelopes: [
                (99.0, 80.0, 45.0, 70.0, 99.0, 75.0, 60.0, 0.0), // Op1
                (99.0, 80.0, 45.0, 70.0, 99.0, 75.0, 60.0, 0.0), // Op2
                (99.0, 90.0, 35.0, 80.0, 99.0, 55.0, 40.0, 0.0), // Op3
                (99.0, 80.0, 45.0, 70.0, 99.0, 75.0, 60.0, 0.0), // Op4
                (99.0, 95.0, 30.0, 85.0, 99.0, 45.0, 30.0, 0.0), // Op5
                (99.0, 80.0, 45.0, 70.0, 99.0, 75.0, 60.0, 0.0), // Op6
            ],
        },
        // SAX - Saxophone Sound
        Dx7Preset {
            name: "SAX",
            algorithm: 11,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Main carrier
                (1.0, 90.0, 5.0, 0.0),  // Op2: Detuned carrier
                (3.0, 70.0, 0.0, 0.0),  // Op3: Modulator
                (1.0, 85.0, -5.0, 0.0), // Op4: Detuned carrier
                (7.0, 50.0, 0.0, 0.0),  // Op5: Modulator
                (1.0, 75.0, 0.0, 5.0),  // Op6: Breath with feedback
            ],
            envelopes: [
                (70.0, 65.0, 55.0, 60.0, 99.0, 90.0, 80.0, 0.0), // Op1
                (70.0, 65.0, 55.0, 60.0, 99.0, 90.0, 80.0, 0.0), // Op2
                (75.0, 70.0, 50.0, 65.0, 99.0, 75.0, 65.0, 0.0), // Op3
                (70.0, 65.0, 55.0, 60.0, 99.0, 90.0, 80.0, 0.0), // Op4
                (80.0, 75.0, 45.0, 70.0, 99.0, 65.0, 55.0, 0.0), // Op5
                (85.0, 90.0, 40.0, 75.0, 70.0, 30.0, 20.0, 0.0), // Op6
            ],
        },
        // VIBRAPHONE - Classic Mallet Sound
        Dx7Preset {
            name: "VIBES",
            algorithm: 9,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Main carrier
                (1.0, 85.0, 7.0, 0.0),  // Op2: Detuned carrier
                (3.5, 65.0, 0.0, 0.0),  // Op3: Metallic modulator
                (7.0, 45.0, 0.0, 0.0),  // Op4: High modulator
                (1.0, 75.0, -7.0, 0.0), // Op5: Detuned carrier
                (14.0, 25.0, 0.0, 1.0), // Op6: Shimmer
            ],
            envelopes: [
                (99.0, 45.0, 30.0, 35.0, 99.0, 85.0, 75.0, 0.0), // Op1
                (99.0, 45.0, 30.0, 35.0, 99.0, 85.0, 75.0, 0.0), // Op2
                (99.0, 55.0, 25.0, 45.0, 99.0, 65.0, 45.0, 0.0), // Op3
                (99.0, 65.0, 20.0, 55.0, 99.0, 45.0, 25.0, 0.0), // Op4
                (99.0, 45.0, 30.0, 35.0, 99.0, 85.0, 75.0, 0.0), // Op5
                (99.0, 75.0, 15.0, 65.0, 99.0, 25.0, 10.0, 0.0), // Op6
            ],
        },
        // MARIMBA - Wooden Mallet Sound
        Dx7Preset {
            name: "MARIMBA",
            algorithm: 15,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Fundamental
                (2.0, 75.0, 0.0, 0.0),  // Op2: Octave
                (3.0, 55.0, 0.0, 0.0),  // Op3: Third harmonic
                (4.0, 40.0, 0.0, 0.0),  // Op4: Fourth harmonic
                (7.0, 30.0, 0.0, 0.0),  // Op5: Woody overtone
                (11.0, 20.0, 0.0, 0.0), // Op6: High overtone
            ],
            envelopes: [
                (99.0, 70.0, 40.0, 60.0, 99.0, 70.0, 50.0, 0.0), // Op1
                (99.0, 75.0, 35.0, 65.0, 99.0, 60.0, 40.0, 0.0), // Op2
                (99.0, 80.0, 30.0, 70.0, 99.0, 50.0, 30.0, 0.0), // Op3
                (99.0, 85.0, 25.0, 75.0, 99.0, 40.0, 20.0, 0.0), // Op4
                (99.0, 90.0, 20.0, 80.0, 99.0, 30.0, 15.0, 0.0), // Op5
                (99.0, 95.0, 15.0, 85.0, 99.0, 20.0, 10.0, 0.0), // Op6
            ],
        },
        // HARPSICHORD
        Dx7Preset {
            name: "HARPSI",
            algorithm: 18,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Fundamental
                (2.0, 70.0, 0.0, 0.0), // Op2: Octave
                (4.0, 50.0, 0.0, 0.0), // Op3: Two octaves
                (8.0, 35.0, 0.0, 0.0), // Op4: Three octaves
                (1.0, 85.0, 5.0, 0.0), // Op5: Detuned fundamental
                (3.0, 40.0, 0.0, 2.0), // Op6: Pluck attack
            ],
            envelopes: [
                (99.0, 85.0, 45.0, 75.0, 99.0, 50.0, 30.0, 0.0), // Op1
                (99.0, 90.0, 40.0, 80.0, 99.0, 40.0, 25.0, 0.0), // Op2
                (99.0, 95.0, 35.0, 85.0, 99.0, 30.0, 20.0, 0.0), // Op3
                (99.0, 99.0, 30.0, 90.0, 99.0, 20.0, 15.0, 0.0), // Op4
                (99.0, 85.0, 45.0, 75.0, 99.0, 50.0, 30.0, 0.0), // Op5
                (99.0, 99.0, 20.0, 95.0, 99.0, 15.0, 5.0, 0.0),  // Op6
            ],
        },
        // WOODBLOCK - Percussive Sound
        Dx7Preset {
            name: "WOODBLOK",
            algorithm: 12,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Fundamental
                (3.5, 75.0, 0.0, 0.0),  // Op2: Woody overtone
                (7.2, 65.0, 0.0, 0.0),  // Op3: Higher woody tone
                (11.3, 55.0, 0.0, 0.0), // Op4: Even higher
                (1.0, 85.0, 0.0, 0.0),  // Op5: Fundamental support
                (2.1, 45.0, 0.0, 6.0),  // Op6: Noise with feedback
            ],
            envelopes: [
                (99.0, 99.0, 10.0, 99.0, 99.0, 20.0, 5.0, 0.0), // Op1
                (99.0, 99.0, 15.0, 99.0, 99.0, 15.0, 3.0, 0.0), // Op2
                (99.0, 99.0, 20.0, 99.0, 99.0, 10.0, 2.0, 0.0), // Op3
                (99.0, 99.0, 25.0, 99.0, 99.0, 8.0, 1.0, 0.0),  // Op4
                (99.0, 99.0, 10.0, 99.0, 99.0, 20.0, 5.0, 0.0), // Op5
                (99.0, 99.0, 5.0, 99.0, 60.0, 5.0, 0.0, 0.0),   // Op6
            ],
        },
        // XYLOPHONE
        Dx7Preset {
            name: "XYLO",
            algorithm: 22,
            operators: [
                (1.0, 99.0, 0.0, 0.0),  // Op1: Fundamental
                (3.0, 80.0, 0.0, 0.0),  // Op2: Third harmonic
                (5.0, 65.0, 0.0, 0.0),  // Op3: Fifth harmonic
                (7.0, 50.0, 0.0, 0.0),  // Op4: Seventh harmonic
                (9.0, 40.0, 0.0, 0.0),  // Op5: Ninth harmonic
                (11.0, 30.0, 0.0, 1.0), // Op6: Eleventh harmonic
            ],
            envelopes: [
                (99.0, 65.0, 35.0, 55.0, 99.0, 70.0, 50.0, 0.0), // Op1
                (99.0, 70.0, 30.0, 60.0, 99.0, 60.0, 40.0, 0.0), // Op2
                (99.0, 75.0, 25.0, 65.0, 99.0, 50.0, 30.0, 0.0), // Op3
                (99.0, 80.0, 20.0, 70.0, 99.0, 40.0, 25.0, 0.0), // Op4
                (99.0, 85.0, 15.0, 75.0, 99.0, 30.0, 20.0, 0.0), // Op5
                (99.0, 90.0, 10.0, 80.0, 99.0, 20.0, 15.0, 0.0), // Op6
            ],
        },
        // CLARINET
        Dx7Preset {
            name: "CLARINET",
            algorithm: 19,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Fundamental
                (3.0, 60.0, 0.0, 0.0), // Op2: Third harmonic
                (5.0, 40.0, 0.0, 0.0), // Op3: Fifth harmonic
                (7.0, 25.0, 0.0, 0.0), // Op4: Seventh harmonic
                (1.0, 85.0, 0.0, 0.0), // Op5: Fundamental support
                (2.0, 30.0, 0.0, 4.0), // Op6: Breath noise
            ],
            envelopes: [
                (65.0, 60.0, 65.0, 55.0, 99.0, 95.0, 90.0, 0.0), // Op1
                (70.0, 65.0, 60.0, 60.0, 99.0, 80.0, 70.0, 0.0), // Op2
                (75.0, 70.0, 55.0, 65.0, 99.0, 65.0, 55.0, 0.0), // Op3
                (80.0, 75.0, 50.0, 70.0, 99.0, 50.0, 40.0, 0.0), // Op4
                (65.0, 60.0, 65.0, 55.0, 99.0, 95.0, 90.0, 0.0), // Op5
                (90.0, 95.0, 45.0, 80.0, 50.0, 15.0, 10.0, 0.0), // Op6
            ],
        },
        // OBOE
        Dx7Preset {
            name: "OBOE",
            algorithm: 8,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Fundamental
                (2.0, 75.0, 0.0, 0.0), // Op2: Octave
                (3.0, 65.0, 0.0, 0.0), // Op3: Third harmonic
                (4.0, 55.0, 0.0, 0.0), // Op4: Fourth harmonic
                (5.0, 45.0, 0.0, 0.0), // Op5: Fifth harmonic
                (1.0, 60.0, 0.0, 5.0), // Op6: Reed noise
            ],
            envelopes: [
                (70.0, 65.0, 60.0, 60.0, 99.0, 90.0, 85.0, 0.0), // Op1
                (75.0, 70.0, 55.0, 65.0, 99.0, 80.0, 70.0, 0.0), // Op2
                (80.0, 75.0, 50.0, 70.0, 99.0, 70.0, 60.0, 0.0), // Op3
                (85.0, 80.0, 45.0, 75.0, 99.0, 60.0, 50.0, 0.0), // Op4
                (90.0, 85.0, 40.0, 80.0, 99.0, 50.0, 40.0, 0.0), // Op5
                (95.0, 99.0, 35.0, 85.0, 60.0, 20.0, 10.0, 0.0), // Op6
            ],
        },
        // TRUMPET
        Dx7Preset {
            name: "TRUMPET",
            algorithm: 21,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Fundamental
                (2.0, 85.0, 0.0, 0.0), // Op2: Octave
                (3.0, 70.0, 0.0, 0.0), // Op3: Third harmonic
                (4.0, 60.0, 0.0, 0.0), // Op4: Fourth harmonic
                (5.0, 50.0, 0.0, 0.0), // Op5: Fifth harmonic
                (6.0, 40.0, 0.0, 3.0), // Op6: Sixth harmonic with brass bite
            ],
            envelopes: [
                (75.0, 70.0, 60.0, 65.0, 99.0, 85.0, 80.0, 0.0), // Op1
                (80.0, 75.0, 55.0, 70.0, 99.0, 80.0, 70.0, 0.0), // Op2
                (85.0, 80.0, 50.0, 75.0, 99.0, 75.0, 65.0, 0.0), // Op3
                (90.0, 85.0, 45.0, 80.0, 99.0, 70.0, 60.0, 0.0), // Op4
                (95.0, 90.0, 40.0, 85.0, 99.0, 65.0, 55.0, 0.0), // Op5
                (99.0, 95.0, 35.0, 90.0, 99.0, 60.0, 50.0, 0.0), // Op6
            ],
        },
        // TUBA
        Dx7Preset {
            name: "TUBA",
            algorithm: 1,
            operators: [
                (1.0, 99.0, 0.0, 0.0), // Op1: Fundamental
                (0.5, 85.0, 0.0, 0.0), // Op2: Sub octave
                (2.0, 60.0, 0.0, 0.0), // Op3: Octave
                (3.0, 45.0, 0.0, 0.0), // Op4: Third harmonic
                (4.0, 30.0, 0.0, 0.0), // Op5: Fourth harmonic
                (1.0, 70.0, 0.0, 2.0), // Op6: Breath with feedback
            ],
            envelopes: [
                (60.0, 55.0, 60.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op1
                (60.0, 55.0, 60.0, 50.0, 99.0, 90.0, 85.0, 0.0), // Op2
                (65.0, 60.0, 55.0, 55.0, 99.0, 80.0, 70.0, 0.0), // Op3
                (70.0, 65.0, 50.0, 60.0, 99.0, 70.0, 60.0, 0.0), // Op4
                (75.0, 70.0, 45.0, 65.0, 99.0, 60.0, 50.0, 0.0), // Op5
                (80.0, 85.0, 40.0, 70.0, 70.0, 30.0, 20.0, 0.0), // Op6
            ],
        },
        // SPACE VOICE - Ethereal Pad
        Dx7Preset {
            name: "SPACE",
            algorithm: 26,
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
        },
        // GAMELAN - Metallic Percussion
        Dx7Preset {
            name: "GAMELAN",
            algorithm: 13,
            operators: [
                (1.0, 99.0, 0.0, 0.0),   // Op1: Fundamental
                (3.14, 75.0, 0.0, 0.0),  // Op2: Inharmonic
                (5.67, 65.0, 0.0, 0.0),  // Op3: Inharmonic
                (8.23, 55.0, 0.0, 0.0),  // Op4: Inharmonic
                (11.41, 45.0, 0.0, 0.0), // Op5: Inharmonic
                (1.0, 85.0, 0.0, 1.0),   // Op6: Fundamental with feedback
            ],
            envelopes: [
                (99.0, 40.0, 25.0, 30.0, 99.0, 80.0, 70.0, 0.0), // Op1
                (99.0, 45.0, 20.0, 35.0, 99.0, 70.0, 60.0, 0.0), // Op2
                (99.0, 50.0, 15.0, 40.0, 99.0, 60.0, 50.0, 0.0), // Op3
                (99.0, 55.0, 10.0, 45.0, 99.0, 50.0, 40.0, 0.0), // Op4
                (99.0, 60.0, 8.0, 50.0, 99.0, 40.0, 30.0, 0.0),  // Op5
                (99.0, 40.0, 25.0, 30.0, 99.0, 80.0, 70.0, 0.0), // Op6
            ],
        },
    ]
}
