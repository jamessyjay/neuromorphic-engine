use crate::engine::ffi::Environment;

pub struct EnvironmentRegulator {
    current_oxygen: f32,
    current_glucose: f32,
}

impl Default for EnvironmentRegulator {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentRegulator {
    pub fn new() -> Self {
        Self {
            current_oxygen: 1.0,
            current_glucose: 1.0,
        }
    }

    pub fn update_and_get(&mut self, _last_tick_duration_ms: f32, current_tick: u32, base_env: &Environment) -> Environment {
        // Phase 18 Fix: Emergency Metabolic Equilibrium Fix - Decouple from wall-clock
        self.current_oxygen = 1.0;
        self.current_glucose = 1.0;

        // Circadian Drive: Wakefulness multiplier decays from 1.5 to 1.0 over the first 5000 ticks
        let wakefulness_multiplier = if current_tick <= 5000 {
            1.5 - ((current_tick as f32) / 5000.0) * 0.5
        } else {
            1.0
        };

        // Return new environment state
        Environment {
            global_oxygen_level: self.current_oxygen,
            global_glucose_level: self.current_glucose,
            // Decay dopamine back to baseline (0.0)
            dopamine_multiplier: base_env.dopamine_multiplier * 0.95,
            toxin_levels: base_env.toxin_levels,
            light_level: base_env.light_level,
            stdp_learning_rate: base_env.stdp_learning_rate,
            wakefulness_multiplier,
        }
    }
}
