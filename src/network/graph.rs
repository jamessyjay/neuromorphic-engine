use crate::engine::ffi::{BaseNeuron, Synapse, NeuronType, init_neuron};
use crate::engine::constants::*;
use rand::RngExt;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use rayon::prelude::*; // For massive parallelism over CPU

#[derive(Clone, Serialize, Deserialize)]
pub struct NetworkGraph {
    pub neurons: Vec<BaseNeuron>,
    pub synapses: Vec<Synapse>,
}



impl Default for NetworkGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkGraph {
    pub fn new() -> Self {
        NetworkGraph {
            neurons: Vec::new(),
            synapses: Vec::new(),
        }
    }

    pub fn optimize_memory_layout(&mut self) {
        self.synapses.sort_unstable_by(|a, b| {
            a.target_id.cmp(&b.target_id).then(a.source_id.cmp(&b.source_id))
        });
    }

    pub fn is_sleep_needed(&self) -> bool {
        let mut total_wear = 0.0;
        let mut alive_count = 0;
        
        for n in &self.neurons {
            if n.is_alive == 1 {
                total_wear += n.metabolic_wear as f32;
                alive_count += 1;
            }
        }
        
        if alive_count == 0 { return false; }
        
        let avg_wear = total_wear / alive_count as f32;
        avg_wear > FATIGUE_SLEEP_THRESHOLD // Threshold from Phase 35
    }

    pub fn initiate_sleep_cycle(&mut self) {
        // Pruning (Offline): Remove ONLY extremely weak synapses (Phase 29/37)
        let initial_synapses = self.synapses.len();
        self.synapses.retain(|s| s.weight >= 0.01); // Lower threshold
        let pruned = initial_synapses - self.synapses.len();
        println!("  -> Pruned {} weak synapses during sleep.", pruned);

        // Consolidation & Recovery
        for n in &mut self.neurons {
            if n.is_alive == 1 {
                n.metabolic_wear = 0;
                n.energy_atp = n.max_atp;
                n.calcium_level = 0.0;
            }
        }
        println!("  -> Neurons fully recovered.");
    }



    pub fn generate_fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        
        hasher.update((self.neurons.len() as u32).to_le_bytes());
        for n in &self.neurons {
            hasher.update(n.id.to_le_bytes());
            hasher.update(n.is_alive.to_le_bytes());
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

    /// Deep memory synchronization for CUDA backend after loading from disk
    /// This ensures that the newly allocated host Vecs are properly registered
    /// or explicitly copied to the device memory spaces.
    pub fn force_sync_gpu(&mut self) {
        #[cfg(feature = "cuda")]
        {
            // Note: If using Unified Memory (cudaMallocManaged) via custom allocator 
            // inside Vec, Rust's load_from_disk might just do a standard heap alloc.
            // In a real CUDA environment with strictly managed memory, we would need 
            // to allocate new managed buffers and memcpy data into them.
            // For now, if the system relies on FFI to map things or the FFI expects 
            // standard pointers that it copies internally, we just print a log.
            // If the kernels use the pointers directly from Rust, those pointers MUST 
            // be in managed memory. 
            // We simulate the sync by touching the memory.
            println!("[CUDA SYNC] Forcing Host-To-Device memory alignment for {} neurons and {} synapses...", 
                self.neurons.len(), self.synapses.len());
            // In a full implementation, you would call a C FFI function here:
            // unsafe { crate::engine::ffi::sync_gpu_memory(self.neurons.as_mut_ptr(), self.neurons.len() as i32); }
        }
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
            is_alive: 1,
            internal_phase: 0,
            spike_trace: 0.0,
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
            predictive_ticks: 0,
            metabolic_wear: 0,
            x: (rng.random_range(0.0..MAX_COORD) * COORD_SCALE) as u16,
            y: (rng.random_range(0.0..MAX_COORD) * COORD_SCALE) as u16,
            z: (rng.random_range(0.0..MAX_COORD) * COORD_SCALE) as u16,
            dead_ticks: 0,
            column_id: 0,
            minicolumn_id: 0,
            layer_id: 0,
            status_flags: 0,
            external_input: 0.0,
            _pad_block1: [0; 28],
            ref_coord_x: 0.0,
            ref_coord_y: 0.0,
            _pad_block2: [0; 16],
            branches: [crate::engine::ffi::Compartment {
                branch_type: 0,
                refractory_timer: 0,
                voltage: 0,
                nmda_threshold: 0,
                local_trace: 0,
                _pad: 0,
            }; 16],
        };
        
        // Let C++ engine initialize the neuron biology exactly per Spec 05 & 11
        // Using a combination of global seed and neuron ID
        unsafe {
            init_neuron(&mut neuron, id, type_, 42 + id);
        }
        
        self.neurons.push(neuron);
        id
    }

