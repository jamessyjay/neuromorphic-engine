// src/network/stimulus.rs
use crate::network::graph::NetworkGraph;
use crate::engine::ffi::{Environment, NeuronType};
use crate::engine::constants::*;

pub struct StimulusService {
    pub interval: u32,
    pub delay_between_stimuli: u32,
    pub last_tick: u32,
    pub training_mode: bool,
    bell_group: Vec<u32>,
    meat_group: Vec<u32>,
    initialized: bool,
}

impl StimulusService {
    pub fn new(interval: u32) -> Self {
        Self {
            interval,
            delay_between_stimuli: STDP_IDEAL_WINDOW_TICKS, 
            last_tick: 0,
            training_mode: true,
            bell_group: Vec::new(),
            meat_group: Vec::new(),
            initialized: false,
        }
    }

    fn initialize(&mut self, graph: &NetworkGraph) {
        if self.initialized { return; }
        
        let mut sensors = Vec::new();
        for (i, n) in graph.neurons.iter().enumerate() {
            if n.type_ == NeuronType::Photoreceptor as u8 || n.type_ == NeuronType::Nociceptor as u8 {
                sensors.push(i as u32);
            }
        }
        
        // If not enough sensors, just use a percentage like before, but correctly discovered
        if sensors.is_empty() {
            let total_sensors = (graph.neurons.len() as f32 * SENSOR_GROUP_PERCENTAGE) as usize;
            for i in 0..total_sensors {
                sensors.push(i as u32);
            }
        }

        let mid = sensors.len() / 2;
        self.bell_group = sensors[..mid].to_vec();
        self.meat_group = sensors[mid..].to_vec();
        
        self.initialized = true;
    }

    pub fn process_tick(&mut self, graph: &mut NetworkGraph, env: &mut Environment, current_tick: u32) {
        if graph.neurons.is_empty() { return; }
        
        self.initialize(graph);

        // --- PHASE 3: TEST TOGGLE (Causality fix: Check at start of tick) ---
        if current_tick > 0 && current_tick % (self.interval * TRAINING_CYCLES_BEFORE_TEST) == 0 {
            self.training_mode = !self.training_mode;
            if !self.training_mode {
                println!("\n--- TEST: GIVING ONLY BELL, NO MEAT ---");
                env.dopamine_multiplier = 1.0; 
            } else {
                println!("\n--- RETURNING TO TRAINING ---");
            }
        }

        // --- PHASE 1: BELL (Stimulus A) ---
        if current_tick % self.interval == 0 {
            println!("[STIMULUS] BELL (Stimulus A)");
            for &id in &self.bell_group {
                let idx = id as usize;
                if idx < graph.neurons.len() && graph.neurons[idx].is_alive == 1 {
                    graph.neurons[idx].current_voltage = FORCED_SPIKE_VOLTAGE;
                }
            }
        }

        // --- PHASE 2: MEAT (Stimulus B) ---
        if self.training_mode && (current_tick % self.interval == self.delay_between_stimuli) {
            println!("[STIMULUS] MEAT (Stimulus B) + Dopamine");
            for &id in &self.meat_group {
                let idx = id as usize;
                if idx < graph.neurons.len() && graph.neurons[idx].is_alive == 1 {
                    graph.neurons[idx].current_voltage = FORCED_SPIKE_VOLTAGE;
                }
            }
            env.dopamine_multiplier = REWARD_DOPAMINE_MULTIPLIER; 
        }

        // Dopamine decay
        if env.dopamine_multiplier > 1.0 {
            env.dopamine_multiplier -= DOPAMINE_DECAY_RATE;
        }
    }
}
