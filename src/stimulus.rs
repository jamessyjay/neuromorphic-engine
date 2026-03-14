// src/stimulus.rs
use crate::graph::NetworkGraph;
use crate::ffi::Environment;
use crate::constants::*;

pub struct StimulusService {
    pub interval: u32,
    pub delay_between_stimuli: u32, // Delay between Bell and Meat stimuli
    pub last_tick: u32,
    pub training_mode: bool,        // If true - train, if false - test
}

impl StimulusService {
    pub fn new(interval: u32) -> Self {
        Self {
            interval,
            delay_between_stimuli: STDP_IDEAL_WINDOW_TICKS, // 30ms - ideal window for STDP // 30ms - ideal window for STDP
            last_tick: 0,
            training_mode: true,
        }
    }

    pub fn process_tick(&mut self, graph: &mut NetworkGraph, env: &mut Environment, current_tick: u32) {
        let num_neurons = graph.neurons.len();
        if num_neurons == 0 { return; }

        // Take 4% of neurons for sensors (to fit two groups)
        let total_sensors = (num_neurons as f32 * SENSOR_GROUP_PERCENTAGE) as usize;
        let group_size = total_sensors / 2;
        
        let bell_sensors = 0..group_size;
        let meat_sensors = group_size..total_sensors;

        // --- PHASE 1: BELL (Stimulus A) ---
        // Give bell every `interval` ticks
        if current_tick % self.interval == 0 {
            println!("[STIMULUS] BELL (Stimulus A)");
            for i in bell_sensors {
                if i < graph.neurons.len() {
                    graph.neurons[i].current_voltage = FORCED_SPIKE_VOLTAGE; // Forced spike
                }
            }
        }

        // --- PHASE 2: MEAT (Stimulus B) ---
        // Give meat only in training mode and with delay after bell
        if self.training_mode && (current_tick % self.interval == self.delay_between_stimuli) {
            println!("[STIMULUS] MEAT (Stimulus B) + Dopamine");
            for i in meat_sensors {
                if i < graph.neurons.len() {
                    graph.neurons[i].current_voltage = FORCED_SPIKE_VOLTAGE;
                }
            }
            // Meat itself causes pleasure
            env.dopamine_multiplier = REWARD_DOPAMINE_MULTIPLIER; 
        }

        // --- PHASE 3: TEST (Testing for stupidity) ---
        // Every 5 training cycles disable meat and watch the reaction
        if current_tick > 0 && current_tick % (self.interval * TRAINING_CYCLES_BEFORE_TEST) == 0 {
            self.training_mode = !self.training_mode;
            if !self.training_mode {
                println!("\n--- TEST: GIVING ONLY BELL, NO MEAT ---");
                env.dopamine_multiplier = 1.0; // Reset dopamine to normal
            } else {
                println!("\n--- RETURNING TO TRAINING ---");
            }
        }

        // Gradual dopamine decay to not keep the network in eternal ecstasy
        if env.dopamine_multiplier > 1.0 {
            env.dopamine_multiplier -= DOPAMINE_DECAY_RATE;
        }
    }
}
