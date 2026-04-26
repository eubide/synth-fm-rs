use crate::optimization::OPTIMIZATION_TABLES;

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

    current_level: f32,
    target_level: f32,
    rate: f32,
    stage: EnvelopeStage,
    velocity: f32,
    sample_rate: f32,
    key_scale_factor: f32,

    // Smoothing variables for click reduction
    rate_smoother: f32,
    target_rate: f32,
    smoothing_samples: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvelopeStage {
    Idle,
    Stage1,
    Stage2,
    Stage3,
    Stage4,
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

            current_level: 0.0,
            target_level: 0.0,
            rate: 0.0,
            stage: EnvelopeStage::Idle,
            velocity: 1.0,
            sample_rate,
            key_scale_factor: 1.0,

            // Initialize smoothing system - reduced for better transient response
            rate_smoother: 0.0,
            target_rate: 0.0,
            smoothing_samples: sample_rate * 0.002, // 2ms smoothing time for crystalline attacks
        }
    }

    pub fn trigger_with_key_scale(&mut self, velocity: f32, key_scale_factor: f32) {
        self.velocity = velocity;
        self.key_scale_factor = key_scale_factor;
        self.stage = EnvelopeStage::Stage1;
        self.target_level = self.level1 / 99.0;

        // For fast attacks (rate1 > 90), skip smoothing for crystalline transients
        let new_rate = self.calculate_rate(self.rate1) * self.key_scale_factor;
        if self.rate1 > 90.0 {
            // Instant attack - no smoothing for maximum clarity
            self.rate = new_rate;
            self.target_rate = new_rate;
            self.rate_smoother = new_rate;
        } else {
            // Smooth rate transition for slower attacks
            self.set_target_rate(new_rate);
        }
    }

    pub fn release(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Stage4;
            self.target_level = self.level4 / 99.0;

            // Smooth transition to release rate
            let new_rate = self.calculate_rate(self.rate4) * self.key_scale_factor;
            self.set_target_rate(new_rate);
        }
    }

    pub fn process(&mut self) -> f32 {
        if self.stage == EnvelopeStage::Idle {
            return 0.0;
        }

        // Smooth rate transitions to reduce clicks
        self.update_rate_smoothing();

        // Exponential approach for natural envelope curves
        // DX7-authentic timing: approach_factor = -ln(threshold) * rate_per_sample
        // where -ln(0.001) ≈ 6.908. This gives correct DX7 rate-to-time mapping:
        // Rate 99 → ~10ms, Rate 50 → ~600ms, Rate 25 → ~5s, Rate 0 → held
        let distance = self.target_level - self.current_level;
        if distance.abs() > 0.0001 {
            let approach_factor = (self.rate * 6.908).clamp(0.0000001, 0.5);
            self.current_level += distance * approach_factor;

            // Check if we're close enough to target to advance stage
            if distance.abs() < 0.001 {
                self.current_level = self.target_level;
                self.advance_stage();
            }
        } else {
            self.current_level = self.target_level;
            self.advance_stage();
        }

        // Apply velocity scaling
        self.current_level * self.velocity
    }

    fn advance_stage(&mut self) {
        match self.stage {
            EnvelopeStage::Stage1 => {
                self.stage = EnvelopeStage::Stage2;
                self.target_level = self.level2 / 99.0;
                let new_rate = self.calculate_rate(self.rate2) * self.key_scale_factor;
                self.set_target_rate(new_rate);
            }
            EnvelopeStage::Stage2 => {
                self.stage = EnvelopeStage::Stage3;
                self.target_level = self.level3 / 99.0;
                let new_rate = self.calculate_rate(self.rate3) * self.key_scale_factor;
                self.set_target_rate(new_rate);
            }
            EnvelopeStage::Stage3 => {
                // Sustain stage - stay here until release() is called
            }
            EnvelopeStage::Stage4 => {
                self.stage = EnvelopeStage::Idle;
                self.current_level = 0.0;
                self.rate = 0.0;
                self.rate_smoother = 0.0;
                self.target_rate = 0.0;
            }
            EnvelopeStage::Idle => {}
        }
    }

    fn calculate_rate(&self, rate_value: f32) -> f32 {
        if rate_value == 0.0 {
            return 0.0;
        }

        // Use optimized DX7 rate calculation
        let multiplier = OPTIMIZATION_TABLES.dx7_rate_to_multiplier(rate_value as u8);
        multiplier / self.sample_rate
    }

    pub fn is_active(&self) -> bool {
        self.stage != EnvelopeStage::Idle
    }

    /// Live envelope output: `level * velocity`, in 0..=1.
    pub fn current_output(&self) -> f32 {
        self.current_level * self.velocity
    }

    pub fn reset(&mut self) {
        self.current_level = 0.0;
        self.stage = EnvelopeStage::Idle;
        self.rate = 0.0;
        self.rate_smoother = 0.0;
        self.target_rate = 0.0;
    }

    // Set target rate for smooth transitions
    fn set_target_rate(&mut self, new_rate: f32) {
        self.target_rate = new_rate;

        // If we're starting from zero rate, set immediately
        if self.rate == 0.0 {
            self.rate = new_rate;
            self.rate_smoother = new_rate;
        }
    }

    // Smooth rate interpolation to reduce clicks at stage transitions
    fn update_rate_smoothing(&mut self) {
        if (self.rate - self.target_rate).abs() > 0.0001 {
            let rate_diff = self.target_rate - self.rate;
            let smoothing_factor = 1.0 / self.smoothing_samples;

            self.rate += rate_diff * smoothing_factor;

            // Snap to target when close enough
            if rate_diff.abs() < 0.0001 {
                self.rate = self.target_rate;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44_100.0;

    #[test]
    fn new_envelope_is_idle_and_outputs_zero() {
        let mut env = Envelope::new(SR);
        assert!(!env.is_active());
        assert_eq!(env.process(), 0.0);
        assert_eq!(env.current_output(), 0.0);
    }

    #[test]
    fn trigger_activates_envelope_and_starts_attack() {
        let mut env = Envelope::new(SR);
        env.trigger_with_key_scale(1.0, 1.0);
        assert!(env.is_active());
        // Run a few samples and verify output rises from 0
        let mut last = 0.0;
        for _ in 0..256 {
            last = env.process();
        }
        assert!(last > 0.0, "envelope should produce non-zero output after trigger");
    }

    #[test]
    fn fast_attack_is_instant_no_smoothing() {
        // rate1 = 99 path triggers the no-smoothing branch
        let mut env = Envelope::new(SR);
        env.rate1 = 99.0;
        env.trigger_with_key_scale(1.0, 1.0);
        // Should reach near 1.0 within ~10ms (about 441 samples)
        let mut peak = 0.0_f32;
        for _ in 0..2000 {
            peak = peak.max(env.process());
        }
        assert!(peak > 0.95, "fast attack should reach near unity, got {peak}");
    }

    #[test]
    fn slow_attack_takes_time() {
        let mut env = Envelope::new(SR);
        env.rate1 = 5.0; // very slow attack
        env.trigger_with_key_scale(1.0, 1.0);
        // After only 64 samples the level should still be very low
        let mut last = 0.0;
        for _ in 0..64 {
            last = env.process();
        }
        assert!(last < 0.5, "slow attack should still be ramping after 64 samples, got {last}");
    }

    #[test]
    fn release_drives_envelope_to_idle() {
        let mut env = Envelope::new(SR);
        env.rate1 = 99.0;
        env.rate4 = 99.0;
        env.level4 = 0.0;
        env.trigger_with_key_scale(1.0, 1.0);
        for _ in 0..4096 {
            env.process();
        }
        env.release();
        // Rate 99 release reaches idle in well under 100ms.
        for _ in 0..(SR as usize / 10) {
            env.process();
            if !env.is_active() {
                break;
            }
        }
        assert!(!env.is_active(), "envelope should reach idle after release");
        assert_eq!(env.process(), 0.0);
    }

    #[test]
    fn release_when_idle_is_noop() {
        let mut env = Envelope::new(SR);
        env.release();
        assert!(!env.is_active());
        assert_eq!(env.process(), 0.0);
    }

    #[test]
    fn velocity_scales_output() {
        let mut env_full = Envelope::new(SR);
        let mut env_half = Envelope::new(SR);
        env_full.rate1 = 99.0;
        env_half.rate1 = 99.0;
        env_full.trigger_with_key_scale(1.0, 1.0);
        env_half.trigger_with_key_scale(0.5, 1.0);

        let mut peak_full = 0.0_f32;
        let mut peak_half = 0.0_f32;
        for _ in 0..2000 {
            peak_full = peak_full.max(env_full.process());
            peak_half = peak_half.max(env_half.process());
        }
        assert!(peak_full > peak_half * 1.5, "full velocity should output significantly more than half ({peak_full} vs {peak_half})");
    }

    #[test]
    fn key_scale_factor_speeds_up_envelope() {
        let mut env_norm = Envelope::new(SR);
        let mut env_fast = Envelope::new(SR);
        env_norm.rate1 = 30.0;
        env_fast.rate1 = 30.0;
        env_norm.trigger_with_key_scale(1.0, 1.0);
        env_fast.trigger_with_key_scale(1.0, 4.0); // 4x scale factor → faster

        let mut last_norm = 0.0;
        let mut last_fast = 0.0;
        for _ in 0..512 {
            last_norm = env_norm.process();
            last_fast = env_fast.process();
        }
        assert!(last_fast >= last_norm, "key-scaled envelope should ramp faster: norm={last_norm}, fast={last_fast}");
    }

    #[test]
    fn rate_zero_yields_no_motion() {
        let mut env = Envelope::new(SR);
        env.rate1 = 0.0;
        env.trigger_with_key_scale(1.0, 1.0);
        // process should not advance level when rate is 0
        let v0 = env.process();
        for _ in 0..1024 {
            env.process();
        }
        let v1 = env.process();
        // With rate=0 the approach factor is clamped to a tiny value but never increases
        // level beyond what the floating point noise allows. Both should be ~ near v0.
        assert!((v1 - v0).abs() < 0.5, "rate 0 should not move appreciably: v0={v0}, v1={v1}");
    }

    #[test]
    fn reset_returns_to_initial_state() {
        let mut env = Envelope::new(SR);
        env.trigger_with_key_scale(1.0, 1.0);
        for _ in 0..256 {
            env.process();
        }
        env.reset();
        assert!(!env.is_active());
        assert_eq!(env.process(), 0.0);
    }

    #[test]
    fn current_output_matches_velocity_scale() {
        let mut env = Envelope::new(SR);
        env.rate1 = 99.0;
        env.trigger_with_key_scale(0.7, 1.0);
        for _ in 0..2000 {
            env.process();
        }
        let live = env.current_output();
        assert!(live > 0.0 && live <= 1.0, "live output should be 0-1, got {live}");
    }

    #[test]
    fn full_envelope_lifecycle_traverses_all_stages() {
        let mut env = Envelope::new(SR);
        env.rate1 = 99.0;
        env.rate2 = 99.0;
        env.rate3 = 99.0;
        env.rate4 = 99.0;
        env.level1 = 99.0;
        env.level2 = 75.0;
        env.level3 = 40.0;
        env.level4 = 0.0;
        env.trigger_with_key_scale(1.0, 1.0);
        let mut last = 0.0;
        for _ in 0..8192 {
            last = env.process();
        }
        assert!((last - 0.4).abs() < 0.1, "should hold near level3=0.4, got {last}");

        env.release();
        for _ in 0..(SR as usize / 10) {
            env.process();
            if !env.is_active() {
                break;
            }
        }
        assert!(!env.is_active());
    }
}
