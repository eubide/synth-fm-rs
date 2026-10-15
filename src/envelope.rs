//! DX7 envelope generator, ported from MSFA / Dexed `env.cc`.
//!
//! The level lives in a log domain (Q24: `1 << 24` per doubling of amplitude),
//! so stage levels sit ~0.75 dB apart per step and decays are linear in dB, as
//! on the hardware. The operator's output level, keyboard level scaling and
//! velocity enter as `outlevel` and offset every stage target.

/// Dexed advances the envelope once per block of 64 samples at 44.1 kHz; the
/// rate increments and the static-stage table are expressed in those units.
const BLOCK: f64 = 64.0;
const REFERENCE_RATE: f64 = 44_100.0;
const ONE_DOUBLING: f64 = (1 << 24) as f64;
/// Level of an operator at output level 99 and envelope level 99; maps to
/// amplitude 1.0.
const FULL_SCALE: f64 = 15.0 * ONE_DOUBLING;
/// Rising stages jump straight to this level (about 50 dB under full scale)
/// before curving up — the DX7's characteristic attack.
const ATTACK_JUMP: f64 = (1716 << 16) as f64;
/// Rising stages approach this ceiling; the remaining whole doublings scale
/// the step, which bends the attack.
const ATTACK_CEILING: f64 = 17.0 * ONE_DOUBLING;

const LEVEL_LUT: [i32; 20] = [
    0, 5, 9, 13, 17, 20, 23, 25, 27, 29, 31, 33, 35, 37, 39, 41, 42, 43, 45, 46,
];

/// Samples at 44.1 kHz that a stage lasts when it does not move the level,
/// for rates 0..=76. Measured on two TF1 units by the MSFA authors; faster
/// rates use `20 * (99 - rate)`.
const STATICS: [i32; 77] = [
    1764000, 1764000, 1411200, 1411200, 1190700, 1014300, 992250, 882000, 705600, 705600, 584325,
    507150, 502740, 441000, 418950, 352800, 308700, 286650, 253575, 220500, 220500, 176400, 145530,
    145530, 125685, 110250, 110250, 88200, 88200, 74970, 61740, 61740, 55125, 48510, 44100, 37485,
    31311, 30870, 27562, 27562, 22050, 18522, 17640, 15435, 14112, 13230, 11025, 9261, 9261, 7717,
    6615, 6615, 5512, 5512, 4410, 3969, 3969, 3439, 2866, 2690, 2249, 1984, 1896, 1808, 1411, 1367,
    1234, 1146, 926, 837, 837, 705, 573, 573, 529, 441, 441,
];

/// DX7 output/envelope level (0..99) to the hardware's log scale: 0.75 dB per
/// step from 20 up, a steeper table below.
pub fn scale_out_level(level: i32) -> i32 {
    if level >= 20 {
        28 + level
    } else {
        LEVEL_LUT[level.max(0) as usize]
    }
}

#[derive(Debug, Clone)]
pub struct Envelope {
    pub rate1: f32,
    pub rate2: f32,
    pub rate3: f32,
    pub rate4: f32,

    pub level1: f32,
    pub level2: f32,
    pub level3: f32,
    pub level4: f32,

    level: f64,
    target: f64,
    rising: bool,
    /// 0..=2 key-down segments, 3 release, 4 finished.
    stage: usize,
    /// Level change per sample.
    inc: f64,
    /// Samples left in a stage that holds its level.
    static_count: f64,
    key_down: bool,
    triggered: bool,
    /// Output level, keyboard level scaling and velocity, in microsteps
    /// (32 per output-level step).
    outlevel: i32,
    /// Keyboard rate scaling, in qrate units (4 per doubling of speed).
    rate_scaling: i32,
    sample_rate: f32,
    output: f32,
}

impl Envelope {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            rate1: 99.0,
            rate2: 50.0,
            rate3: 35.0,
            rate4: 50.0,

            level1: 99.0,
            level2: 75.0,
            level3: 50.0,
            level4: 0.0,

