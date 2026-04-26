use crate::optimization::OPTIMIZATION_TABLES;

/// DX7 Pitch Envelope Generator.
///
/// Independent from the operator amplitude EG. Produces a pitch offset (in
/// semitones) that is summed to the voice's note frequency.
///
/// DX7 conventions:
/// - 4 rates (0-99) and 4 levels (0-99).
/// - Level 50 → no pitch offset; 0 → -4 octaves; 99 → +4 octaves.
/// - Stage 1-3 run automatically on note-on; release stage runs on note-off.
/// - When `enabled` is false the EG is bypassed and outputs 0 semitones.
#[derive(Debug, Clone)]
pub struct PitchEg {
    pub enabled: bool,
    pub rate1: f32,
    pub rate2: f32,
    pub rate3: f32,
    pub rate4: f32,
    pub level1: f32,
    pub level2: f32,
    pub level3: f32,
    pub level4: f32,

    current_level: f32, // current normalized level in 0..99 space
    target_level: f32,
    rate: f32, // per-sample approach rate (0..1)
    stage: PitchEgStage,
    sample_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PitchEgStage {
    Idle,
    Stage1,
    Stage2,
    Stage3, // sustain
    Stage4, // release
}

impl PitchEg {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            enabled: false,
            rate1: 99.0,
            rate2: 99.0,
            rate3: 99.0,
            rate4: 99.0,
            level1: 50.0,
            level2: 50.0,
            level3: 50.0,
            level4: 50.0,
            current_level: 50.0,
            target_level: 50.0,
            rate: 0.0,
            stage: PitchEgStage::Idle,
            sample_rate,
        }
    }

    pub fn trigger(&mut self) {
        if !self.enabled {
            self.current_level = 50.0;
            self.target_level = 50.0;
            self.stage = PitchEgStage::Idle;
            return;
        }
        // DX7 starts the pitch EG from level4 (the rest position) and ramps to level1.
        self.current_level = self.level4;
        self.stage = PitchEgStage::Stage1;
        self.target_level = self.level1;
        self.rate = self.calc_rate(self.rate1);
    }

    pub fn release(&mut self) {
        if !self.enabled || self.stage == PitchEgStage::Idle {
            return;
        }
        self.stage = PitchEgStage::Stage4;
        self.target_level = self.level4;
        self.rate = self.calc_rate(self.rate4);
    }

    pub fn reset(&mut self) {
        self.current_level = if self.enabled { self.level4 } else { 50.0 };
        self.target_level = self.current_level;
        self.stage = PitchEgStage::Idle;
        self.rate = 0.0;
    }

    /// Process one sample. Returns the pitch offset in **semitones** (not Hz).
    /// 0.0 means no offset.
    pub fn process(&mut self) -> f32 {
        if !self.enabled || self.stage == PitchEgStage::Idle {
            return 0.0;
        }

        let distance = self.target_level - self.current_level;
        if distance.abs() > 0.0001 {
            let approach = (self.rate * 6.908).clamp(0.0000001, 0.5);
            self.current_level += distance * approach;
            if (self.target_level - self.current_level).abs() < 0.05 {
                self.current_level = self.target_level;
                self.advance_stage();
            }
        } else {
            self.current_level = self.target_level;
            self.advance_stage();
        }

        // Convert level (0..99, 50=neutral) to semitones. ±4 octaves = ±48 semitones
        // means each level step is 48/49 ≈ 0.98 semitones away from neutral.
        ((self.current_level - 50.0) / 49.0) * 48.0
    }

    fn advance_stage(&mut self) {
        match self.stage {
            PitchEgStage::Stage1 => {
                self.stage = PitchEgStage::Stage2;
                self.target_level = self.level2;
                self.rate = self.calc_rate(self.rate2);
            }
            PitchEgStage::Stage2 => {
                self.stage = PitchEgStage::Stage3;
                self.target_level = self.level3;
                self.rate = self.calc_rate(self.rate3);
            }
            PitchEgStage::Stage3 => {
                // Sustain: hold until release()
            }
            PitchEgStage::Stage4 => {
                self.stage = PitchEgStage::Idle;
            }
            PitchEgStage::Idle => {}
        }
    }

    fn calc_rate(&self, rate_value: f32) -> f32 {
        if rate_value == 0.0 {
            return 0.0;
        }
        // Reuse the DX7 amplitude-EG rate table, scaled to seconds.
        OPTIMIZATION_TABLES.dx7_rate_to_multiplier(rate_value as u8) / self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44_100.0;

    fn make_default() -> PitchEg {
        PitchEg::new(SR)
    }

    #[test]
    fn new_pitch_eg_is_disabled_and_idle() {
        let mut peg = make_default();
        assert!(!peg.enabled);
        assert_eq!(peg.process(), 0.0);
    }

    #[test]
    fn disabled_trigger_resets_state_and_outputs_zero() {
        let mut peg = make_default();
        peg.enabled = false;
        peg.trigger();
        assert_eq!(peg.process(), 0.0);
    }

    #[test]
    fn enabled_trigger_starts_pitch_envelope() {
        let mut peg = make_default();
        peg.enabled = true;
        peg.level1 = 99.0; // +4 octaves target
        peg.level4 = 50.0; // start at neutral
        peg.rate1 = 99.0; // fast
        peg.trigger();

        let mut peak = 0.0_f32;
        for _ in 0..2048 {
            peak = peak.max(peg.process().abs());
        }
        assert!(peak > 1.0, "envelope should produce semitone offset, got {peak}");
    }

    #[test]
    fn release_drives_envelope_back_to_neutral_when_level4_is_neutral() {
        let mut peg = make_default();
        peg.enabled = true;
        peg.level1 = 80.0;
        peg.level2 = 80.0;
        peg.level3 = 80.0;
        peg.level4 = 50.0;
        peg.rate1 = 99.0;
        peg.rate4 = 99.0;
        peg.trigger();

        for _ in 0..1024 {
            peg.process();
        }
        peg.release();
        let mut last = 0.0;
        for _ in 0..(SR as usize) {
            last = peg.process();
            if last.abs() < 0.01 {
                break;
            }
        }
        assert!(last.abs() < 0.5, "should ramp back near 0 semitones, got {last}");
    }

    #[test]
    fn release_when_disabled_is_noop() {
        let mut peg = make_default();
        peg.enabled = false;
        peg.release();
        assert_eq!(peg.process(), 0.0);
    }

    #[test]
    fn release_when_idle_is_noop() {
        let mut peg = make_default();
        peg.enabled = true;
        // No trigger → still Idle
        peg.release();
        assert_eq!(peg.process(), 0.0);
    }

    #[test]
    fn reset_returns_to_baseline() {
        let mut peg = make_default();
        peg.enabled = true;
        peg.trigger();
        for _ in 0..256 {
            peg.process();
        }
        peg.reset();
        // After reset, output should be 0 (idle stage)
        assert_eq!(peg.process(), 0.0);
    }

    #[test]
    fn level_50_means_zero_semitones() {
        let mut peg = make_default();
        peg.enabled = true;
        peg.level1 = 50.0;
        peg.level2 = 50.0;
        peg.level3 = 50.0;
        peg.level4 = 50.0;
        peg.rate1 = 99.0;
        peg.trigger();
        for _ in 0..1024 {
            let s = peg.process();
            assert!(s.abs() < 0.5, "all levels=50 should produce ~0, got {s}");
        }
    }

    #[test]
    fn level_99_at_sustain_targets_about_plus_four_octaves() {
        let mut peg = make_default();
        peg.enabled = true;
        peg.level1 = 99.0;
        peg.level2 = 99.0;
        peg.level3 = 99.0;
        peg.rate1 = 99.0;
        peg.rate2 = 99.0;
        peg.rate3 = 99.0;
        peg.trigger();
        let mut last = 0.0;
        for _ in 0..(SR as usize) {
            last = peg.process();
        }
        // (99-50)/49 * 48 = 48 semitones = +4 octaves
        assert!((last - 48.0).abs() < 1.0, "expected ~+48, got {last}");
    }

    #[test]
    fn level_zero_targets_minus_four_octaves() {
        let mut peg = make_default();
        peg.enabled = true;
        peg.level1 = 0.0;
        peg.level2 = 0.0;
        peg.level3 = 0.0;
        peg.rate1 = 99.0;
        peg.rate2 = 99.0;
        peg.rate3 = 99.0;
        peg.trigger();
        let mut last = 0.0;
        for _ in 0..(SR as usize) {
            last = peg.process();
        }
        // (0-50)/49 * 48 = -48.97 ≈ -48
        assert!((last + 48.0).abs() < 1.5, "expected ~-48, got {last}");
    }

    #[test]
    fn rate_zero_holds_initial_position() {
        let mut peg = make_default();
        peg.enabled = true;
        peg.rate1 = 0.0;
        peg.level1 = 99.0;
        peg.level4 = 50.0;
        peg.trigger();
        // With rate=0 the envelope should not advance toward level1
        let mut last = 0.0;
        for _ in 0..1024 {
            last = peg.process();
        }
        assert!(last.abs() < 5.0, "rate 0 should not advance, got {last}");
    }

    #[test]
    fn full_lifecycle_traverses_stages() {
        let mut peg = make_default();
        peg.enabled = true;
        peg.rate1 = 99.0;
        peg.rate2 = 99.0;
        peg.rate3 = 99.0;
        peg.rate4 = 99.0;
        peg.level1 = 70.0;
        peg.level2 = 60.0;
        peg.level3 = 50.0;
        peg.level4 = 50.0;
        peg.trigger();
        for _ in 0..(SR as usize / 2) {
            peg.process();
        }
        peg.release();
        for _ in 0..(SR as usize / 2) {
            peg.process();
        }
        // After full lifecycle the EG returns to Idle and output is 0.
        assert_eq!(peg.process(), 0.0);
    }
}
