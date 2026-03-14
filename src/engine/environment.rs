use crate::engine::ffi::Environment;

const TARGET_TICK_MS: f32 = 2.0;
const CRITICAL_TICK_MS: f32 = 10.0;
const EMA_ALPHA: f32 = 0.01;

pub struct EnvironmentRegulator {
    current_oxygen: f32,
    current_glucose: f32,
    ema_tick_ms: f32,
}

impl EnvironmentRegulator {
    pub fn new() -> Self {
        Self {
            current_oxygen: 1.0,
            current_glucose: 1.0,
            ema_tick_ms: TARGET_TICK_MS, // Start assuming perfect performance
        }
    }

    pub fn update_and_get(&mut self, last_tick_duration_ms: f32, base_env: &Environment) -> Environment {
        // Calculate Oxygen (Fast Response)
        let penalty = (last_tick_duration_ms - TARGET_TICK_MS) / (CRITICAL_TICK_MS - TARGET_TICK_MS);
        self.current_oxygen = 1.0 - penalty.clamp(0.0, 1.0);

        // Calculate Glucose (Slow Response)
        self.ema_tick_ms = (last_tick_duration_ms * EMA_ALPHA) + (self.ema_tick_ms * (1.0 - EMA_ALPHA));
        let gluc_penalty = (self.ema_tick_ms - TARGET_TICK_MS) / (CRITICAL_TICK_MS - TARGET_TICK_MS);
        self.current_glucose = 1.0 - gluc_penalty.clamp(0.0, 1.0);

        // Return new environment state
        Environment {
            global_oxygen_level: self.current_oxygen,
            global_glucose_level: self.current_glucose,
            dopamine_multiplier: base_env.dopamine_multiplier,
            toxin_levels: base_env.toxin_levels,
            light_level: base_env.light_level,
            stdp_learning_rate: base_env.stdp_learning_rate,
        }
    }
}
