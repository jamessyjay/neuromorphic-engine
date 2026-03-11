use crate::ffi::{BaseNeuron, Synapse, NeuronType, init_neuron};
use crate::constants::*;
use rand::RngExt;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::collections::HashMap;
use sha2::{Sha256, Digest};

const MAX_ATP: f32 = 100.0;
const CLUSTERING_RADIUS_SQ: f32 = 400.0; 
const STOCHASTIC_GROWTH_PROB: f64 = 0.05; 
const NMDA_SPIKE_THRESHOLD: f32 = 5.0; // Phase 33

#[derive(Serialize, Deserialize)]
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

    pub fn is_sleep_needed(&self) -> bool {
        let mut total_wear = 0.0;
        let mut alive_count = 0;
        
        for n in &self.neurons {
            if n.is_alive {
                total_wear += n.metabolic_wear as f32;
                alive_count += 1;
            }
        }
        
        if alive_count == 0 { return false; }
        
        let avg_wear = total_wear / alive_count as f32;
        avg_wear > 40000.0 // Threshold from Phase 35
    }

    pub fn initiate_sleep_cycle(&mut self) {
        // Pruning (Offline): Remove weak synapses
        let initial_synapses = self.synapses.len();
        self.synapses.retain(|s| s.weight >= 0.05);
        let pruned = initial_synapses - self.synapses.len();
        println!("  -> Pruned {} weak synapses during sleep.", pruned);

        // Consolidation & Recovery
        for n in &mut self.neurons {
            if n.is_alive {
                n.metabolic_wear = 0;
                n.energy_atp = n.max_atp;
                n.calcium_level = 0.0;
            }
        }
        println!("  -> Neurons fully recovered.");

        // Snapshot (Serialization)
        println!("  -> Serializing network to disk...");
        let file = File::create("brain_save.bin").expect("Failed to create brain_save.bin");
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &self).expect("Failed to serialize network");
    }

    pub fn load_from_disk() -> Option<Self> {
        if std::path::Path::new("brain_save.bin").exists() {
            let file = File::open("brain_save.bin").expect("Failed to open brain_save.bin");
            let reader = BufReader::new(file);
            match bincode::deserialize_from(reader) {
                Ok(graph) => Some(graph),
                Err(e) => {
                    eprintln!("Failed to deserialize brain_save.bin: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn generate_fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        
        hasher.update((self.neurons.len() as u32).to_le_bytes());
        for n in &self.neurons {
            hasher.update(n.id.to_le_bytes());
            hasher.update((n.is_alive as u8).to_le_bytes());
            hasher.update(n.energy_atp.to_bits().to_le_bytes());
            hasher.update(n.metabolic_wear.to_le_bytes());
        }
        
        hasher.update((self.synapses.len() as u32).to_le_bytes());
        for s in &self.synapses {
            hasher.update(s.source_id.to_le_bytes());
            hasher.update(s.target_id.to_le_bytes());
            hasher.update(s.weight.to_bits().to_le_bytes());
            hasher.update(s.flags.to_le_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }

    pub fn snapshot_synapses(&self) -> HashMap<(u32, u32, u8), f32> {
        let mut map = HashMap::new();
        for s in &self.synapses {
            let branch_id = s.flags & 0x07;
            map.insert((s.source_id, s.target_id, branch_id), s.weight);
        }
        map
    }

    pub fn diff_synapses(before: &HashMap<(u32, u32, u8), f32>, after: &HashMap<(u32, u32, u8), f32>) {
        let mut pruned = 0;
        let mut strengthened = 0;
        let mut weakened = 0;
        let mut unchanged = 0;

        for (key, old_weight) in before {
            if let Some(new_weight) = after.get(key) {
                if new_weight > old_weight {
                    strengthened += 1;
                } else if new_weight < old_weight {
                    weakened += 1;
                } else {
                    unchanged += 1;
                }
            } else {
                pruned += 1;
            }
        }
        
        let mut new_synapses = 0;
        for key in after.keys() {
            if !before.contains_key(key) {
                new_synapses += 1;
            }
        }

        println!("--- SYNAPTIC DIFF REPORT ---");
        println!("Total Pre-Sleep: {}", before.len());
        println!("Total Post-Sleep: {}", after.len());
        println!("Pruned (Removed): {}", pruned);
        println!("New Synapses: {}", new_synapses);
        println!("Strengthened (LTP): {}", strengthened);
        println!("Weakened (LTD): {}", weakened);
        println!("Unchanged: {}", unchanged);
        println!("----------------------------");
    }

    pub fn add_neuron(&mut self, type_: NeuronType) -> u32 {
        let mut rng = rand::rng();
        let id = self.neurons.len() as u32;
        let mut neuron = BaseNeuron {
            id,
            type_: type_ as u8,
            death_reason: 0,
            is_alive: true,
            _pad1: 0,
            size: 1.0,
            current_voltage: 0.0,
            activation_threshold: 0.0,
            resting_voltage: 0.0,
            energy_atp: MAX_ATP,
            max_atp: MAX_ATP,
            leak_coeff: 1.0,
            calcium_level: 0.0,
            vesicles_pool: MAX_ATP, // Shared constant for pool size
            refractory_time_left: 0.0,
            last_spike_tick: 0,
            spikes_in_last_window: 0,
            metabolic_wear: 0,
            x: (rng.random_range(0.0..100.0) * 655.35) as u16,
            y: (rng.random_range(0.0..100.0) * 655.35) as u16,
            z: (rng.random_range(0.0..100.0) * 655.35) as u16,
            _pad2: 0,
            branches: [crate::ffi::Compartment {
                branch_type: 0,
                refractory_timer: 0,
                voltage: 0,
                nmda_threshold: 0,
                _padding: [0; 2],
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
            if synapse.weight > 0.0 && synapse.neurotransmitter_buffer > ASTROCYTE_CLEAN_THRESHOLD {
                synapse.neurotransmitter_buffer -= ASTROCYTE_CLEAN_AMOUNT;
            }
        }
        
        // 2. Supply ATP to hard working neurons (Only if alive and low ATP)
        for neuron in &mut self.neurons {
            if neuron.is_alive && neuron.energy_atp < ASTROCYTE_CLEAN_THRESHOLD {
                neuron.energy_atp = f32::min(MAX_ATP, neuron.energy_atp + ASTROCYTE_ATP_SUPPLY);
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
            let is_below_threshold = s.weight < PRUNING_WEIGHT_THRESHOLD;
            if is_below_threshold {
                s.ticks_below_threshold += SERVICE_TICK_INTERVAL; // Service runs every 10 ticks (Phase 34: Use constant)
            } else {
                s.ticks_below_threshold = 0;
            }

            let is_persistent_dead = s.ticks_below_threshold >= PRUNING_TICK_THRESHOLD;
            
            !is_endpoint_dead && !is_persistent_dead
        });
    }

    // Background Service: NeuroGenesis (Growth & Structural Plasticity)
    pub fn run_neurogenesis_service(&mut self, current_tick: u32) {
        if current_tick % NEUROGENESIS_INTERVAL != 0 { return; } 
        
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
            if !n_source.is_alive || !n_target.is_alive || n_source.energy_atp < 80.0 { continue; } // TODO: ATP Constant

            // 2. Spatial Check: Are they close? (Simulating clusters via distance)
            let x1 = n_source.x as f32 / 655.35;
            let y1 = n_source.y as f32 / 655.35;
            let z1 = n_source.z as f32 / 655.35;
            
            let x2 = n_target.x as f32 / 655.35;
            let y2 = n_target.y as f32 / 655.35;
            let z2 = n_target.z as f32 / 655.35;

            let dx = x1 - x2;
            let dy = y1 - y2;
            let dz = z1 - z2;
            let dist_sq = dx*dx + dy*dy + dz*dz;

            if dist_sq < CLUSTERING_RADIUS_SQ { 
                // 3. Activity Correlation: Both spiked recently?
                let s_active = current_tick - n_source.last_spike_tick < 100; // TODO: Correlation window constant
                let t_active = current_tick - n_target.last_spike_tick < 100;

                if s_active && t_active {
                    // 4. Stochastic Growth
                    if rng.random_bool(STOCHASTIC_GROWTH_PROB) {
                        // Success! Connect these neurons with target branch
                        // Find a living branch or default to 0
                        let branch_id = rng.random_range(0..8) as u8;
                        let is_inhib = rng.random_bool(0.2); 
                        self.connect(i as u32, j as u32, SEED_WEIGHT, branch_id, is_inhib, false);
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
            n.branches[idx].nmda_threshold = ((NMDA_SPIKE_THRESHOLD + 100.0) * 100.0) as u16;
            n.branches[idx].voltage = 10000; // 0.0f is packed as (0 + 100) * 100 = 10000
            n.branches[idx].refractory_timer = 0;
        }
    }
}