            level: 0.0,
            target: 0.0,
            rising: false,
            stage: 4,
            inc: 0.0,
            static_count: 0.0,
            key_down: false,
            triggered: false,
            outlevel: 0,
            rate_scaling: 0,
            sample_rate,
            output: 0.0,
        }
    }

    pub fn trigger(&mut self, outlevel: i32, rate_scaling: i32) {
        self.triggered = true;
        self.outlevel = outlevel;
        self.rate_scaling = rate_scaling;
        self.level = 0.0;
        self.key_down = true;
        self.advance(0);
    }

    pub fn release(&mut self) {
        if self.triggered && self.key_down {
            self.key_down = false;
            self.advance(3);
        }
    }

    /// Moves the current level with `outlevel` so a held note follows a live
    /// edit of output level, keyboard scaling or velocity sensitivity.
    pub fn set_outlevel(&mut self, outlevel: i32) {
        let delta = (outlevel - self.outlevel) as f64 * 65536.0;
        self.outlevel = outlevel;
        if !self.triggered {
            return;
        }
        self.level = (self.level + delta).max(0.0);
        if self.stage < 4 {
            self.target = self.target_for(self.segment(self.stage).1);
            self.rising = self.target > self.level;
        }
    }

    /// Advances one sample and returns the amplitude (1.0 = full scale).
    pub fn process(&mut self) -> f32 {
        if !self.is_active() {
            self.output = 0.0;
            return 0.0;
        }

        if self.static_count > 0.0 {
            self.static_count -= 1.0;
            if self.static_count <= 0.0 {
                self.static_count = 0.0;
                self.advance(self.stage + 1);
            }
        }

        let moving = self.stage < 3 || (self.stage < 4 && !self.key_down);
        if moving && self.static_count == 0.0 {
            if self.rising {
                self.level = self.level.max(ATTACK_JUMP);
                self.level += ((ATTACK_CEILING - self.level) / ONE_DOUBLING).floor() * self.inc;
                if self.level >= self.target {
                    self.level = self.target;
                    self.advance(self.stage + 1);
                }
            } else {
                self.level -= self.inc;
                if self.level <= self.target {
                    self.level = self.target;
                    self.advance(self.stage + 1);
                }
            }
        }

        self.output = ((self.level - FULL_SCALE) / ONE_DOUBLING).exp2() as f32;
        self.output
    }

    /// A note ends once the release reaches its target, unless L4 keeps it
    /// sounding.
    pub fn is_active(&self) -> bool {
        self.triggered && (self.stage < 4 || self.level4 > 0.0)
    }

    /// Amplitude produced by the last `process` call, in 0..=1 at full scale.
    pub fn current_output(&self) -> f32 {
        self.output
    }

    pub fn reset(&mut self) {
        self.triggered = false;
        self.key_down = false;
        self.stage = 4;
        self.level = 0.0;
        self.static_count = 0.0;
        self.output = 0.0;
    }

    fn segment(&self, ix: usize) -> (i32, i32) {
        let (rate, level) = match ix {
            0 => (self.rate1, self.level1),
            1 => (self.rate2, self.level2),
            2 => (self.rate3, self.level3),
            _ => (self.rate4, self.level4),
        };
        (
            rate.round().clamp(0.0, 99.0) as i32,
            level.round().clamp(0.0, 99.0) as i32,
        )
    }

    fn target_for(&self, level: i32) -> f64 {
        let actual = ((scale_out_level(level) >> 1) << 6) + self.outlevel - 4256;
        (actual.max(16) << 16) as f64
    }

    fn advance(&mut self, ix: usize) {
        self.stage = ix;
        if ix >= 4 {
            return;
        }
        let (rate, level) = self.segment(ix);
        self.target = self.target_for(level);
        self.rising = self.target > self.level;

        let holds = self.target == self.level || (ix == 0 && level == 0);
        self.static_count = if holds {
            let static_rate = (rate + self.rate_scaling).min(99);
            let mut samples = if static_rate < 77 {
                STATICS[static_rate as usize]
            } else {
                20 * (99 - static_rate)
            };
            if static_rate < 77 && ix == 0 && level == 0 {
                samples /= 20;
            }
            // Dexed scales this by 44100 / sr, which shortens holds above
            // 44.1 kHz; the table is in samples, so the ratio goes the other way.
            samples as f64 * self.sample_rate as f64 / REFERENCE_RATE
        } else {
            0.0
        };

        let qrate = (((rate * 41) >> 6) + self.rate_scaling).min(63);
        let inc_per_block = ((4 + (qrate & 3)) << (2 + 6 + (qrate >> 2))) as f64;
        self.inc = inc_per_block / BLOCK * REFERENCE_RATE / self.sample_rate as f64;
    }

    #[cfg(test)]
    fn log_level(&self) -> f64 {
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44_100.0;
    /// `outlevel` for output level 99 with no scaling (`scale_out_level(99) << 5`).
    const OL_99: i32 = 4064;

    fn envelope(sr: f32, rates: [f32; 4], levels: [f32; 4]) -> Envelope {
        let mut env = Envelope::new(sr);
        [env.rate1, env.rate2, env.rate3, env.rate4] = rates;
        [env.level1, env.level2, env.level3, env.level4] = levels;
        env
    }

    /// Level after Dexed's block `block` (it updates once per 64 samples).
    fn level_after_block(env: &mut Envelope, block: usize, done: &mut usize) -> f64 {
        let until = (block + 1) * 64;
        while *done < until {
            env.process();
            *done += 1;
        }
        env.log_level()
    }

    /// Reference values below come from Dexed's `env.cc` built standalone and
    /// sampled once per block (`Env::getsample`).
    fn assert_matches_dexed(mut env: Envelope, expected: &[(usize, i64)], tolerance: f64) {
        let mut done = 0;
        for &(block, dexed) in expected {
            let ours = level_after_block(&mut env, block, &mut done);
            assert!(
                (ours - dexed as f64).abs() <= tolerance,
                "block {block}: ours {ours}, Dexed {dexed}"
            );
        }
    }

    #[test]
    fn attack_curve_matches_dexed() {
        let mut env = envelope(SR, [60.0, 50.0, 50.0, 50.0], [99.0, 80.0, 80.0, 0.0]);
        env.trigger(OL_99, 0);
        // Per-sample stepping crosses doubling boundaries earlier than Dexed's
        // per-block update; allow 0.05 doublings (0.3 dB).
        assert_matches_dexed(
            env,
            &[
                (0, 120324096),
                (10, 179306496),
                (50, 248512512),
                (100, 235405312),
                (200, 213909504),
                (399, 213909504),
            ],
            0.05 * ONE_DOUBLING,
        );
    }

    /// Dexed finishes an instant attack inside one block and starts the decay
    /// on the next; per-sample stepping starts it a few dozen samples sooner.
    /// Allow one block of decay (rate 40 → qrate 25 → 5 << 14 per block).
    const ONE_DECAY_BLOCK: f64 = (5 << 14) as f64;

    #[test]
    fn decay_is_linear_in_db_like_dexed() {
        let mut env = envelope(SR, [99.0, 40.0, 99.0, 50.0], [99.0, 50.0, 50.0, 0.0]);
        env.trigger(OL_99, 0);
        assert_matches_dexed(
            env,
            &[
                (0, 251658240),
                (100, 243466240),
                (500, 210698240),
                (1000, 169738240),
                (1999, 150994944),
            ],
            ONE_DECAY_BLOCK,
        );
    }

    #[test]
    fn decay_timing_scales_with_sample_rate() {
        let mut env = envelope(48_000.0, [99.0, 40.0, 99.0, 50.0], [99.0, 50.0, 50.0, 0.0]);
        env.trigger(OL_99, 0);
        assert_matches_dexed(env, &[(500, 214026740), (1088, 169772096)], ONE_DECAY_BLOCK);
    }

    #[test]
    fn release_falls_to_the_floor_and_ends_the_note() {
        let mut env = envelope(SR, [99.0, 99.0, 99.0, 50.0], [99.0, 99.0, 99.0, 0.0]);
        env.trigger(3136, 0);
        let mut done = 0;
        level_after_block(&mut env, 99, &mut done);
        env.release();
        let mut done = 0;
        for &(block, dexed) in &[(1, 190316544_i64), (200, 138149888), (500, 59506688)] {
            let ours = level_after_block(&mut env, block, &mut done);
            assert!(
                (ours - dexed as f64).abs() <= 1.0,
                "block {block}: {ours} vs {dexed}"
            );
        }
        level_after_block(&mut env, 1100, &mut done);
        assert_eq!(env.log_level(), 1048576.0);
        assert!(!env.is_active());
        assert_eq!(env.process(), 0.0);
    }

    #[test]
    fn a_stage_that_holds_its_level_waits_the_measured_time() {
        // L1 == L2: stage 2 keeps the level for STATICS[30] samples, then decays.
        let mut env = envelope(SR, [99.0, 30.0, 99.0, 99.0], [99.0, 99.0, 0.0, 0.0]);
        env.trigger(OL_99, 0);
        let mut done = 0;
        assert_eq!(level_after_block(&mut env, 963, &mut done), 251658240.0);
        assert!(level_after_block(&mut env, 966, &mut done) < 251658240.0);
    }

    #[test]
    fn full_scale_maps_to_unit_amplitude_and_levels_are_in_db() {
        let mut env = envelope(SR, [99.0, 99.0, 99.0, 99.0], [99.0, 99.0, 50.0, 0.0]);
        env.trigger(OL_99, 0);
        let peak = (0..64).map(|_| env.process()).fold(0.0, f32::max);
        assert!((peak - 1.0).abs() < 1e-6, "peak {peak}");

        for _ in 0..44_100 {
            env.process();
        }
        let sustain_db = 20.0 * env.current_output().log10();
        // L3 = 50: ((127 >> 1) - (78 >> 1)) * 64 microsteps = 6 doublings down.
        assert!((sustain_db + 36.12).abs() < 0.05, "sustain {sustain_db} dB");
    }

    #[test]
    fn rate_scaling_speeds_up_every_stage() {
        let mut plain = envelope(SR, [99.0, 30.0, 99.0, 99.0], [99.0, 0.0, 0.0, 0.0]);
        let mut scaled = plain.clone();
        plain.trigger(OL_99, 0);
        scaled.trigger(OL_99, 8);
        for _ in 0..4096 {
            plain.process();
            scaled.process();
        }
        assert!(scaled.log_level() < plain.log_level());
    }

    #[test]
    fn set_outlevel_moves_a_held_note() {
        let mut env = envelope(SR, [99.0, 99.0, 99.0, 99.0], [99.0, 99.0, 99.0, 0.0]);
        env.trigger(OL_99, 0);
        for _ in 0..256 {
            env.process();
        }
        env.set_outlevel(OL_99 - 256);
        for _ in 0..256 {
            env.process();
        }
        assert!((env.current_output() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn new_envelope_is_idle_and_outputs_zero() {
        let mut env = Envelope::new(SR);
        assert!(!env.is_active());
        assert_eq!(env.process(), 0.0);
        assert_eq!(env.current_output(), 0.0);
    }

    #[test]
    fn release_when_idle_is_noop() {
        let mut env = Envelope::new(SR);
        env.release();
        assert!(!env.is_active());
        assert_eq!(env.process(), 0.0);
    }

    #[test]
    fn nonzero_level4_keeps_the_note_alive_after_release() {
        let mut env = envelope(SR, [99.0, 99.0, 99.0, 99.0], [99.0, 99.0, 99.0, 50.0]);
        env.trigger(OL_99, 0);
        env.release();
        for _ in 0..44_100 {
            env.process();
        }
        assert!(env.is_active());
        assert!(env.current_output() > 0.0);
    }

    #[test]
    fn reset_returns_to_initial_state() {
        let mut env = Envelope::new(SR);
        env.trigger(OL_99, 0);
        for _ in 0..256 {
            env.process();
        }
        env.reset();
        assert!(!env.is_active());
        assert_eq!(env.process(), 0.0);
    }

    #[test]
    fn scale_out_level_uses_the_table_below_20() {
        assert_eq!(scale_out_level(0), 0);
        assert_eq!(scale_out_level(19), 46);
        assert_eq!(scale_out_level(20), 48);
        assert_eq!(scale_out_level(99), 127);
    }
}