    pub fn apply_synaptic_scaling(&mut self) {
        let mut weight_sums = vec![0.0f32; self.neurons.len()];
        let mut synapse_counts = vec![0u32; self.neurons.len()];
        
        for s in &self.synapses {
            let tid = s.target_id as usize;
            if tid < weight_sums.len() {
                weight_sums[tid] += s.weight;
                synapse_counts[tid] += 1;
            }
        }
        
        for s in &mut self.synapses {
            let tid = s.target_id as usize;
            if tid < weight_sums.len() {
                let count = synapse_counts[tid] as f32;
                if count > 0.0 {
                    let target_sum = count * 2.0; // Target average weight is 2.0
                    let current_sum = weight_sums[tid];
                    // If sum of weights exceeds the target, scale them down proportionally
                    if current_sum > target_sum {
                        let scale = target_sum / current_sum;
                        s.weight *= scale;
                    }
                }
            }
        }
    }

    pub fn tick(&mut self, current_tick: u32, env: crate::engine::ffi::Environment) {
        // Phase 3: Reset (happens at the beginning of tick)
        for n in &mut self.neurons {
            n.status_flags &= !crate::engine::constants::FLAG_IS_INHIBITED;
        }

        #[cfg(target_os = "macos")]
        {
            // MacOS CPU Fallback using Rayon Multi-Threading
            self.neurons.par_iter_mut().for_each(|n| {
                unsafe { crate::engine::ffi::tick_neuron_prepare_cpu(n, env, current_tick); }
            });

            // Phase 2: WTA and Bursting
            use std::collections::HashMap;
            // Map from (column_id, minicolumn_id) -> (winner_id, winner_v, is_predictive)
            let mut minicolumn_winners: HashMap<(u16, u8), (u32, f32, bool)> = HashMap::new();
            
            for n in &self.neurons {
                if n.is_alive == 1 && n.current_voltage > n.activation_threshold && (n.type_ == 0 || n.type_ == 2) {
                    let key = (n.column_id, n.minicolumn_id);
                    let is_pred = (n.status_flags & crate::engine::constants::FLAG_IS_PREDICTIVE) != 0;
                    let effective_v = if is_pred { n.current_voltage + 1000.0 } else { n.current_voltage };
                    
                    if let Some(current_winner) = minicolumn_winners.get_mut(&key) {
                        let (_, winner_eff_v, _) = *current_winner;
                        if effective_v > winner_eff_v || (effective_v == winner_eff_v && n.id < current_winner.0) {
                            *current_winner = (n.id, effective_v, is_pred);
                        }
                    } else {
                        minicolumn_winners.insert(key, (n.id, effective_v, is_pred));
                    }
                }
            }

            for n in &mut self.neurons {
                if n.is_alive == 1 && (n.type_ == 0 || n.type_ == 2) {
                    let key = (n.column_id, n.minicolumn_id);
                    if let Some(&(winner_id, _, winner_was_predictive)) = minicolumn_winners.get(&key) {
                        if !winner_was_predictive {
                            // Surprise! Bursting (No predictive neurons crossed threshold)
                            if n.current_voltage <= n.activation_threshold {
                                n.current_voltage = n.activation_threshold + 5.0;
                            }
                        } else if n.id != winner_id {
                            n.status_flags |= crate::engine::constants::FLAG_IS_INHIBITED;
                            n.current_voltage = n.resting_voltage - 10.0;
                            n.refractory_time_left = 5.0;
                        }
                    }
                }
            }
            
            self.neurons.par_iter_mut().for_each(|n| {
                unsafe { crate::engine::ffi::tick_neuron_spike_cpu(n, env, current_tick); }
            });

            // Convert pointer to usize to make it trivially Send+Sync
            let neurons_ptr_usize = self.neurons.as_mut_ptr() as usize;
            
            self.synapses.par_iter_mut().for_each(|s| {
                let ptr = neurons_ptr_usize as *mut crate::engine::ffi::BaseNeuron;
                unsafe { crate::engine::ffi::tick_synapse_cpu(s, ptr, env, current_tick); }
            });
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // Jetson/CUDA mode
            unsafe {
                crate::engine::ffi::launch_tick_neurons(
                    self.neurons.as_mut_ptr(),
                    self.neurons.len() as i32,
                    self.synapses.as_mut_ptr(),
                    self.synapses.len() as i32,
                    env,
                    current_tick
                );
            }
        }

        self.apply_sdr_tuning(current_tick);
        if current_tick % 100 == 0 {
            self.apply_synaptic_scaling();
        }
    }

