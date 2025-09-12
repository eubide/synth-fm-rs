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
        }
    }

    pub fn trigger(&mut self, velocity: f32) {
        self.trigger_with_key_scale(velocity, 1.0);
    }

    pub fn trigger_with_key_scale(&mut self, velocity: f32, key_scale_factor: f32) {
        self.velocity = velocity;
        self.key_scale_factor = key_scale_factor;
        self.stage = EnvelopeStage::Stage1;
        self.target_level = self.level1 / 99.0;
        self.rate = self.calculate_rate(self.rate1) * self.key_scale_factor;
        // Debug: println!("Envelope triggered: velocity {:.2}, target {:.2}, rate {:.4}",
        //          velocity, self.target_level, self.rate);
    }

    pub fn release(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Stage4;
            self.target_level = self.level4 / 99.0;
            self.rate = self.calculate_rate(self.rate4) * self.key_scale_factor;
        }
    }

    pub fn process(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => return 0.0,
            _ => {}
        }

        if self.current_level < self.target_level {
            self.current_level += self.rate;
            if self.current_level >= self.target_level {
                self.current_level = self.target_level;
                self.advance_stage();
            }
        } else if self.current_level > self.target_level {
            self.current_level -= self.rate;
            if self.current_level <= self.target_level {
                self.current_level = self.target_level;
                self.advance_stage();
            }
        }

        self.current_level * self.velocity
    }

    fn advance_stage(&mut self) {
        match self.stage {
            EnvelopeStage::Stage1 => {
                self.stage = EnvelopeStage::Stage2;
                self.target_level = self.level2 / 99.0;
                self.rate = self.calculate_rate(self.rate2) * self.key_scale_factor;
            }
            EnvelopeStage::Stage2 => {
                self.stage = EnvelopeStage::Stage3;
                self.target_level = self.level3 / 99.0;
                self.rate = self.calculate_rate(self.rate3) * self.key_scale_factor;
            }
            EnvelopeStage::Stage3 => {
                // Sustain stage - stay here until release() is called
            }
            EnvelopeStage::Stage4 => {
                self.stage = EnvelopeStage::Idle;
                self.current_level = 0.0;
            }
            EnvelopeStage::Idle => {}
        }
    }

    fn calculate_rate(&self, rate_value: f32) -> f32 {
        if rate_value == 0.0 {
            return 0.0;
        }

        let time_seconds = (100.0 - rate_value) / 25.0 + 0.001;
        1.0 / (time_seconds * self.sample_rate)
    }

    pub fn is_active(&self) -> bool {
        self.stage != EnvelopeStage::Idle
    }

    pub fn reset(&mut self) {
        self.current_level = 0.0;
        self.stage = EnvelopeStage::Idle;
    }
}
