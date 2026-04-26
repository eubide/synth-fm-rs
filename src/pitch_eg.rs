//! DX7 Pitch Envelope Generator using ROM tables.
//!
//! Independent from the operator amplitude EG. Produces a pitch offset (in
//! semitones) that is summed to the voice's note frequency.
//!
//! DX7 conventions:
//! - 4 rates (0-99) and 4 levels (0-99) as user-facing parameters.
//! - Level 50 → no pitch offset; tabulated levels reach roughly ±4 octaves.
//! - Stage 1-3 run automatically on note-on; release stage runs on note-off.
//! - When `enabled` is false the EG is bypassed and outputs 0 semitones.
//!
//! Internally we model the Dexed/MSFA approach: linear approach in log-freq
//! space (`level += inc` per sample) instead of the previous exponential
//! approach. The increment and target come from ROM tables transcribed from
//! `pitchenv.cc` in MSFA.

/// DX7 ROM `pitchenv_rate[100]`. Indexes the patch rate parameter (0..99) and
/// produces a raw step that is multiplied by `unit_` to get the per-sample
/// increment.
const PITCHENV_RATE: [u8; 100] = [
    1, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 16,
    16, 17, 18, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 30, 31, 33, 34, 36, 37, 38, 39, 41, 42,
    44, 46, 47, 49, 51, 53, 54, 56, 58, 60, 62, 64, 66, 68, 70, 72, 74, 76, 79, 82, 85, 88, 91, 94,
    98, 102, 106, 110, 115, 120, 125, 130, 135, 141, 147, 153, 159, 165, 171, 178, 185, 193, 202,
    211, 232, 243, 254, 255,
];

/// DX7 ROM `pitchenv_tab[100]`. Indexes the patch level parameter (0..99) and
/// returns a signed byte; `(byte) << 19` is a log-freq Q24 amount (1 << 24 =
/// 1 octave = 12 semitones), so each unit ≈ 12/32 = 0.375 semitones.
const PITCHENV_TAB: [i8; 100] = [
    -128, -116, -104, -95, -85, -76, -68, -61, -56, -52, -49, -46, -43, -41, -39, -37, -35, -33,
    -32, -31, -30, -29, -28, -27, -26, -25, -24, -23, -22, -21, -20, -19, -18, -17, -16, -15, -14,
    -13, -12, -11, -10, -9, -8, -7, -6, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35,
    38, 40, 43, 46, 49, 53, 58, 65, 73, 82, 92, 103, 115, 127,
];

/// 1 ROM unit (i8 entry) is `1 << 19` log-freq units; an octave is `1 << 24`,
/// so each unit corresponds to `12 << 19 / (1 << 24) = 0.375` semitones.
const SEMITONES_PER_TAB_UNIT: f32 = 12.0 / 32.0;

/// Per-sample base increment factor, derived from the empirical
/// `unit_ = N * (1 << 24) / (21.3 * sample_rate)` in MSFA `pitchenv.cc`.
/// Removing the `N` (we process per sample, not per block) and converting to
/// semitones yields `12 / (21.3 * sr)` per ROM rate unit.
const PITCH_RATE_UNIT_DIVIDER: f32 = 21.3;

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

    current_semitones: f32,
    target_semitones: f32,
    inc_per_sample: f32, // always non-negative; sign comes from `rising`
    rising: bool,
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

/// Convert a DX7 level parameter (0..99) to semitones using the ROM table.
fn level_to_semitones(level: f32) -> f32 {
    let idx = (level.round().clamp(0.0, 99.0)) as usize;
    PITCHENV_TAB[idx] as f32 * SEMITONES_PER_TAB_UNIT
}