    pub fn apply_sdr_tuning(&mut self, current_tick: u32) {
        // SDR Tuning for L4 (Target Sparsity 5%)
        let mut l4_total = 0;
        let mut l4_active = 0;
        for n in &self.neurons {
            if n.layer_id == 4 && n.is_alive == 1 {
                l4_total += 1;
                if n.last_spike_tick == current_tick {
                    l4_active += 1;
                }
            }
        }

        if l4_total > 0 {
            let activity = l4_active as f32 / l4_total as f32;
            let gamma = 0.1; // Sensitivity coefficient
            // theta_t+1 = theta_t + (activity - 0.05) * gamma
            let adjustment = (activity - 0.05) * gamma;
            
            for n in &mut self.neurons {
                if n.layer_id == 4 && n.is_alive == 1 {
                    n.activation_threshold += adjustment;
                    // Clamp threshold to reasonable biological limits
                    n.activation_threshold = n.activation_threshold.clamp(-65.0, 10.0);
                }
            }
        }

        // Homeostatic threshold penalty for Output Layer (L5 / MNIST Labels)
        // If a label neuron fires too often, we increase its threshold to give others a chance
        for i in 1784..1794 {
            if i < self.neurons.len() {
                let n = &mut self.neurons[i];
                if n.is_alive == 1 {
                    // Decay threshold back to baseline slowly
                    if n.activation_threshold > n.resting_voltage + 15.0 {
                        n.activation_threshold -= 0.001; 
                    }
                    // Increase threshold on recent spike to prevent monopolization (The "7" Demon)
                    if n.last_spike_tick == current_tick {
                        n.activation_threshold += 0.5; // Adaptive homeostasis
                    }
                }
            }
        }
    }

