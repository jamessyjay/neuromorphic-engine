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

pub struct SensoryHub {
    pub input_frame: Vec<f32>,
    /// Maps pixel index (0..1024) to a list of L4 Photoreceptor IDs
    pub pixel_to_neuron_map: std::collections::HashMap<usize, Vec<u32>>,
}

impl SensoryHub {
    pub fn new() -> Self {
        Self {
            input_frame: vec![0.0; TOTAL_INPUT_CHANNELS],
            pixel_to_neuron_map: std::collections::HashMap::new(),
        }
    }

    pub fn build_mapping(&mut self, graph: &NetworkGraph) {
        self.pixel_to_neuron_map.clear();

        // 32x32 image is divided into 10 patches (e.g., 5 columns x 2 rows of patches)
        // Image width = 32, height = 32
        // Patch width = 32 / 5 = 6.4 (let's say 6 or 7 pixels)
        // Actually, just map column_id (0..9) to a distinct set of pixels.
        // For simplicity: each column gets ~102 pixels.
        
        let mut rng = rand::rng();
        
        for n in &graph.neurons {
            if n.layer_id == 4 && n.type_ == NeuronType::Photoreceptor as u8 {
                // Column 0..9
                let col = n.column_id as usize;
                
                // Which pixels belong to this column?
                // E.g., chunk the 1024 pixels into 10 blocks of 102
                let patch_start = (col * 102).min(1024);
                let patch_end = ((col + 1) * 102).min(1024);
                
                // Pick 5 random pixels from this patch for each photoreceptor to look at
                for _ in 0..5 {
                    let pixel_idx = rand::RngExt::random_range(&mut rng, patch_start..patch_end);
                    self.pixel_to_neuron_map.entry(pixel_idx).or_default().push(n.id);
                }
            }
        }
    }

    pub fn update_frame(&mut self, new_data: Vec<f32>) {
        if new_data.len() == TOTAL_INPUT_CHANNELS {
            self.input_frame = new_data;
        }
    }

    pub fn apply_to_graph(&self, graph: &mut NetworkGraph) {
        let mut rng = rand::rng();
        
        for (pixel_idx, val) in self.input_frame.iter().enumerate() {
            if *val > 0.5 {
                if let Some(target_neurons) = self.pixel_to_neuron_map.get(&pixel_idx) {
                    for &n_id in target_neurons {
                        // SDR Filter: only activate SDR_TARGET_SPARSITY percentage
                        if rand::RngExt::random_bool(&mut rng, SDR_TARGET_SPARSITY as f64) {
                            graph.neurons[n_id as usize].current_voltage = STIMULUS_VOLTAGE_BOOST;
                        }
                    }
                }
            }
        }
    }
}