/// Convert a DX7 rate parameter (0..99) to per-sample increment in semitones.
fn rate_to_semitones_per_sample(rate: f32, sample_rate: f32) -> f32 {
    let idx = (rate.round().clamp(0.0, 99.0)) as usize;
    PITCHENV_RATE[idx] as f32 * 12.0 / (PITCH_RATE_UNIT_DIVIDER * sample_rate)
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
            current_semitones: 0.0,
            target_semitones: 0.0,
            inc_per_sample: 0.0,
            rising: false,
            stage: PitchEgStage::Idle,
            sample_rate,
        }
    }

    pub fn trigger(&mut self) {
        if !self.enabled {
            self.current_semitones = 0.0;
            self.target_semitones = 0.0;
            self.stage = PitchEgStage::Idle;
            return;
        }
        // DX7 starts the pitch EG from level4 (the rest position) and ramps to level1.
        self.current_semitones = level_to_semitones(self.level4);
        self.stage = PitchEgStage::Stage1;
        let target = level_to_semitones(self.level1);
        let rate = rate_to_semitones_per_sample(self.rate1, self.sample_rate);
        self.set_target(target, rate);
    }

    pub fn release(&mut self) {
        if !self.enabled || self.stage == PitchEgStage::Idle {
            return;
        }
        self.stage = PitchEgStage::Stage4;
        let target = level_to_semitones(self.level4);
        let rate = rate_to_semitones_per_sample(self.rate4, self.sample_rate);
        self.set_target(target, rate);
    }

    pub fn reset(&mut self) {
        self.current_semitones = 0.0;
        self.target_semitones = 0.0;
        self.inc_per_sample = 0.0;
        self.rising = false;
        self.stage = PitchEgStage::Idle;
    }

    /// Process one sample. Returns the pitch offset in **semitones** (not Hz).
    /// 0.0 means no offset.
    pub fn process(&mut self) -> f32 {
        if !self.enabled || self.stage == PitchEgStage::Idle {
            return 0.0;
        }

        if self.rising {
            self.current_semitones += self.inc_per_sample;
            if self.current_semitones >= self.target_semitones {
                self.current_semitones = self.target_semitones;
                self.advance_stage();
            }
        } else {
            self.current_semitones -= self.inc_per_sample;
            if self.current_semitones <= self.target_semitones {
                self.current_semitones = self.target_semitones;
                self.advance_stage();
            }
        }

        self.current_semitones
    }

    fn advance_stage(&mut self) {
        match self.stage {
            PitchEgStage::Stage1 => {
                self.stage = PitchEgStage::Stage2;
                let target = level_to_semitones(self.level2);
                let rate = rate_to_semitones_per_sample(self.rate2, self.sample_rate);
                self.set_target(target, rate);
            }
            PitchEgStage::Stage2 => {
                self.stage = PitchEgStage::Stage3;
                let target = level_to_semitones(self.level3);
                let rate = rate_to_semitones_per_sample(self.rate3, self.sample_rate);
                self.set_target(target, rate);
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

    fn set_target(&mut self, target: f32, rate: f32) {
        self.target_semitones = target;
        self.inc_per_sample = rate;
        self.rising = target > self.current_semitones;
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
        peg.level1 = 99.0; // +4 octaves target via ROM
        peg.level4 = 50.0; // start at neutral
        peg.rate1 = 99.0; // fast
        peg.trigger();

        let mut peak = 0.0_f32;
        for _ in 0..2048 {
            peak = peak.max(peg.process().abs());
        }
        assert!(
            peak > 1.0,
            "envelope should produce semitone offset, got {peak}"
        );
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
        assert!(
            last.abs() < 0.5,
            "should ramp back near 0 semitones, got {last}"
        );
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
        // pitchenv_tab[99] = 127 → 127 * 0.375 = 47.625 semitones
        assert!((last - 47.625).abs() < 1.0, "expected ~+47.6, got {last}");
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
        // pitchenv_tab[0] = -128 → -128 * 0.375 = -48 semitones exactly
        assert!((last + 48.0).abs() < 1.0, "expected ~-48, got {last}");
    }

    #[test]
    fn rate_zero_holds_initial_position() {
        // PITCHENV_RATE[0] = 1 (not 0) — even the slowest DX7 rate creeps a bit
        // every sample, so over 1024 samples we expect a tiny but non-zero drift.
        let mut peg = make_default();
        peg.enabled = true;
        peg.rate1 = 0.0;
        peg.level1 = 99.0;
        peg.level4 = 50.0;
        peg.trigger();
        let mut last = 0.0;
        for _ in 0..1024 {
            last = peg.process();
        }
        assert!(last.abs() < 5.0, "rate 0 should creep slowly, got {last}");
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

    #[test]
    fn rom_tables_have_expected_anchors() {
        // Smoke test on the ROM tables themselves so a transcription bug surfaces fast.
        assert_eq!(PITCHENV_TAB[50], 0); // neutral
        assert_eq!(PITCHENV_TAB[0], -128); // -4 octaves
        assert_eq!(PITCHENV_TAB[99], 127); // +4 octaves
        assert_eq!(PITCHENV_RATE[99], 255); // fastest
        assert!(PITCHENV_RATE[0] >= 1); // even rate 0 creeps
    }
}