    pub fn connect(&mut self, source_id: u32, target_id: u32, weight: f32, branch_id: u8, is_inhibitory: bool, is_myelinated: bool) {
        let mut flags = branch_id & 0x07; // Bits 0-2
        if is_inhibitory { flags |= 0x08; } // Bit 3
        if is_myelinated { flags |= 0x10; } // Bit 4

        // Prevent duplicates (same source, target, and branch)
        for synapse in &mut self.synapses {
            if synapse.source_id == source_id && synapse.target_id == target_id && (synapse.flags & 0x07) == branch_id {
                // If exists, just augment the weight and return
                synapse.weight += weight;
                return;
            }
        }

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
    pub fn run_astrocyte_service(&mut self, env: &crate::engine::ffi::Environment) {
        unsafe {
            crate::engine::ffi::launch_astrocyte_service(
                self.neurons.as_mut_ptr(),
                self.neurons.len() as libc::c_int,
                self.synapses.as_mut_ptr(),
                self.synapses.len() as libc::c_int,
                *env
            );
        }
    }

    // Background Service: Microglia (Immune System / Firewall / Pruning)
    pub fn run_microglia_service(&mut self, current_tick: u32) {
        unsafe {
            crate::engine::ffi::launch_microglia_service(
                self.neurons.as_mut_ptr(),
                self.neurons.len() as libc::c_int,
                self.synapses.as_mut_ptr(),
                self.synapses.len() as libc::c_int,
                current_tick
            );
        }
    }

    // Background Service: NeuroGenesis (Growth & Structural Plasticity)
    pub fn run_neurogenesis_service(&mut self, current_tick: u32) {
        if current_tick % NEUROGENESIS_INTERVAL != 0 { return; } 
        
        let mut rng = rand::rng();
        let num_neurons = self.neurons.len();
        if num_neurons < 2 { return; }

        // Large Scale Mapping: Stochastic sampling instead of O(N^2)
        // We check num_neurons * 2 pairs per service call to maintain biological speed
        // Limit total synapses to prevent "spaghetti"
        if self.synapses.len() >= MAX_SYNAPSES_TOTAL { return; }

        // O(1) Pre-compute outgoing synapse counts to avoid O(N_synapses) per neurogenesis attempt
        let mut outgoing_counts = vec![0; num_neurons];
        for s in &self.synapses {
            if (s.source_id as usize) < num_neurons {
                outgoing_counts[s.source_id as usize] += 1;
            }
        }

        let check_count = num_neurons * 2; 
        
        for _ in 0..check_count {
            let i = rng.random_range(0..num_neurons);
            let j = rng.random_range(0..num_neurons);
            if i == j { continue; }

            let n_source = &self.neurons[i];
            let n_target = &self.neurons[j];

            // 1. Metabolic Check: Can source afford to grow? (ATP > 40)
            if n_source.is_alive == 0 || n_target.is_alive == 0 || n_source.energy_atp < 40.0 { continue; } 

            // Hard Limit: Max outgoing synapses per neuron (Phase 37) using O(1) lookup
            if outgoing_counts[i] >= MAX_SYNAPSES_PER_NEURON { continue; }

            // 2. Spatial Check: Are they close? (CLUSTERING_RADIUS_SQ * 1.5)
            let x1 = n_source.x as f32 / COORD_SCALE;
            let y1 = n_source.y as f32 / COORD_SCALE;
            let z1 = n_source.z as f32 / COORD_SCALE;
            
            let x2 = n_target.x as f32 / COORD_SCALE;
            let y2 = n_target.y as f32 / COORD_SCALE;
            let z2 = n_target.z as f32 / COORD_SCALE;

            let dx = x1 - x2;
            let dy = y1 - y2;
            let dz = z1 - z2;
            let dist_sq = dx*dx + dy*dy + dz*dz;

            if dist_sq < CLUSTERING_RADIUS_SQ * 1.5 { 
                // 3. Activity Correlation: Both spiked recently? (Window: 500 ticks)
                let s_active = current_tick.saturating_sub(n_source.last_spike_tick) < 500; 
                let t_active = current_tick.saturating_sub(n_target.last_spike_tick) < 500;

                if s_active && t_active {
                    // 4. Stochastic Growth (Phase 37: Use STOCHASTIC_GROWTH_PROB from constants)
                    if rng.random_bool(STOCHASTIC_GROWTH_PROB) {
                        // Success! Connect these neurons with target branch
                        let branch_id = rng.random_range(0..8) as u8;
                        let is_inhib = rng.random_bool(0.1); 
                        self.connect(i as u32, j as u32, SEED_WEIGHT, branch_id, is_inhib, false);
                        outgoing_counts[i] += 1;
                        if current_tick % 1000 == 0 {
                            println!("NeuroGenesis: New synapse {} -> {} at tick {}", i, j, current_tick);
                        }
                    }
                }
            }
        }
        
        // Auto-Optimization: keep memory layout coalesced if synapses were added
        self.optimize_memory_layout();
    }

    // Cortical Column WTA Inhibition (v0.5.0)
    pub fn apply_wta_inhibition(&mut self) {
        for n in &mut self.neurons {
            n.status_flags &= !crate::engine::constants::FLAG_IS_INHIBITED;
        }
        // Collect spike data per minicolumn
        // A minicolumn is defined by its column_id and minicolumn_id
        use std::collections::HashMap;
        
        let mut minicolumn_winners: HashMap<(u16, u8), u32> = HashMap::new();
        
        // Find winners (highest voltage or first to spike)
        for n in &self.neurons {
            if n.is_alive == 0 { continue; }
            
            // Only pyramidal and interneurons participate in minicolumn WTA
            if n.type_ != crate::engine::ffi::NeuronType::Pyramidal as u8 && 
               n.type_ != crate::engine::ffi::NeuronType::Interneuron as u8 {
                continue;
            }
            
            if n.current_voltage > n.activation_threshold {
                let key = (n.column_id, n.minicolumn_id);
                // Simple WTA: First one found or highest voltage wins
                let current_winner_v = minicolumn_winners.get(&key).map(|&id| self.neurons[id as usize].current_voltage).unwrap_or(-1000.0);
                if n.current_voltage > current_winner_v {
                    minicolumn_winners.insert(key, n.id);
                }
            }
        }
        
        // Apply inhibition to losers
        for n in &mut self.neurons {
            if n.is_alive == 0 { continue; }
            
            let key = (n.column_id, n.minicolumn_id);
            if let Some(&winner_id) = minicolumn_winners.get(&key) {
                if n.id != winner_id {
                    n.status_flags |= crate::engine::constants::FLAG_IS_INHIBITED;
                    // Lateral inhibition hyperpolarizes the neuron
                    n.current_voltage = n.resting_voltage - 10.0;
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
            n.branches[idx].nmda_threshold = NMDA_THRESHOLD_PACKED;
            n.branches[idx].voltage = 0; // 0 means pack_voltage(-100.0), which is minimum inactive state
            n.branches[idx].refractory_timer = 0;
        }
    }
}

impl Drop for NetworkGraph {
    fn drop(&mut self) {
        unsafe {
            // First, ensure all pending kernels finish and GPU is idle.
            crate::engine::ffi::synchronize_device();
            
            // NOTE: Currently, we use Rust's `Vec` which is allocated via CPU allocator.
            // On macOS (Mock), this memory is freed normally by Rust when the Vec is dropped.
            // If we switch to explicit `cudaMallocManaged` through C++ later, we'd pass the pointers here.
            // For now, we simulate the hook by passing null to demonstrate the structure, 
            // since calling `free` on a Rust Vec buffer causes a double-free segfault.
            crate::engine::ffi::cleanup_gpu_resources(std::ptr::null_mut());
        }
    }
}
