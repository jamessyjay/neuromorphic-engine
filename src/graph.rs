use crate::ffi::{BaseNeuron, Synapse, NeuronType, init_neuron};
use rand::RngExt;

pub struct NetworkGraph {
    pub neurons: Vec<BaseNeuron>,
    pub synapses: Vec<Synapse>,
}

impl NetworkGraph {
    pub fn new() -> Self {
        NetworkGraph {
            neurons: Vec::new(),
            synapses: Vec::new(),
        }
    }

    pub fn add_neuron(&mut self, type_: NeuronType) -> u32 {
        let mut rng = rand::rng();
        let id = self.neurons.len() as u32;
        let mut neuron = BaseNeuron {
            id,
            type_,
            size: 1.0,
            current_voltage: 0.0,
            activation_threshold: 0.0,
            resting_voltage: 0.0,
            energy_atp: 100.0,
            max_atp: 100.0,
            leak_coeff: 1.0,
            calcium_level: 0.0,
            vesicles_pool: 100.0,
            refractory_time_left: 0.0,
            spikes_in_last_window: 0,
            metabolic_wear: 0.0,
            last_spike_tick: 0,
            x: rng.random_range(0.0..100.0),
            y: rng.random_range(0.0..100.0),
            z: rng.random_range(0.0..100.0),
            death_reason: 0,
            is_alive: true,
            branches: [crate::ffi::Compartment {
                branch_type: 0,
                voltage: 0.0,
                nmda_threshold: 0.0,
                _padding: [0; 3],
            }; 8],
        };
        
        // Let C++ engine initialize the neuron biology exactly per Spec 05 & 11
        // Using a combination of global seed and neuron ID
        unsafe {
            init_neuron(&mut neuron, id, type_, 42 + id);
        }
        
        self.neurons.push(neuron);
        id
    }

    pub fn tick(&mut self, current_tick: u32, env: crate::ffi::Environment) {
        unsafe {
            crate::ffi::launch_tick_neurons(
                self.neurons.as_mut_ptr(),
                self.neurons.len() as i32,
                self.synapses.as_mut_ptr(),
                self.synapses.len() as i32,
                env,
                current_tick
            );
        }
    }

    pub fn connect(&mut self, source_id: u32, target_id: u32, weight: f32, branch_id: u8, is_inhibitory: bool, is_myelinated: bool) {
        let mut flags = branch_id & 0x07; // Bits 0-2
        if is_inhibitory { flags |= 0x08; } // Bit 3
        if is_myelinated { flags |= 0x10; } // Bit 4

        let synapse = Synapse {
            source_id,
            target_id,
            weight,
            neurotransmitter_buffer: 0.0,
            last_activation_tick: 0,
            ticks_below_threshold: 0,
            ticks_until_delivery: 0,
            flags,
            _padding: [0; 2],
        };
        
        self.synapses.push(synapse);
    }
    
    // Background Service: Astrocyte (Garbage Collector & Support)
    pub fn run_astrocyte_service(&mut self, _current_tick: u32) {
        // Optimization: Periodic chunked processing or threshold-based
        // For now, let's just make the loops as tight as possible
        
        // 1. Clean synaptic overflow (Only if weight > 0)
        for synapse in &mut self.synapses {
            if synapse.weight > 0.0 && synapse.neurotransmitter_buffer > 50.0 {
                synapse.neurotransmitter_buffer -= 10.0;
            }
        }
        
        // 2. Supply ATP to hard working neurons (Only if alive and low ATP)
        for neuron in &mut self.neurons {
            if neuron.is_alive && neuron.energy_atp < 50.0 {
                neuron.energy_atp = f32::min(100.0, neuron.energy_atp + 10.0);
            }
        }
    }

    // Background Service: Microglia (Immune System / Firewall / Pruning)
    pub fn run_microglia_service(&mut self, _current_tick: u32) {
        let dead_ids: std::collections::HashSet<u32> = self.neurons.iter()
            .filter(|n| !n.is_alive)
            .map(|n| n.id)
            .collect();

        // Phase 29: Structural Plasticity - PRUNING
        // Remove synapses that are:
        // 1. Connected to dead neurons
        // 2. Below the weight threshold (0.01) - "Impotent connections"
        self.synapses.retain_mut(|s| {
            let is_endpoint_dead = dead_ids.contains(&s.source_id) || dead_ids.contains(&s.target_id);
            
            // Phase 31: Hard Pruning with Persistence
            let is_below_threshold = s.weight < 0.05;
            if is_below_threshold {
                s.ticks_below_threshold += 10; // Service runs every 10 ticks
            } else {
                s.ticks_below_threshold = 0;
            }

            let is_persistent_dead = s.ticks_below_threshold >= 1000;
            
            !is_endpoint_dead && !is_persistent_dead
        });
    }

    // Background Service: NeuroGenesis (Growth & Structural Plasticity)
    pub fn run_neurogenesis_service(&mut self, current_tick: u32) {
        if current_tick % 500 != 0 { return; } // Reduced frequency per Phase 31
        
        let mut rng = rand::rng();
        let num_neurons = self.neurons.len();
        if num_neurons < 2 { return; }

        // Large Scale Mapping: Stochastic sampling instead of O(N^2)
        // We check num_neurons pairs per service call
        let check_count = num_neurons; 
        
        for _ in 0..check_count {
            let i = rng.random_range(0..num_neurons);
            let j = rng.random_range(0..num_neurons);
            if i == j { continue; }

            let n_source = &self.neurons[i];
            let n_target = &self.neurons[j];

            // 1. Metabolic Check: Can source afford to grow?
            if !n_source.is_alive || !n_target.is_alive || n_source.energy_atp < 80.0 { continue; }

            // 2. Spatial Check: Are they close? (Simulating clusters via distance)
            let dx = n_source.x - n_target.x;
            let dy = n_source.y - n_target.y;
            let dz = n_source.z - n_target.z;
            let dist_sq = dx*dx + dy*dy + dz*dz;

            if dist_sq < 400.0 { // Tighter cluster radius = 20.0
                // 3. Activity Correlation: Both spiked recently?
                let s_active = current_tick - n_source.last_spike_tick < 100;
                let t_active = current_tick - n_target.last_spike_tick < 100;

                if s_active && t_active {
                    // 4. Stochastic Growth P=0.2 per check
                    if rng.random_bool(0.2) {
                        // Success! Connect these neurons with target branch
                        // Find a living branch or default to 0
                        let branch_id = rng.random_range(0..8) as u8;
                        let is_inhib = rng.random_bool(0.2); 
                        self.connect(i as u32, j as u32, 0.05, branch_id, is_inhib, false);
                    }
                }
            }
        }
    }

    pub fn grow_dendritic_tree(&mut self, neuron_id: u32) {
        let mut rng = rand::rng();
        let num_branches = rng.random_range(2..=4);
        let n = &mut self.neurons[neuron_id as usize];
        
        // Fisher-Yates shuffle to pick unique branch indices
        let mut indices: Vec<usize> = (0..8).collect();
        for i in (1..8).rev() {
            let j = rng.random_range(0..=i);
            indices.swap(i, j);
        }

        for i in 0..num_branches {
            let idx = indices[i];
            let b_type = rng.random_range(1..=3) as u8;
            n.branches[idx].branch_type = b_type;
            n.branches[idx].nmda_threshold = 5.0; // 5.0 glutamate units for NMDA spike
            n.branches[idx].voltage = 0.0;
        }
    }
}
