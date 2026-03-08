use crate::ffi::{BaseNeuron, Synapse, NeuronType, init_neuron};

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
            is_alive: true,
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

    pub fn connect(&mut self, source_id: u32, target_id: u32, weight: f32, is_inhibitory: bool, is_myelinated: bool) {
        let synapse = Synapse {
            source_id,
            target_id,
            weight,
            neurotransmitter_buffer: 0.0,
            last_activation_tick: 0,
            ticks_until_delivery: 0,
            is_inhibitory,
            is_myelinated,
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

    // Background Service: Microglia (Immune System / Firewall)
    pub fn run_microglia_service(&mut self, _current_tick: u32) {
        // Optimization: Find dead neurons once, then nullify synapses using O(1) lookup
        let dead_ids: std::collections::HashSet<u32> = self.neurons.iter()
            .filter(|n| !n.is_alive)
            .map(|n| n.id)
            .collect();

        if dead_ids.is_empty() { return; }

        for synapse in &mut self.synapses {
            if synapse.weight > 0.0 {
                if dead_ids.contains(&synapse.source_id) || dead_ids.contains(&synapse.target_id) {
                    synapse.weight = 0.0;
                    synapse.neurotransmitter_buffer = 0.0;
                }
            }
        }
    }
}
