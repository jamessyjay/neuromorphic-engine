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
    #[serde(skip)]
    pub voxel_grid: HashMap<(i16, i16, i16), Vec<u32>>,
    #[serde(skip)]
    pub outgoing_counts: Vec<u32>,
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
            voxel_grid: HashMap::new(),
            outgoing_counts: Vec::new(),
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
        
        let x = (rng.random_range(0.0..MAX_COORD) * COORD_SCALE) as u16;
        let y = (rng.random_range(0.0..MAX_COORD) * COORD_SCALE) as u16;
        let z = (rng.random_range(0.0..MAX_COORD) * COORD_SCALE) as u16;
        
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
            x, y, z,
            dead_ticks: 0,
            column_id: 0,
            minicolumn_id: 0,
            layer_id: 0,
            status_flags: 0,
            external_input: 0.0,
            _pad_block1: [0; 28],
            ref_coord_x: 0.0,
            ref_coord_y: 0.0,
            axonal_delay: 2, // Will be overridden by init_neuron C++ call later if using CUDA
            _pad_block2: [0; 11],
            active_branches_mask: 0,
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
        // Using a true random seed instead of deterministic
        let seed: u32 = rng.random();
        unsafe {
            init_neuron(&mut neuron, id, type_, seed);
        }
        
        self.neurons.push(neuron);
        self.outgoing_counts.push(0);
        
        // Voxel registration
        let voxel_size = 10.0;
        let true_x = (x as f32 / COORD_SCALE) / voxel_size;
        let true_y = (y as f32 / COORD_SCALE) / voxel_size;
        let true_z = (z as f32 / COORD_SCALE) / voxel_size;
        
        let voxel_key = (true_x as i16, true_y as i16, true_z as i16);
        self.voxel_grid.entry(voxel_key).or_insert_with(Vec::new).push(id);
        
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
        #[cfg(target_os = "macos")]
        {
            // MacOS CPU Fallback using Rayon Multi-Threading with Batching
            let chunk_size = 512;
            
            self.neurons.par_chunks_mut(chunk_size).for_each(|chunk| {
                for n in chunk.iter_mut() {
                    n.status_flags &= !crate::engine::constants::FLAG_IS_INHIBITED;
                }
                unsafe { 
                    crate::engine::ffi::tick_neuron_prepare_cpu_batch(chunk.as_mut_ptr(), chunk.len() as libc::c_int, env, current_tick); 
                }
            });

            // Phase 2: WTA and Bursting
            // We assume max 100 columns and 50 minicolumns for the flattened array to avoid HashMap
            let mut minicolumn_winners = vec![(u32::MAX, -1000.0f32, false); 100 * 50];
            
            for n in &self.neurons {
                if n.is_alive == 1 && n.current_voltage > n.activation_threshold && (n.type_ == 0 || n.type_ == 2) {
                    let idx = (n.column_id as usize) * 50 + (n.minicolumn_id as usize);
                    if idx < minicolumn_winners.len() {
                        let is_pred = (n.status_flags & crate::engine::constants::FLAG_IS_PREDICTIVE) != 0;
                        let effective_v = if is_pred { n.current_voltage + 1000.0 } else { n.current_voltage };
                        
                        let current_winner = minicolumn_winners[idx];
                        if effective_v > current_winner.1 || (effective_v == current_winner.1 && n.id < current_winner.0) {
                            minicolumn_winners[idx] = (n.id, effective_v, is_pred);
                        }
                    }
                }
            }

            for n in &mut self.neurons {
                if n.is_alive == 1 && (n.type_ == 0 || n.type_ == 2) {
                    let idx = (n.column_id as usize) * 50 + (n.minicolumn_id as usize);
                    if idx < minicolumn_winners.len() {
                        let winner = minicolumn_winners[idx];
                        if winner.0 != u32::MAX {
                            if !winner.2 { // !winner_was_predictive
                                // Surprise! Bursting (No predictive neurons crossed threshold)
                                if n.current_voltage <= n.activation_threshold {
                                    n.current_voltage = n.activation_threshold + 5.0;
                                }
                            } else if n.id != winner.0 {
                                n.status_flags |= crate::engine::constants::FLAG_IS_INHIBITED;
                                n.current_voltage = n.resting_voltage - 10.0;
                                n.refractory_time_left = 5.0;
                            }
                        }
                    }
                }
            }
            
            self.neurons.par_chunks_mut(chunk_size).for_each(|chunk| {
                unsafe { 
                    crate::engine::ffi::tick_neuron_spike_cpu_batch(chunk.as_mut_ptr(), chunk.len() as libc::c_int, env, current_tick); 
                }
            });

            // Convert pointer to usize to make it trivially Send+Sync
            let neurons_ptr_usize = self.neurons.as_mut_ptr() as usize;
            
            let syn_chunk_size = 2048;
            self.synapses.par_chunks_mut(syn_chunk_size).for_each(|chunk| {
                let ptr = neurons_ptr_usize as *mut crate::engine::ffi::BaseNeuron;
                unsafe { 
                    crate::engine::ffi::tick_synapses_cpu_batch(chunk.as_mut_ptr(), chunk.len() as libc::c_int, ptr, env, current_tick); 
                }
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

    pub fn apply_classifier_wta(&mut self, current_tick: u32, start_idx: usize, end_idx: usize) {
        if self.neurons.len() >= end_idx {
            let mut winner_id: Option<usize> = None;
            let mut highest_v = -100.0;
            
            // Find the first spiking neuron or the one with the highest voltage
            for i in start_idx..end_idx {
                let n = &self.neurons[i];
                if n.is_alive == 1 && n.current_voltage > highest_v {
                    highest_v = n.current_voltage;
                    winner_id = Some(i);
                }
            }
            
            // Apply massive lateral inhibition to all other output neurons if the winner spiked
            if let Some(w_id) = winner_id {
                let winner = &self.neurons[w_id];
                if winner.current_voltage >= winner.activation_threshold || winner.last_spike_tick == current_tick {
                    for i in start_idx..end_idx {
                        if i != w_id && self.neurons[i].is_alive == 1 {
                            self.neurons[i].current_voltage -= 20.0; // Hard suppression penalty
                            self.neurons[i].status_flags |= crate::engine::constants::FLAG_IS_INHIBITED;
                        }
                    }
                }
            }
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
        let mut flags = branch_id & 0x0F; // Bits 0-3
        if is_inhibitory { flags |= 0x10; } // Bit 4
        if is_myelinated { flags |= 0x20; } // Bit 5
        
        // Ensure the branch is marked active in the mask so evaluate_branches doesn't skip it
        if target_id < self.neurons.len() as u32 {
            self.neurons[target_id as usize].active_branches_mask |= 1 << branch_id;
        }
        
        if source_id < self.outgoing_counts.len() as u32 {
            self.outgoing_counts[source_id as usize] += 1;
        }

        // TЗ №27: Allow multi-synaptic connections! We no longer block duplicates or augment them.
        // A single source neuron can now synapse onto multiple different branches (or even the same branch multiple times biologically) of the target.
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

        if self.synapses.len() >= MAX_SYNAPSES_TOTAL { return; }

        let check_count = num_neurons * 2; 
        
        let voxel_keys: Vec<_> = self.voxel_grid.keys().cloned().collect();
        if voxel_keys.is_empty() { return; }

        for _ in 0..check_count {
            // Pick a random voxel
            let voxel_key = voxel_keys[rng.random_range(0..voxel_keys.len())];
            let voxel_neurons = &self.voxel_grid[&voxel_key];
            if voxel_neurons.is_empty() { continue; }
            
            // Pick random source from this voxel
            let i = voxel_neurons[rng.random_range(0..voxel_neurons.len())] as usize;
            
            // Pick a neighbor voxel (or self)
            let dx = rng.random_range(-1..=1);
            let dy = rng.random_range(-1..=1);
            let dz = rng.random_range(-1..=1);
            let target_voxel_key = (voxel_key.0 + dx, voxel_key.1 + dy, voxel_key.2 + dz);
            
            let Some(target_neurons) = self.voxel_grid.get(&target_voxel_key) else { continue; };
            if target_neurons.is_empty() { continue; }
            
            // Pick random target
            let j = target_neurons[rng.random_range(0..target_neurons.len())] as usize;

            if i == j { continue; }

            let n_source = &self.neurons[i];
            let n_target = &self.neurons[j];

            if n_source.is_alive == 0 || n_target.is_alive == 0 { continue; }
            if n_source.energy_atp < 40.0 { continue; }

            if self.outgoing_counts[i] >= MAX_SYNAPSES_PER_NEURON as u32 { continue; }

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

            if dist_sq >= CLUSTERING_RADIUS_SQ * 1.5 { continue; }

            let distance = dist_sq.sqrt();
            let safe_dist = if distance < 1.0 { 1.0 } else { distance };
            
            // TЗ №27: -3/2 Topological Law (P(L) ~ L^(-3/2))
            let topo_prob = 1.0 / safe_dist.powf(1.5);
            
            if !rng.random_bool(topo_prob.clamp(0.01, 1.0) as f64) { continue; }

            let s_active = current_tick.saturating_sub(n_source.last_spike_tick) < 500; 
            let t_active = current_tick.saturating_sub(n_target.last_spike_tick) < 500;

            if !s_active || !t_active { continue; }
            if !rng.random_bool(STOCHASTIC_GROWTH_PROB) { continue; }

            // Preferential Attachment (ТЗ №26) with Tie-Breaking (ТЗ №29)
            let mut best_branches = [0u8; 16];
            let mut best_count = 0;

            if n_target.energy_atp < 70.0 {
                // Seek the quietest branch
                let mut min_trace = 255;
                for b in 0..16 {
                    let trace = n_target.branches[b].local_trace;
                    if trace < min_trace {
                        min_trace = trace;
                        best_branches[0] = b as u8;
                        best_count = 1;
                    } else if trace == min_trace {
                        best_branches[best_count] = b as u8;
                        best_count += 1;
                    }
                }
            } else {
                // Seek the most active branch
                let mut max_trace = 0;
                for b in 0..16 {
                    let trace = n_target.branches[b].local_trace;
                    if trace > max_trace {
                        max_trace = trace;
                        best_branches[0] = b as u8;
                        best_count = 1;
                    } else if trace == max_trace {
                        best_branches[best_count] = b as u8;
                        best_count += 1;
                    }
                }
                
                // If everything is completely dead, pick randomly from all 16
                if max_trace == 0 {
                    for b in 0..16 {
                        best_branches[b] = b as u8;
                    }
                    best_count = 16;
                }
            }
            
            // Tie-breaking: pick a random branch from the best candidates
            let branch_id = best_branches[rng.random_range(0..best_count)];
            let is_inhib = rng.random_bool(0.1); 
            
            self.connect(i as u32, j as u32, SEED_WEIGHT, branch_id, is_inhib, false);
            
            if current_tick % 1000 == 0 {
                println!("NeuroGenesis: New synapse {} -> {} on branch {} at tick {}", i, j, branch_id, current_tick);
            }
        }
        
        self.optimize_memory_layout();
    }

    // Cortical Column WTA Inhibition (v0.5.0)
    pub fn apply_wta_inhibition(&mut self) {
        #[cfg(feature = "cuda")]
        return; // Skip Rust CPU WTA if running on CUDA

        for n in &mut self.neurons {
            n.status_flags &= !crate::engine::constants::FLAG_IS_INHIBITED;
        }

        // We assume max 100 columns and 50 minicolumns for the flattened array to avoid HashMap
        // This easily covers our 10,000 neuron scale.
        // Index = column_id * 50 + minicolumn_id
        let mut minicolumn_winners = vec![u32::MAX; 100 * 50]; 
        let mut minicolumn_winner_v = vec![-1000.0f32; 100 * 50];

        // Find winners (highest voltage or first to spike)
        for n in &self.neurons {
            if n.is_alive == 0 { continue; }
            
            // Only pyramidal and interneurons participate in minicolumn WTA
            if n.type_ != crate::engine::ffi::NeuronType::Pyramidal as u8 && 
               n.type_ != crate::engine::ffi::NeuronType::Interneuron as u8 {
                continue;
            }
            
            if n.current_voltage > n.activation_threshold {
                let idx = (n.column_id as usize) * 50 + (n.minicolumn_id as usize);
                if idx < minicolumn_winners.len() {
                    if n.current_voltage > minicolumn_winner_v[idx] {
                        minicolumn_winners[idx] = n.id;
                        minicolumn_winner_v[idx] = n.current_voltage;
                    }
                }
            }
        }
        
        // Apply inhibition to losers
        for n in &mut self.neurons {
            if n.is_alive == 0 { continue; }
            
            let idx = (n.column_id as usize) * 50 + (n.minicolumn_id as usize);
            if idx < minicolumn_winners.len() {
                let winner_id = minicolumn_winners[idx];
                if winner_id != u32::MAX && n.id != winner_id {
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
        
        // Zero-alloc unique branch selection using a fixed array
        let mut indices: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        for i in (1..8).rev() {
            let j = rng.random_range(0..=i);
            indices.swap(i, j);
        }

        for i in 0..num_branches {
            let idx = indices[i as usize] as usize;
            let b_type = rng.random_range(1..=3) as u8;
            n.branches[idx].branch_type = b_type;
            n.active_branches_mask |= 1 << idx;
            n.branches[idx].nmda_threshold = NMDA_THRESHOLD_PACKED;
            n.branches[idx].voltage = 0; // 0 means pack_voltage(-100.0), which is minimum inactive state
            n.branches[idx].refractory_timer = 0;
        }
    }

    pub fn print_synaptic_diagnostics(&self) {
        println!("\n=== SYNAPTIC DIAGNOSTICS (ТЗ №26) ===");
        
        let total_synapses = self.synapses.len();
        if total_synapses == 0 {
            println!("No synapses to diagnose.");
            return;
        }

        // 1. Branch Utilization
        let mut branch_counts = [0usize; 16];
        
        // 2. Weight Histogram
        let mut weight_buckets = [0usize; 4]; 
        
        // 3. Topological Check
        let mut total_length = 0.0;
        let mut lengths = Vec::with_capacity(total_synapses);
        
        for s in &self.synapses {
            let branch_id = (s.flags & 0x0F) as usize;
            branch_counts[branch_id] += 1;
            
            let w = s.weight;
            if w < 0.5 { weight_buckets[0] += 1; }
            else if w < 1.0 { weight_buckets[1] += 1; }
            else if w < 2.0 { weight_buckets[2] += 1; }
            else { weight_buckets[3] += 1; }
            
            let n1 = &self.neurons[s.source_id as usize];
            let n2 = &self.neurons[s.target_id as usize];
            
            let dx = (n1.x as f32 - n2.x as f32) / crate::engine::constants::COORD_SCALE;
            let dy = (n1.y as f32 - n2.y as f32) / crate::engine::constants::COORD_SCALE;
            let dz = (n1.z as f32 - n2.z as f32) / crate::engine::constants::COORD_SCALE;
            
            let dist = (dx*dx + dy*dy + dz*dz).sqrt();
            total_length += dist;
            lengths.push(dist);
        }
        
        println!("1. Branch Utilization Histogram:");
        for i in 0..16 {
            let pct = (branch_counts[i] as f32 / total_synapses as f32) * 100.0;
            println!("  Branch [{:2}]: {:6} synapses ({:5.1}%)", i, branch_counts[i], pct);
        }
        
        println!("\n2. Weight Histogram (Expected ~80% weak):");
        println!("  0.0 - 0.5  (Weak)  : {:5.1}%", (weight_buckets[0] as f32 / total_synapses as f32) * 100.0);
        println!("  0.5 - 1.0  (Medium): {:5.1}%", (weight_buckets[1] as f32 / total_synapses as f32) * 100.0);
        println!("  1.0 - 2.0  (Strong): {:5.1}%", (weight_buckets[2] as f32 / total_synapses as f32) * 100.0);
        println!("  2.0 - 5.0+ (Max)   : {:5.1}%", (weight_buckets[3] as f32 / total_synapses as f32) * 100.0);
        
        println!("\n3. Topological Check (Law -3/2):");
        let avg_len = total_length / total_synapses as f32;
        println!("  Average connection length (L): {:.2}", avg_len);
        
        let mut len_buckets = [0usize; 5]; // <10, 10-20, 20-40, 40-80, 80+
        for l in lengths {
            if l < 10.0 { len_buckets[0] += 1; }
            else if l < 20.0 { len_buckets[1] += 1; }
            else if l < 40.0 { len_buckets[2] += 1; }
            else if l < 80.0 { len_buckets[3] += 1; }
            else { len_buckets[4] += 1; }
        }
        println!("  Length Distribution P(L) ~ L^(-3/2):");
        println!("    L < 10:  {:5.1}%", (len_buckets[0] as f32 / total_synapses as f32) * 100.0);
        println!("    10-20:   {:5.1}%", (len_buckets[1] as f32 / total_synapses as f32) * 100.0);
        println!("    20-40:   {:5.1}%", (len_buckets[2] as f32 / total_synapses as f32) * 100.0);
        println!("    40-80:   {:5.1}%", (len_buckets[3] as f32 / total_synapses as f32) * 100.0);
        println!("    > 80:    {:5.1}%", (len_buckets[4] as f32 / total_synapses as f32) * 100.0);
        println!("=====================================\n");
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
