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

            // Initialize smoothing system
            rate_smoother: 0.0,
            target_rate: 0.0,
            smoothing_samples: sample_rate * 0.005, // 5ms smoothing time
        }
    }

    pub fn trigger_with_key_scale(&mut self, velocity: f32, key_scale_factor: f32) {
        self.velocity = velocity;
        self.key_scale_factor = key_scale_factor;
        self.stage = EnvelopeStage::Stage1;
        self.target_level = self.level1 / 99.0;

        // Initialize smooth rate transition
        let new_rate = self.calculate_rate(self.rate1) * self.key_scale_factor;
        self.set_target_rate(new_rate);
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
        match self.stage {
            EnvelopeStage::Idle => return 0.0,
            _ => {}
        }

        // Smooth rate transitions to reduce clicks
        self.update_rate_smoothing();

        // Use exponential approach instead of linear for more natural sound
        let distance = self.target_level - self.current_level;
        if distance.abs() > 0.0001 {
            // Exponential approach with smooth rate
            let approach_factor = (self.rate * 2.0).min(0.1);
            self.current_level += distance * approach_factor;

            // Check if we're close enough to target to advance stage
            if distance.abs() < 0.001 {
                self.current_level = self.target_level;
                self.advance_stage();
            }
        }

        // Apply soft knee to velocity response for smoother dynamics
        let velocity_curve = self.velocity * self.velocity * (3.0 - 2.0 * self.velocity);
        self.current_level * velocity_curve
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
