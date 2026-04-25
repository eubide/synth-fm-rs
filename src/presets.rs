use crate::fm_synth::SynthEngine;

#[derive(Clone)]
#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub struct Dx7Preset {
    pub name: String,
    pub collection: String,
    pub algorithm: u8,
    pub operators: [(f32, f32, f32, f32); 6], // (ratio, level, detune, feedback)
    pub envelopes: [(f32, f32, f32, f32, f32, f32, f32, f32); 6], // (r1-r4, l1-l4)
    pub master_tune: Option<f32>,        // ±150 cents
    pub mono_mode: Option<bool>,         // false = poly, true = mono
    pub pitch_bend_range: Option<f32>,   // 0-12 semitones
    pub portamento_enable: Option<bool>, // portamento on/off
    pub portamento_time: Option<f32>,    // 0-99
}

impl Dx7Preset {
    pub fn apply_to_synth(&self, synth: &mut SynthEngine) {
        synth.set_algorithm(self.algorithm);
        synth.set_preset_name(self.name.clone());

        for voice in synth.voices_mut() {
            for (i, op) in voice.operators.iter_mut().enumerate() {
                let (ratio, level, detune, feedback) = self.operators[i];
                op.frequency_ratio = ratio;
                op.output_level = level;
                op.detune = detune;
                op.feedback = feedback;

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
