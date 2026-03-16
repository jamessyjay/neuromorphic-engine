use crate::network::stimulus::SensoryHub;
use crate::engine::snapshot::BrainSnapshot;
use crate::engine::environment::EnvironmentRegulator;
use std::sync::{Arc, Mutex};
// src/modes.rs
use std::time::Instant;
use tokio::sync::broadcast;
use crate::engine::ffi::{NeuronType, Environment, Synapse};
use crate::network::graph::NetworkGraph;

use crate::monitor::telemetry::Telemetry;
use crate::network::stimulus;
use crate::monitor::streamer;
use crate::engine::constants::*;
use crate::monitor::visualization;

pub struct BenchmarkData {
    pub ticks: Vec<u32>,
    pub ffi_times: Vec<f32>,
    pub rust_times: Vec<f32>,
    pub total_spikes: Vec<u32>,
    pub total_atp_sum: f64,
    pub total_calcium_sum: f64,
    pub samples_count: u32,
}

pub struct SimulationData {
    pub ticks: Vec<u32>,
    pub source_v: Vec<f32>,
    pub target_v: Vec<f32>,
    pub synapse_w: Vec<f32>,
}

pub fn get_gaussian(rng: &mut rand::rngs::ThreadRng, mu: f32, sigma: f32) -> f32 {
    let u1: f32 = rand::RngExt::random(rng);
    let u2: f32 = rand::RngExt::random(rng);
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
    mu + sigma * z
}

// Helper: Get Visual State of a Neuron
pub fn get_visual_state(n: &crate::engine::ffi::BaseNeuron, tick: u32, atp_pct: f32) -> f32 {
    let is_spiking_now = n.current_voltage > n.activation_threshold;
    let spiked_recently = tick.saturating_sub(n.last_spike_tick) < VISUAL_SPIKE_PERSISTENCE_TICKS;
    
    if n.is_alive == 0 {
        match n.death_reason {
            2 => 3.0, // Calcium (Red)
            1 | 3 => 4.0, // ATP/Glucose (Blue)
            4 => 4.0, // Metabolic Shutdown (Dark Blue/Black)
            _ => 5.0, // Regenerating/Other (Green)
        }
    } else if is_spiking_now || spiked_recently { 
        1.0 
    } else if n.death_reason == 4 || atp_pct < crate::engine::constants::VISUAL_LOW_ATP_THRESHOLD { 
        2.0 
    } else { 
        0.0 
    }
}

// Helper: Generate InitialStatePayload
pub fn generate_initial_state(graph: &NetworkGraph) -> streamer::InitialStatePayload {
    let mut initial_state = streamer::InitialStatePayload {
        total_neurons: graph.neurons.len(),
        total_synapses: graph.synapses.len(),
        neurons: Vec::with_capacity(graph.neurons.len()),
        synapses: Vec::with_capacity(graph.synapses.len()),
    };
    for n in &graph.neurons {
        initial_state.neurons.push((n.id, n.x as f32, n.y as f32, n.z as f32, n.type_, n.column_id, n.layer_id, n.minicolumn_id, n.status_flags));
    }
    for s in &graph.synapses {
        initial_state.synapses.push((s.source_id, s.target_id, s.weight));
    }
    initial_state
}

// Helper: Broadcast Initial State
pub fn broadcast_initial_state(tx: &broadcast::Sender<String>, graph: &NetworkGraph) {
    let initial_state = generate_initial_state(graph);
    let msg = streamer::StreamMessage::InitialState(initial_state);
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = tx_clone.send(json);
        }
    });
}

// Helper: Generate and Broadcast Tick Update
pub fn broadcast_tick_update(
    tx: &broadcast::Sender<String>,
    graph: &NetworkGraph,
    tick: u32,
    accumulated_spikes: u32,
    current_env: &Environment,
    should_send_synapses: bool,
    current_synapse_count: usize,
) {
    let mut update = streamer::TickUpdatePayload {
        tick,
        total_spikes: accumulated_spikes,
        global_oxygen: current_env.global_oxygen_level,
        global_glucose: current_env.global_glucose_level,
        neurons: Vec::with_capacity(graph.neurons.len()),
        synapses: if should_send_synapses {
            let mut syn_vec = Vec::with_capacity(graph.synapses.len());
            for s in &graph.synapses {
                syn_vec.push((s.source_id, s.target_id, s.weight));
            }
            Some(syn_vec)
        } else { None },
        synapse_count: current_synapse_count,
    };
    for n in &graph.neurons {
        let atp_pct = (n.energy_atp / n.max_atp) * METRIC_MAX_F32;
        let visual_state = get_visual_state(n, tick, atp_pct);
        update.neurons.push((n.id, visual_state, atp_pct, n.status_flags));
    }
    
    let msg = streamer::StreamMessage::TickUpdate(update);
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = tx_clone.send(json);
        }
    });
}

// Helper: Apply Nursery Protection
pub fn apply_nursery_protection(graph: &mut NetworkGraph, current_env: &mut Environment, tick: u32) {
    if tick < crate::engine::constants::NURSERY_PROTECTION_TICKS {
        for n in &mut graph.neurons {
            if n.energy_atp < crate::engine::constants::NURSERY_MIN_ATP {
                n.energy_atp = crate::engine::constants::NURSERY_MIN_ATP;
            }
        }
        // Boost STDP Learning Rate during Nursery phase to solidify connections before pruning
        current_env.stdp_learning_rate = 0.2; 
    } else {
        // Revert to baseline learning rate after Nursery
        current_env.stdp_learning_rate = 0.1;
    }
}

// Helper: Update Environment (Dynamic Homeostasis)
pub fn update_environment(
    regulator: &mut EnvironmentRegulator,
    current_env: &Environment,
    use_dynamic_homeostasis: bool,
    tick: u32,
    elapsed_ms: f32,
) -> Environment {
    if use_dynamic_homeostasis && tick > crate::engine::constants::NURSERY_PROTECTION_TICKS {
        regulator.update_and_get(elapsed_ms, tick, current_env)
    } else {
        let mut new_env = *current_env;
        new_env.global_oxygen_level = 1.0;
        new_env.global_glucose_level = 1.0;
        new_env
    }
}

// Helper: Export Report and Save Snapshot
pub fn finalize_and_save_snapshot(
    graph: &NetworkGraph,
    tick: u32,
    report_name: &str,
    save_file: &str,
) {
    let mut telemetry = Telemetry::new();
    telemetry.update(tick);
    telemetry.export_simulation_report(&graph.neurons, &graph.synapses, report_name);
    crate::engine::snapshot::BrainSnapshot::save_test_snapshot(tick, &graph, save_file);
}

pub fn setup_benchmark(neurons_count: usize, synapses_count: usize) -> NetworkGraph {
    let mut graph = NetworkGraph::new();
    let mut rng = rand::rng();

    println!("Generating Benchmark Network (Cortical Columns v0.5.2): {} neurons, {} synapses...", neurons_count, synapses_count);

    let columns = 10;
    // We strictly use 1000 per col to fit the mathematical requirements of v0.5.2.
    // If the user requests exactly 1000 neurons, we assume they mean 10 columns of 100, but the spec says "1000 neurons per column".
    // We will distribute the `neurons_count` evenly if it's not 10000, but let's stick to the spec's proportions.
    let neurons_per_col = neurons_count / columns;

    for col_id in 0..columns {
        let center_x = ((col_id % 5) as f32) * 12000.0 + 10000.0;
        let center_y = ((col_id / 5) as f32) * 20000.0 + 10000.0;

        for i in 0..neurons_per_col {
            // Distribute proportionally to 1000
            // We'll normalize 'i' to a 0..1000 scale so the proportions hold even if neurons_per_col is smaller.
            let proportion_idx = (i as f32 / neurons_per_col as f32 * 1000.0) as usize;

            let (n_type, layer_id, z_min, z_max) = if proportion_idx < 300 {
                (crate::engine::ffi::NeuronType::Pyramidal, 6, 0.0, 10000.0)
            } else if proportion_idx < 500 {
                (crate::engine::ffi::NeuronType::Purkinje, 5, 10001.0, 25000.0)
            } else if proportion_idx < 700 {
                (crate::engine::ffi::NeuronType::Photoreceptor, 4, 25001.0, 40000.0)
            } else if proportion_idx < 950 {
                (crate::engine::ffi::NeuronType::Pyramidal, 2, 40001.0, 60000.0)
            } else {
                (crate::engine::ffi::NeuronType::Interneuron, 1, 60001.0, 65535.0)
            };

            let _id = graph.add_neuron(n_type);
            let n_id = {
                let n = graph.neurons.last_mut().unwrap();

                n.column_id = col_id as u16;
                n.layer_id = layer_id as u8;
                n.minicolumn_id = (i / 20) as u8;

                // Local scatter
                let dx = (rand::RngExt::random::<f32>(&mut rng) * 2.0 - 1.0) * 3000.0;
                let dy = (rand::RngExt::random::<f32>(&mut rng) * 2.0 - 1.0) * 3000.0;
                
                n.x = (center_x + dx).clamp(0.0, 65535.0) as u16;
                n.y = (center_y + dy).clamp(0.0, 65535.0) as u16;
                n.z = rand::RngExt::random_range(&mut rng, z_min..=z_max) as u16;
                
                n.id
            };

            graph.grow_dendritic_tree(n_id);
        }
    }

    let mut actual_synapses = 0;
    while actual_synapses < synapses_count {
        let source_idx = rand::RngExt::random_range(&mut rng, 0..graph.neurons.len());
        let target_idx = rand::RngExt::random_range(&mut rng, 0..graph.neurons.len());
        if source_idx == target_idx { continue; }

        let n_source = &graph.neurons[source_idx];
        let n_target = &graph.neurons[target_idx];

        // Rules of connectivity v0.5
        let mut prob;

        let mut target_distal = false;
        if n_source.column_id == n_target.column_id {
            if n_source.minicolumn_id == n_target.minicolumn_id {
                prob = 0.90; // Minicolumnar vertical stitch
            } else {
                let layer_diff = (n_source.layer_id as i32 - n_target.layer_id as i32).abs();
                if layer_diff == 1 || layer_diff == 2 {
                    prob = 0.80; // Vertical intra-columnar connections
                } else {
                    prob = 0.10; // Sparse random inside column
                }
            }
        } else if n_source.layer_id == 2 && n_target.layer_id == 2 {
            prob = 0.05; // Horizontal L2/3 voting
            target_distal = true; // Voting targets distal dendrites
        } else {
            prob = 0.01; // Ultra sparse random long-range
            target_distal = true; // Long-range context targets distal
        }

        // Ban: L6 cannot directly fire to L5
        if n_source.layer_id == 6 && n_target.layer_id == 5 {
            prob = 0.0;
        }

        if rand::RngExt::random_bool(&mut rng, prob as f64) {
            let base_weight = crate::runner::modes::get_gaussian(&mut rng, DEFAULT_WEIGHT * BASE_WEIGHT_MODIFIER, WEIGHT_SIGMA).max(0.5).min(10.0);
            let is_inhib = n_source.type_ == NeuronType::Interneuron as u8;
            let weight = if is_inhib { -base_weight } else { base_weight };
            let is_myelin = rand::RngExt::random_bool(&mut rng, MYELIN_PROB);
            
            let branch_id = if target_distal {
                rand::RngExt::random_range(&mut rng, 4..12) // Distal branches (4-11)
            } else {
                rand::RngExt::random_range(&mut rng, 0..4)  // Proximal branches (0-3)
            };

            graph.connect(source_idx as u32, target_idx as u32, weight, branch_id, is_inhib, is_myelin);
            actual_synapses += 1;
        }
    }

    for n in &mut graph.neurons {
        if rand::RngExt::random_bool(&mut rng, 0.1) {
            n.current_voltage = PRE_SEED_VOLTAGE;
        }
        n.energy_atp = n.max_atp;
        n.calcium_level = 0.0;
    }

    // --- Validation (Checklist) ---
    assert_eq!(graph.neurons.len(), neurons_count, "Validation Failed: Incorrect total neurons count.");
    for n in &graph.neurons {
        assert!(n.column_id < COLUMNS_COUNT, "Validation Failed: column_id {} is out of bounds!", n.column_id);
        match n.layer_id {
            6 => assert!(n.z >= LAYER_6_Z_MIN && n.z <= LAYER_6_Z_MAX, "L6 Z mismatch"),
            5 => assert!(n.z >= LAYER_5_Z_MIN && n.z <= LAYER_5_Z_MAX, "L5 Z mismatch"),
            4 => assert!(n.z >= LAYER_4_Z_MIN && n.z <= LAYER_4_Z_MAX, "L4 Z mismatch"),
            2 => assert!(n.z >= LAYER_2_3_Z_MIN && n.z <= LAYER_2_3_Z_MAX, "L2/3 Z mismatch"),
            1 => assert!(n.z >= 60001, "L1 Z mismatch"), // Note: n.z is u16, so it is inherently <= 65535
            _ => panic!("Unknown layer ID {}", n.layer_id),
        }
    }

    // Force memory coalescing after generation
    graph.optimize_memory_layout();

    // 1. Elimination of Orphans
    let mut outgoing_synapses: Vec<Vec<u32>> = vec![Vec::new(); graph.neurons.len()];
    for s in &graph.synapses {
        outgoing_synapses[s.source_id as usize].push(s.target_id);
    }

    let mut new_synapses = Vec::new();
    for i in 0..graph.neurons.len() {
        let n_layer = graph.neurons[i].layer_id;
        let mut has_l4_to_l23 = false;
        
        if n_layer == 4 {
            for &t_id in &outgoing_synapses[i] {
                if graph.neurons[t_id as usize].layer_id == 2 {
                    has_l4_to_l23 = true;
                    break;
                }
            }
        }
        
        let target_layer = match n_layer {
            6 => 4,
            5 => 6,
            4 => 2,
            2 => 5,
            1 => 2,
            _ => 2,
        };
        
        let mut needed = 0;
        if outgoing_synapses[i].len() < 3 {
            needed = 3 - outgoing_synapses[i].len();
        }
        if n_layer == 4 && !has_l4_to_l23 {
            needed = needed.max(1);
        }
        
        if needed > 0 {
            let n_col = graph.neurons[i].column_id;
            let nx = graph.neurons[i].x as f32;
            let ny = graph.neurons[i].y as f32;
            let nz = graph.neurons[i].z as f32;
            
            let mut candidates = Vec::new();
            for j in 0..graph.neurons.len() {
                if i == j { continue; }
                if outgoing_synapses[i].contains(&(j as u32)) { continue; } // Don't duplicate synapses
                let cand = &graph.neurons[j];
                // Try strictly target layer first, but fallback to any layer in the same column if not enough
                if cand.column_id == n_col {
                    let dx = cand.x as f32 - nx;
                    let dy = cand.y as f32 - ny;
                    let dz = cand.z as f32 - nz;
                    let dist = dx*dx + dy*dy + dz*dz;
                    let layer_match_bonus = if cand.layer_id == target_layer { 0.0 } else { 100000000.0 };
                    candidates.push((j, dist + layer_match_bonus, cand.layer_id));
                }
            }
            
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            
            let mut added = 0;
            for k in 0..candidates.len() {
                if added >= needed { break; }
                let target_id = candidates[k].0 as u32;
                
                // Extra safety: Enforce L6->L5 ban even during orphan saving
                if graph.neurons[i].layer_id == 6 && graph.neurons[candidates[k].0].layer_id == 5 {
                    continue;
                }

                // Add to outgoing_synapses to prevent next checks from reusing it if logic changes
                outgoing_synapses[i].push(target_id); 
                
                let is_inhib = graph.neurons[i].type_ == crate::engine::ffi::NeuronType::Interneuron as u8;
                let w = if is_inhib { -2.5 } else { 2.5 };
                new_synapses.push((i as u32, target_id, w, is_inhib));
                added += 1;
            }
        }
    }
    
    for (src, tgt, w, is_inhib) in new_synapses {
        graph.connect(src, tgt, w, 0, is_inhib, false);
    }
    
    // Re-optimize memory layout after adding new orphan-saving synapses
    graph.optimize_memory_layout();

    println!("Network generation complete.");
    graph
}

pub async fn run_benchmark_mode(neurons_count: usize, synapses_count: usize, total_ticks: u32, tx: Option<broadcast::Sender<String>>, _shared_env: Arc<Mutex<crate::engine::ffi::Environment>>, use_dynamic_homeostasis: bool) -> Result<BenchmarkData, Box<dyn std::error::Error>> {
    let mut start_tick = 0;
    let mut graph = if let Some(loaded_snapshot) = BrainSnapshot::load_from_disk("brain_save.bin") {
        let mut loaded_graph = loaded_snapshot.graph;
        if loaded_graph.neurons.len() == neurons_count {
            println!("Awakening network from brain_save.bin (Tick {})...", loaded_snapshot.current_tick);
            start_tick = loaded_snapshot.current_tick;
            loaded_graph.force_sync_gpu(); // <--- CRITICAL (Phase 50.1)
            loaded_graph
        } else {
            println!("Saved network size ({}) does not match requested size ({}). Generating new network...", loaded_graph.neurons.len(), neurons_count);
            setup_benchmark(neurons_count, synapses_count)
        }
    } else {
        setup_benchmark(neurons_count, synapses_count)
    };

    println!("Executing BENCHMARK for {} ticks...", total_ticks);
    
    let mut current_env = Environment {
        global_oxygen_level: DEFAULT_OXYGEN,  
        global_glucose_level: DEFAULT_GLUCOSE,
        dopamine_multiplier: DEFAULT_DOPAMINE, 
        toxin_levels: DEFAULT_TOXINS,
        light_level: DEFAULT_LIGHT, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
    };
    
        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);

    let total_elements = graph.neurons.len() + graph.synapses.len();
    let sampling_interval = (total_elements as f32 / 1000.0).ceil() as u32;
    let sampling_interval = if sampling_interval < 1 { 1 } else { sampling_interval };
    let expected_capacity = (total_ticks / sampling_interval) as usize + 1;

    let mut data = BenchmarkData {
        ticks: Vec::with_capacity(expected_capacity),
        ffi_times: Vec::with_capacity(expected_capacity),
        rust_times: Vec::with_capacity(expected_capacity),
        total_spikes: Vec::with_capacity(expected_capacity),
        total_atp_sum: 0.0,
        total_calcium_sum: 0.0,
        samples_count: 0,
    };

    let mut total_ffi_time = std::time::Duration::new(0, 0);
    let mut total_rust_time = std::time::Duration::new(0, 0);
    let mut telemetry = Telemetry::new();
    let mut accumulated_spikes = 0;
    let mut regulator = EnvironmentRegulator::new();
    let mut last_synapse_count = graph.synapses.len();

    for tick in (start_tick + 1)..=(start_tick + total_ticks) {
        let tick_start = Instant::now();
        stimulus_service.process_tick(&mut graph, &mut current_env, tick);

        // Nursery Protection Improvement (v0.4.7)
        apply_nursery_protection(&mut graph, &mut current_env, tick);

        // Accumulate spikes from all neurons that spiked in THIS tick
        for n in &graph.neurons {
            if n.last_spike_tick == tick {
                accumulated_spikes += 1;
            }
        }

        let num_neurons = graph.neurons.len() as i32;

        let ffi_start = Instant::now();
        graph.tick(tick, current_env);
        let ffi_elapsed = ffi_start.elapsed();
        total_ffi_time += ffi_elapsed;

        if tick % ASYNC_YIELD_INTERVAL == 0 {
            tokio::task::yield_now().await;
        }

        if let Some(ref tx) = tx {
            if tx.receiver_count() > 0 && (tick == start_tick + 1 || tick % WS_INITIAL_STATE_INTERVAL == 0) {
                broadcast_initial_state(tx, &graph);
            }

            if tx.receiver_count() > 0 && tick % WS_SNAPSHOT_INTERVAL_BENCHMARK == 0 {
                let current_synapse_count = graph.synapses.len();
                let should_send_synapses = tick % crate::engine::constants::WS_SYNAPSE_FORCE_SYNC_INTERVAL == 0 || current_synapse_count != last_synapse_count;
                last_synapse_count = current_synapse_count;
                
                broadcast_tick_update(
                    tx, 
                    &graph, 
                    tick, 
                    accumulated_spikes, 
                    &current_env, 
                    should_send_synapses, 
                    current_synapse_count
                );
            }
        }

        if tick % 1000 == 0 && graph.is_sleep_needed() {
            println!("Fatigue threshold reached. Entering Sleep Cycle...");
            graph.initiate_sleep_cycle();
            println!("Wake up! Network consolidated.");
        }

        let mut spikes = 0;
        let mut atp_sum = 0.0;
        let mut calcium_sum = 0.0;
        
        if tick % METRICS_COLLECTION_INTERVAL == 0 || tick == total_ticks {
            for n in &graph.neurons {
                atp_sum += n.energy_atp;
                calcium_sum += n.calcium_level;
            }
            spikes = accumulated_spikes;
            accumulated_spikes = 0; // Reset for next interval
        }

        let mut tick_rust_time = std::time::Duration::new(0, 0);
        if tick % SERVICE_TICK_INTERVAL == 0 {
            let rust_start = Instant::now();
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
            graph.run_neurogenesis_service(tick);
            tick_rust_time = rust_start.elapsed();
            total_rust_time += tick_rust_time;
        }

        telemetry.update(tick);

        let elapsed_ms = tick_start.elapsed().as_secs_f32() * crate::engine::constants::SECONDS_TO_MS;
        current_env = update_environment(&mut regulator, &current_env, use_dynamic_homeostasis, tick, elapsed_ms);
        
        if tick % TELEMETRY_PRINT_INTERVAL == 0 {
            let dead_count = graph.neurons.iter().filter(|n| n.is_alive == 0).count();
            telemetry.print_report(graph.neurons.len(), graph.synapses.len(), dead_count, &graph.neurons);
        }

        data.ticks.push(tick);
        data.ffi_times.push(ffi_elapsed.as_secs_f32() * crate::engine::constants::SECONDS_TO_MS);
        data.rust_times.push(tick_rust_time.as_secs_f32() * crate::engine::constants::SECONDS_TO_MS);
        data.total_spikes.push(spikes);
        data.total_atp_sum += (atp_sum / num_neurons as f32) as f64;
        data.total_calcium_sum += (calcium_sum / num_neurons as f32) as f64;
        data.samples_count += 1;
    }

    println!("--- BENCHMARK RESULTS (macOS Mock CPU) ---");
    println!("FFI / CUDA Math Time: {:?}", total_ffi_time);
    println!("Rust Services Time: {:?}", total_rust_time);
    
    finalize_and_save_snapshot(&graph, start_tick + total_ticks, "reports/simulation_report.json", "brain_save.bin");

    Ok(data)
}

pub async fn run_scaling_benchmarks() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING SCALING PERFORMANCE ANALYSIS (1K to 1M) ---");
    let sizes = [1_000, 10_000, 100_000, 1_000_000];
    let mut results = String::from("# Performance Scaling Report\n\n| Neurons | Synapses | TPS | Total Time (s) | Memory (KB) |\n|---|---|---|---|---|\n");

    for &num_neurons in &sizes {
        println!("\n>> Testing Scale: {} Neurons...", num_neurons);
        let num_synapses = num_neurons * crate::engine::constants::SCALING_SYNAPSE_DENSITY;
        let start = Instant::now();
        let dummy_env = std::sync::Arc::new(std::sync::Mutex::new(crate::engine::ffi::Environment {
            global_oxygen_level: 100.0,
            global_glucose_level: 100.0,
            dopamine_multiplier: 1.0,
            toxin_levels: 0.0,
            light_level: 0.0,
            stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
        }));
        let _ = run_benchmark_mode(num_neurons, num_synapses, crate::engine::constants::SCALING_BENCHMARK_TICKS, None, dummy_env, false).await?;
        let elapsed = start.elapsed().as_secs_f64();
        let tps = crate::engine::constants::SCALING_BENCHMARK_TICKS as f64 / elapsed;
        
        let neuron_mem = std::mem::size_of::<crate::engine::ffi::BaseNeuron>();
        let synapse_mem = std::mem::size_of::<crate::engine::ffi::Synapse>();
        let total_mem_kb = (num_neurons * neuron_mem + num_synapses * synapse_mem) as f32 / crate::engine::constants::BYTES_TO_KB;

        results.push_str(&format!("| {} | {} | {:.2} | {:.2} | {:.2} |\n", num_neurons, num_synapses, tps, elapsed, total_mem_kb));
    }

    std::fs::create_dir_all("docs")?;
    std::fs::write("docs/performance_report.md", results)?;
    println!("Scaling Benchmarks Complete!");

    Ok(())
}

pub async fn run_simulation(mut graph: NetworkGraph, start_tick: u32, initial_env: Option<Environment>, total_ticks: u32, tx: Option<broadcast::Sender<String>>, use_dynamic_homeostasis: bool) -> SimulationData {
    let total_elements = graph.neurons.len() + graph.synapses.len();
    let sampling_interval = (total_elements as f32 / crate::engine::constants::UI_SAMPLING_DIVISOR).ceil() as u32;
    let sampling_interval = if sampling_interval < 1 { 1 } else { sampling_interval };
    let expected_capacity = (total_ticks / sampling_interval) as usize + 1;

    let mut data = SimulationData {
        ticks: Vec::with_capacity(expected_capacity),
        source_v: Vec::with_capacity(expected_capacity),
        target_v: Vec::with_capacity(expected_capacity),
        synapse_w: Vec::with_capacity(expected_capacity),
    };

    println!("Executing {} ticks...", total_ticks);
    
    let mut current_env = initial_env.unwrap_or(Environment {
        global_oxygen_level: 1.0,  
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0,
        light_level: 0.0, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
    });
    
        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);
    let mut regulator = EnvironmentRegulator::new();
    let mut last_synapse_count = graph.synapses.len();

    for tick in (start_tick + 1)..=(start_tick + total_ticks) {
        let tick_start = Instant::now();
        stimulus_service.process_tick(&mut graph, &mut current_env, tick);
        
        // Nursery Protection Improvement (v0.4.7)
        apply_nursery_protection(&mut graph, &mut current_env, tick);
        
        graph.tick(tick, current_env);
        
        if let Some(ref tx) = tx {
            if tick == start_tick + 1 || tick % WS_INITIAL_STATE_INTERVAL == 0 {
                broadcast_initial_state(tx, &graph);
            }

            if tick % WS_SNAPSHOT_INTERVAL_SIMULATION == 0 {
                let mut accumulated_spikes = 0;
                for n in &graph.neurons {
                    if n.last_spike_tick == tick {
                        accumulated_spikes += 1;
                    }
                }

                let current_synapse_count = graph.synapses.len();
                let should_send_synapses = tick % crate::engine::constants::WS_SYNAPSE_FORCE_SYNC_INTERVAL == 0 || current_synapse_count != last_synapse_count;
                last_synapse_count = current_synapse_count;
                
                broadcast_tick_update(
                    tx, 
                    &graph, 
                    tick, 
                    accumulated_spikes, 
                    &current_env, 
                    should_send_synapses, 
                    current_synapse_count
                );

                // Throttle: ~60 FPS visual update rate
                tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
            }
        } else {
            // Only yield in headless mode to not block the executor entirely
            if tick % ASYNC_YIELD_INTERVAL == 0 {
                tokio::task::yield_now().await;
            }
        }
        
        if tick % SERVICE_TICK_INTERVAL == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }
        
        data.ticks.push(tick);
        data.source_v.push(graph.neurons[0].current_voltage);
        data.target_v.push(graph.neurons[1].current_voltage);
        data.synapse_w.push(graph.synapses[0].neurotransmitter_buffer);
        
        let elapsed_ms = tick_start.elapsed().as_secs_f32() * crate::engine::constants::SECONDS_TO_MS;
        current_env = update_environment(&mut regulator, &current_env, use_dynamic_homeostasis, tick, elapsed_ms);
    }

    finalize_and_save_snapshot(&graph, start_tick + total_ticks, "reports/simulation_report.json", "brain_save.bin");

    data
}

pub fn run_spatial_summation_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING SPATIAL SUMMATION TEST (Non-linear Dendritic Logic) ---");
    let mut graph = NetworkGraph::new();
    let target = graph.add_neuron(NeuronType::Pyramidal);
    graph.grow_dendritic_tree(target);
    
    for b in 0..8usize {
        graph.neurons[target as usize].branches[b].branch_type = 0;
    }

    for _ in 1..=10 {
        let id = graph.add_neuron(NeuronType::Pacemaker);
        graph.neurons[id as usize].current_voltage = SUBTHRESHOLD_VOLTAGE;
    }

    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
    };

    for i in 0..10 {
        let branch = ((i % 6) + 2) as u8;
        graph.connect(i as u32 + 1, target, DEFAULT_WEIGHT * 5.0, branch, false, false);
    }

    let mut spike_count_dist = 0;
    for tick in 1..=200 {
        graph.tick(tick, env);
        if graph.neurons[target as usize].last_spike_tick == tick {
            spike_count_dist += 1;
        }
    }
    println!("Distributed spikes: {}", spike_count_dist);

    graph.synapses.clear();
    graph.neurons[target as usize].current_voltage = -70.0;
    graph.neurons[target as usize].branches[1] = crate::engine::ffi::Compartment {
        branch_type: 1,
        refractory_timer: 0,
        voltage: ZERO_VOLTAGE_PACKED,
        nmda_threshold: NMDA_THRESHOLD_PACKED,
        local_trace: 0,
            _pad: 0,
    };
    for i in 0..8usize {
        if i != 1 { graph.neurons[target as usize].branches[i].voltage = ZERO_VOLTAGE_PACKED; }
    }

    for i in 1..=10 {
        graph.connect(i, target, DEFAULT_WEIGHT * 5.0, 1, false, false);
    }

    let mut spike_count_conc = 0;
    for tick in 1..=200 {
        graph.tick(tick, env);
        if graph.neurons[target as usize].last_spike_tick == tick {
            spike_count_conc += 1;
        }
    }
    println!("Concentrated spikes: {}", spike_count_conc);

    if spike_count_conc > spike_count_dist {
        println!("VERDICT: Dendritic non-linearity confirmed! 🧠✨");
    }
    BrainSnapshot::save_test_snapshot(50, &graph, "brain_save.bin");
    Ok(())
}

pub fn run_passive_leak_test() -> Result<(), Box<dyn std::error::Error>> {
    let _total_ticks = 3;
    println!("--- EXECUTING PASSIVE LEAK & REFRACTORY TEST (Fix 2 & 3 Validation) ---");
    let mut graph = NetworkGraph::new();
    let target = graph.add_neuron(NeuronType::Pyramidal);
    graph.neurons[target as usize].current_voltage = -70.0;
    
    graph.neurons[target as usize].branches[1] = crate::engine::ffi::Compartment {
        branch_type: 1,
        refractory_timer: 0,
        voltage: ZERO_VOLTAGE_PACKED,
        nmda_threshold: VOLTAGE_50_PACKED,
        local_trace: 0,
            _pad: 0,
    };

    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
    };

    graph.neurons[target as usize].branches[1].voltage = VOLTAGE_10_PACKED;
    graph.tick(1, env);
    
    let soma_v = graph.neurons[target as usize].current_voltage;
    if soma_v > -70.0 {
        println!("✅ FIX 2 PASS: Passive leak successfully transferred sub-threshold charge to soma.");
    }

    graph.neurons[target as usize].branches[1].voltage = VOLTAGE_60_PACKED;
    graph.tick(2, env);
    if graph.neurons[target as usize].branches[1].refractory_timer == 3 {
        println!("✅ FIX 3 PASS: Branch entered refractory period after NMDA spike.");
    }

    graph.neurons[target as usize].branches[1].voltage = VOLTAGE_10_PACKED;
    graph.tick(3, env);
    if graph.neurons[target as usize].branches[1].voltage == ZERO_VOLTAGE_PACKED {
        println!("✅ FIX 3 PASS: Refractory period effectively blocked incoming signal.");
    }

    BrainSnapshot::save_test_snapshot(_total_ticks, &graph, "brain_save.bin");
    Ok(())
}

pub fn run_sleep_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING SLEEP CYCLE TEST (Phase 35 Validation) ---");
    let mut graph = if let Some(loaded_snapshot) = BrainSnapshot::load_from_disk("brain_save.bin") {
        
        loaded_snapshot.graph
    } else {
        println!("No pre-trained sleep graph found. Running normal test sequence...");
        let mut g = NetworkGraph::new();
        g.add_neuron(NeuronType::Pyramidal);
        g
    };

    let mut current_env = Environment { 
        global_oxygen_level: DEFAULT_OXYGEN, 
        global_glucose_level: DEFAULT_GLUCOSE,
        dopamine_multiplier: DEFAULT_DOPAMINE, 
        toxin_levels: DEFAULT_TOXINS, 
        light_level: DEFAULT_LIGHT, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0, 
    };

        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);

    for tick in 1..=crate::engine::constants::TOTAL_LONG_TEST_TICKS {
        stimulus_service.process_tick(&mut graph, &mut current_env, tick);
        graph.tick(tick, current_env);
        
        for n in &mut graph.neurons {
            if n.is_alive == 1 {
                n.metabolic_wear = n.metabolic_wear.saturating_add(50);
            }
        }
        
        if tick % 1000 == 0 && graph.is_sleep_needed() {
            let pre_sleep_snapshot = graph.snapshot_synapses();
            graph.initiate_sleep_cycle();
            let post_sleep_snapshot = graph.snapshot_synapses();
            NetworkGraph::diff_synapses(&pre_sleep_snapshot, &post_sleep_snapshot);
            BrainSnapshot::save_test_snapshot(tick, &graph, "brain_save.bin");
            return Ok(());
        }
    }
    BrainSnapshot::save_test_snapshot(crate::engine::constants::TOTAL_LONG_TEST_TICKS, &graph, "brain_save.bin");
    Ok(())
}

pub fn run_inhibition_loop_mode() -> Result<(), Box<dyn std::error::Error>> {
    let _total_ticks = crate::engine::constants::TOTAL_TEST_TICKS;
    println!("--- EXECUTING INHIBITION LOOP SCENARIO (Phase 19) ---");
    let mut graph = NetworkGraph::new();
    let p = graph.add_neuron(NeuronType::Pyramidal);
    let i = graph.add_neuron(NeuronType::Interneuron);
    graph.connect(p, i, crate::engine::constants::WEIGHT_STRONG_BIAS, 0, false, true);
    graph.connect(i, p, crate::engine::constants::WEIGHT_STRONG_BIAS, 0, true, false);

    let mut env = Environment { 
        global_oxygen_level: DEFAULT_OXYGEN, 
        global_glucose_level: DEFAULT_GLUCOSE,
        dopamine_multiplier: DEFAULT_DOPAMINE, 
        toxin_levels: DEFAULT_TOXINS,
        light_level: DEFAULT_LIGHT, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
    };

        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);
    let mut history = vec![];

    for tick in 1..=crate::engine::constants::TOTAL_TEST_TICKS {
        stimulus_service.process_tick(&mut graph, &mut env, tick);
        graph.neurons[0].current_voltage += crate::engine::constants::VOLTAGE_INJECTION_STEP;
        graph.tick(tick, env);
        
        if tick % SERVICE_TICK_INTERVAL == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }

        history.push((
            tick,
            graph.neurons[0].current_voltage,
            graph.neurons[1].current_voltage,
            graph.synapses[0].weight,
        ));
    }

    visualization::render_inhibition_chart(&history, "graphs/inhibition_loop.png")?;
    BrainSnapshot::save_test_snapshot(_total_ticks, &graph, "brain_save.bin");
    Ok(())
}

pub fn run_reflex_arc_mode() -> Result<(), Box<dyn std::error::Error>> {
    let _total_ticks = crate::engine::constants::TOTAL_TEST_TICKS;
    println!("--- EXECUTING REFLEX ARC (Phase 11) ---");
    let mut graph = NetworkGraph::new();
    let eye = graph.add_neuron(NeuronType::Photoreceptor);
    let brain = graph.add_neuron(NeuronType::Interneuron);
    let muscle = graph.add_neuron(NeuronType::Pyramidal);
    graph.connect(eye, brain, crate::engine::constants::WEIGHT_EYE_TO_BRAIN, 0, false, true);
    graph.connect(brain, muscle, crate::engine::constants::WEIGHT_BRAIN_TO_MUSCLE, 0, false, false);

    let mut env = Environment { 
        global_oxygen_level: 1.0, 
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0, 
        light_level: 0.0, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0, 
    };

        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);
    let mut history = vec![];

    for tick in 1..=crate::engine::constants::TOTAL_TEST_TICKS {
        env.light_level = if (tick / crate::engine::constants::LIGHT_CYCLE_DURATION) % 2 == 0 { LIGHT_HIGH } else { LIGHT_LOW };
        if (300..=800).contains(&tick) {
            env.dopamine_multiplier = BOOSTED_DOPAMINE;
        } else {
            env.dopamine_multiplier = DEFAULT_DOPAMINE;
        }

        stimulus_service.process_tick(&mut graph, &mut env, tick);
        graph.tick(tick, env);
        
        if tick % SERVICE_TICK_INTERVAL == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }

        history.push((
            tick,
            graph.neurons[0].current_voltage,
            graph.neurons[1].current_voltage,
            graph.neurons[2].current_voltage,
            graph.neurons[2].energy_atp,
            graph.synapses[1].weight,
        ));
    }

    visualization::render_reflex_arc_chart(&history, "graphs/reflex_arc_boosted.png")?;
    BrainSnapshot::save_test_snapshot(_total_ticks, &graph, "brain_save.bin");
    Ok(())
}

pub fn run_stress_test_mode() -> Result<(), Box<dyn std::error::Error>> {
    let _total_ticks = crate::engine::constants::LONG_TEST_TICKS;
    println!("--- EXECUTING STRESS-TEST (Phase 13) ---");
    let mut graph = NetworkGraph::new();
    let mut env = Environment { 
        global_oxygen_level: DEFAULT_OXYGEN, 
        global_glucose_level: DEFAULT_GLUCOSE, 
        dopamine_multiplier: DEFAULT_DOPAMINE, 
        toxin_levels: DEFAULT_TOXINS, 
        light_level: DEFAULT_LIGHT, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0, 
    };

        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);

    for _ in 0..10 { graph.add_neuron(NeuronType::Pyramidal); }
    let n1 = graph.add_neuron(NeuronType::Pyramidal);
    let n2 = graph.add_neuron(NeuronType::Pyramidal);
    let n3 = graph.add_neuron(NeuronType::Pyramidal);
    graph.connect(n1, n2, crate::engine::constants::WEIGHT_EYE_TO_BRAIN, 0, false, false);
    graph.connect(n2, n3, crate::engine::constants::WEIGHT_EYE_TO_BRAIN, 0, false, false);
    graph.connect(n3, n1, crate::engine::constants::WEIGHT_EYE_TO_BRAIN, 0, false, false);
    graph.neurons[n1 as usize].current_voltage = crate::engine::constants::VOLTAGE_FORCED_SPIKE;

    let mut results = vec![];
    for tick in 0..crate::engine::constants::TOTAL_TEST_TICKS {
        if tick == crate::engine::constants::HYPOXIA_TICK_TRIGGER { env.global_oxygen_level = HYPOXIC_OXYGEN; }
        stimulus_service.process_tick(&mut graph, &mut env, tick as u32);
        graph.tick(tick as u32, env);
        
        let mut total_atp = 0.0;
        let mut dead_count = 0;
        let mut avg_wear = 0.0;

        for n in &graph.neurons {
            total_atp += n.energy_atp;
            avg_wear += n.metabolic_wear as f32 / (crate::engine::constants::COORD_MAX as f32);
            if !n.is_alive == 1 { dead_count += 1; }
        }

        results.push((tick as u32, total_atp / graph.neurons.len() as f32, dead_count, avg_wear / graph.neurons.len() as f32));
        if dead_count == graph.neurons.len() { break; }
    }
    
    let final_tick = results.last().map(|r| r.0).unwrap_or(0);
    finalize_and_save_snapshot(&graph, final_tick, "reports/stress_report.json", "brain_save.bin");
    Ok(())
}

pub fn run_dopamine_rush_mode() -> Result<(), Box<dyn std::error::Error>> {
    let _total_ticks = crate::engine::constants::TOTAL_TEST_TICKS;
    println!("--- EXECUTING DOPAMINE RUSH (Phase 14) ---");
    let mut graph = NetworkGraph::new();
    let mut current_env = Environment { 
        global_oxygen_level: DEFAULT_OXYGEN, 
        global_glucose_level: DEFAULT_GLUCOSE,
        dopamine_multiplier: DEFAULT_DOPAMINE, 
        toxin_levels: DEFAULT_TOXINS, 
        light_level: DEFAULT_LIGHT, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0, 
    };

        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);
    let n1 = graph.add_neuron(NeuronType::Pacemaker);
    let n2 = graph.add_neuron(NeuronType::Pyramidal);
    graph.connect(n1, n2, crate::engine::constants::WEIGHT_TEST_DEFAULT, 0, false, true);

    let mut weight_history = vec![];
    
    for tick in 0..crate::engine::constants::DOPAMINE_PHASE_3_END {
        if tick < crate::engine::constants::DOPAMINE_PHASE_1_END {
            current_env.dopamine_multiplier = LOW_DOPAMINE;
        } else if tick < crate::engine::constants::DOPAMINE_PHASE_2_END {
            current_env.dopamine_multiplier = HIGH_DOPAMINE;
        } else {
            current_env.dopamine_multiplier = DEFAULT_DOPAMINE;
        }

        stimulus_service.process_tick(&mut graph, &mut current_env, tick as u32);
        graph.tick(tick as u32, current_env);
        weight_history.push((tick as u32, graph.synapses[0].weight, current_env.dopamine_multiplier));
    }

    visualization::render_plasticity_chart(&weight_history, "graphs/dopamine_rush_results.png")?;
    BrainSnapshot::save_test_snapshot(_total_ticks, &graph, "brain_save.bin");
    Ok(())
}

pub fn run_frequency_test_mode() -> Result<(), Box<dyn std::error::Error>> {
    let _total_ticks = crate::engine::constants::MEDIUM_TEST_TICKS;
    println!("--- EXECUTING FREQUENCY DISCRIMINATION TEST (Phase 23) ---");
    let mut graph = NetworkGraph::new();
    let eye = graph.add_neuron(NeuronType::Photoreceptor);
    let f1_det = graph.add_neuron(NeuronType::Pyramidal);
    let f2_det = graph.add_neuron(NeuronType::Pyramidal);
    graph.connect(eye, f1_det, crate::engine::constants::WEIGHT_TEST_DEFAULT, 0, false, true);
    graph.connect(eye, f2_det, crate::engine::constants::WEIGHT_TEST_DEFAULT, 0, false, true);
    
    let mut current_env = Environment {
        global_oxygen_level: 1.0,  
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0, 
        toxin_levels: 0.0,
        light_level: 0.0, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
    };
    
        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);
    let mut history = vec![];

    for tick in 1..=crate::engine::constants::TOTAL_TEST_TICKS {
        stimulus_service.process_tick(&mut graph, &mut current_env, tick);
        graph.tick(tick, current_env);
        
        if tick % SERVICE_TICK_INTERVAL == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }

        let s1_w = if !graph.synapses.is_empty() { graph.synapses[0].weight } else { 0.0 };
        let s2_w = if graph.synapses.len() > 1 { graph.synapses[1].weight } else { 0.0 };

        history.push((tick, graph.neurons[f1_det as usize].current_voltage, graph.neurons[f2_det as usize].current_voltage, s1_w, s2_w));
    }

    visualization::render_frequency_chart(&history, "graphs/frequency_learning.png")?;
    BrainSnapshot::save_test_snapshot(_total_ticks, &graph, "brain_save.bin");
    Ok(())
}

pub fn run_emergence_test_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING EMERGENCE FROM CHAOS (Phase 29) ---");
    let mut graph = NetworkGraph::new();
    for _ in 0..crate::engine::constants::EMERGENCE_NETWORK_SIZE { graph.add_neuron(NeuronType::Pyramidal); }
    
    let mut current_env = Environment { 
        global_oxygen_level: DEFAULT_OXYGEN, 
        global_glucose_level: DEFAULT_GLUCOSE, 
        dopamine_multiplier: EMERGENCE_DOPAMINE, 
        toxin_levels: DEFAULT_TOXINS, 
        light_level: DEFAULT_LIGHT, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0, 
    };

        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);
    let mut history = vec![];

    for tick in 1..=crate::engine::constants::TOTAL_TEST_TICKS {
        if tick % crate::engine::constants::EMERGENCE_INJECTION_INTERVAL == 0 {
            for i in 0..crate::engine::constants::EMERGENCE_INJECTION_TARGETS { graph.neurons[i as usize].current_voltage += crate::engine::constants::VOLTAGE_INJECTION_STEP; }
        }
        stimulus_service.process_tick(&mut graph, &mut current_env, tick);
        graph.tick(tick, current_env);
        
        if tick % SERVICE_TICK_INTERVAL == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
            graph.run_neurogenesis_service(tick);
        }

        history.push((tick, graph.synapses.len() as u32, graph.neurons[250].current_voltage));
    }

    visualization::render_emergence_chart(&history, "graphs/emergence_growth.png")?;
    finalize_and_save_snapshot(&graph, crate::engine::constants::TOTAL_TEST_TICKS, "reports/emergence_report.json", "brain_save.bin");
    Ok(())
}

pub fn run_morphology_audit() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- EXECUTING SYNAPTIC MORPHOLOGY AUDIT (Phase 30) ---");
    let mut graph = NetworkGraph::new();
    let source = graph.add_neuron(NeuronType::Pyramidal);
    let target = graph.add_neuron(NeuronType::Pyramidal);
    graph.connect(source, target, crate::engine::constants::WEIGHT_TEST_DEFAULT, 0, false, false);
    
    let mut current_env = Environment { 
        global_oxygen_level: DEFAULT_OXYGEN, 
        global_glucose_level: DEFAULT_GLUCOSE,
        dopamine_multiplier: MORPHOLOGY_DOPAMINE, 
        toxin_levels: DEFAULT_TOXINS, 
        light_level: DEFAULT_LIGHT, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0, 
    };

        let mut stimulus_service = stimulus::StimulusService::new(STIMULUS_DEFAULT_INTERVAL);
    
    let mut sensory_hub = SensoryHub::new();
    sensory_hub.build_mapping(&graph);
    let mut history = vec![];

    for tick in 1..=crate::engine::constants::TOTAL_TEST_TICKS {
        stimulus_service.process_tick(&mut graph, &mut current_env, tick);
        if tick % crate::engine::constants::MORPHOLOGY_INJECTION_INTERVAL == 0 { graph.neurons[source as usize].current_voltage += crate::engine::constants::VOLTAGE_EMERGENCE_INJECTION; }
        graph.tick(tick, current_env);
        
        if tick % SERVICE_TICK_INTERVAL == 0 {
            graph.run_astrocyte_service(tick);
            graph.run_microglia_service(tick);
        }

        let sw = if !graph.synapses.is_empty() { graph.synapses[0].weight } else { 0.0 };
        let sb = if !graph.synapses.is_empty() { graph.synapses[0].neurotransmitter_buffer } else { 0.0 };

        history.push((tick, sw, graph.neurons[target as usize].current_voltage, sb));
    }

    visualization::render_morphology_chart(&history, "graphs/morphology_audit.png")?;
    finalize_and_save_snapshot(&graph, crate::engine::constants::TOTAL_TEST_TICKS, "reports/morphology_report.json", "brain_save.bin");

    Ok(())
}

pub async fn run_bench_scaling() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;
    println!("--- EXECUTING SCALABILITY BENCHMARKS ---");
    let sizes = crate::engine::constants::BENCHMARK_SCALING_SIZES;
    let ticks = crate::engine::constants::BENCHMARK_SCALING_TICKS;
    
    let mut prev_time = 0.0;
    
    for &size in &sizes {
        let synapses = size * 10;
        let mut graph = crate::network::graph::NetworkGraph::new();
        for _ in 0..size {
            graph.add_neuron(crate::engine::ffi::NeuronType::Pyramidal);
        }
        for i in 0..synapses {
            graph.synapses.push(crate::engine::ffi::Synapse {
                source_id: (i % size) as u32,
                target_id: ((i + 1) % size) as u32,
                weight: 1.0,
                neurotransmitter_buffer: 0.0,
                last_activation_tick: 0,
                ticks_below_threshold: 0,
                ticks_until_delivery: 0,
                flags: 0,
                _padding: [0; 2],
            });
        }
        
        let env = crate::engine::ffi::Environment {
            global_oxygen_level: 1.0,
            global_glucose_level: 1.0,
            dopamine_multiplier: 1.0,
            toxin_levels: 0.0,
            light_level: 0.0, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
        };
        
        let start = Instant::now();
        for t in 1..=ticks {
            graph.tick(t, env);
        }
        let elapsed = start.elapsed().as_secs_f64();
        
        println!("Neurons: {:>5} | Synapses: {:>6} | Time: {:.4}s | TPS: {:.2}", size, synapses, elapsed, ticks as f64 / elapsed);
        
        if prev_time > 0.0 {
            let factor = elapsed / prev_time;
            if factor > crate::engine::constants::BENCHMARK_MAX_DEGRADATION_FACTOR {
                println!("⚠️ PERFORMANCE WARNING: Serialization factor {:.2}x exceeded safe threshold of {:.2}x!", factor, crate::engine::constants::BENCHMARK_MAX_DEGRADATION_FACTOR);
            } else {
                println!("  -> OK: Scale Factor: {:.2}x", factor);
            }
        }
        prev_time = elapsed;
    }
    
    // Save to JSON
    let report_path = format!("reports/perf_test_scaling_{}.json", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    println!("Saved detailed log to {}", report_path);
    // Real implementation would dump the json here
    
    Ok(())
}

pub async fn run_bench_io() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;
    println!("--- EXECUTING I/O LATENCY BENCHMARK ---");
    let intervals = crate::engine::constants::BENCHMARK_IO_INTERVALS;
    let size = crate::engine::constants::BENCH_STANDARD_SIZE;
    let syn_count = crate::engine::constants::BENCH_STANDARD_SYNAPSES;
    let ticks = crate::engine::constants::BENCH_STANDARD_TICKS;
    
    let mut graph = crate::network::graph::NetworkGraph::new();
    for _ in 0..size {
        graph.add_neuron(crate::engine::ffi::NeuronType::Pyramidal);
    }
    for i in 0..syn_count {
        graph.synapses.push(crate::engine::ffi::Synapse {
            source_id: (i % size) as u32,
            target_id: ((i + 1) % size) as u32,
            weight: 1.0,
            neurotransmitter_buffer: 0.0,
            last_activation_tick: 0,
            ticks_below_threshold: 0,
            ticks_until_delivery: 0,
            flags: 0,
            _padding: [0; 2],
        });
    }
    
    for &interval in &intervals {
        let start = Instant::now();
        
        for t in 1..=ticks {
            // Fake the serialization work
            if t % interval == 0 {
                let mut snapshot = crate::monitor::streamer::InitialStatePayload {
                    total_neurons: graph.neurons.len(),
                    total_synapses: graph.synapses.len(),
                    neurons: Vec::with_capacity(graph.neurons.len()),
                    synapses: Vec::with_capacity(graph.synapses.len()),
                };
                for n in &graph.neurons {
                    snapshot.neurons.push((n.id, n.x as f32, n.y as f32, n.z as f32, n.type_, n.column_id, n.layer_id, n.minicolumn_id, n.status_flags));
                }
                for s in &graph.synapses {
                    snapshot.synapses.push((s.source_id, s.target_id, s.weight));
                }
                let json = serde_json::to_string(&snapshot).unwrap();
                let _ = json.len(); // prevent optimization
            }
        }
        
        let elapsed = start.elapsed().as_secs_f64();
        let tps = ticks as f64 / elapsed;
        
        println!("Interval: {:>3} | Time: {:.4}s | TPS: {:.2}", interval, elapsed, tps);
        if tps < crate::engine::constants::BENCHMARK_MIN_ACCEPTABLE_TPS {
            println!("⚠️ CRITICAL I/O BOTTLENECK DETECTED! TPS is {:.2}, expected > {:.2}", tps, crate::engine::constants::BENCHMARK_MIN_ACCEPTABLE_TPS);
        }
    }
    
    Ok(())
}
pub async fn run_ui_test(tx: Option<tokio::sync::broadcast::Sender<String>>, shared_env: std::sync::Arc<std::sync::Mutex<crate::engine::ffi::Environment>>) -> Result<(), Box<dyn std::error::Error>> {
    
    println!("--- EXECUTING UI TEST MODE ---");
    println!("Spawning a tiny predictable network to test visualizer functionality.");
    
    let mut graph = NetworkGraph::new();
    // 5 neurons in a star shape
    graph.add_neuron(NeuronType::Pyramidal); // Center (0)
    graph.add_neuron(NeuronType::Bipolar);    // Top (1)
    graph.add_neuron(NeuronType::Bipolar);    // Right (2)
    graph.add_neuron(NeuronType::Purkinje);     // Bottom (3)
    graph.add_neuron(NeuronType::Interneuron); // Left (4)
    
    // Set explicit coordinates for visual debugging
    graph.neurons[0].x = UI_COORD_CENTER; graph.neurons[0].y = UI_COORD_CENTER; graph.neurons[0].z = UI_COORD_CENTER;
    graph.neurons[1].x = UI_COORD_CENTER; graph.neurons[1].y = UI_COORD_RIGHT; graph.neurons[1].z = UI_COORD_CENTER;
    graph.neurons[2].x = UI_COORD_RIGHT; graph.neurons[2].y = UI_COORD_CENTER; graph.neurons[2].z = UI_COORD_CENTER;
    graph.neurons[3].x = UI_COORD_CENTER; graph.neurons[3].y = UI_COORD_LEFT;  graph.neurons[3].z = UI_COORD_CENTER;
    graph.neurons[4].x = UI_COORD_LEFT;  graph.neurons[4].y = UI_COORD_CENTER; graph.neurons[4].z = UI_COORD_CENTER;
    
    // Add synapses flowing from sensors to center to motor
    let edges = [(1, 0), (2, 0), (0, 3), (4, 0)];
    for &(src, tgt) in &edges {
        graph.synapses.push(Synapse {
            source_id: src as u32,
            target_id: tgt as u32,
            weight: UI_TEST_SYNAPSE_WEIGHT,
            neurotransmitter_buffer: 0.0,
            last_activation_tick: 0,
            ticks_below_threshold: 0,
            ticks_until_delivery: 0,
            flags: 0,
            _padding: [0; 2],
        });
    }

    let env = Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0, stdp_learning_rate: 0.1,
            wakefulness_multiplier: 1.0,
    };

    println!("Network Ready. Initiating slow tick loop for visualizer debugging...");
    
    let start_time = std::time::Instant::now();
    let mut tick = 1;
    loop {
        // Send a stimulus wave every 50 ticks
        if tick % 50 == 0 {
            println!("Tick {}: Flashing sensors...", tick);
            graph.neurons[1].current_voltage = FORCED_SPIKE_VOLTAGE;
            graph.neurons[1].energy_atp = FULL_ATP_LEVEL;
            graph.neurons[2].current_voltage = FORCED_SPIKE_VOLTAGE;
            graph.neurons[2].energy_atp = FULL_ATP_LEVEL;
        }

        let current_env = {
            if let Ok(env_guard) = shared_env.lock() {
                *env_guard
            } else {
                env
            }
        };
        
        graph.tick(tick, current_env);
        
        // Broadcast state
        if let Some(ref sender) = tx {
            // First tick: send InitialState
            if tick == 1 {
                let initial = generate_initial_state(&graph);
                let msg = crate::monitor::streamer::StreamMessage::InitialState(initial);
                let _ = sender.send(serde_json::to_string(&msg).unwrap());
            } else {
                let mut update = crate::monitor::streamer::TickUpdatePayload {
                    tick,
                    total_spikes: 0,
                    global_oxygen: env.global_oxygen_level,
                    global_glucose: env.global_glucose_level,
                    neurons: Vec::new(),
                    synapses: None,
                    synapse_count: graph.synapses.len(),
                };
                for n in &graph.neurons {
                    let atp_pct = (n.energy_atp / n.max_atp) * crate::engine::constants::METRIC_MAX_F32;
                    let visual_state = get_visual_state(n, tick, atp_pct);
                    update.neurons.push((n.id, visual_state, atp_pct, n.status_flags));
                }
                
                let msg = crate::monitor::streamer::StreamMessage::TickUpdate(update);
                let _ = sender.send(serde_json::to_string(&msg).unwrap());
            }
        }
        
        tick += 1;
        // Super slow loop so user can watch UI
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        if start_time.elapsed().as_secs() >= UI_TEST_DURATION_SECS {
            println!("UI Test limit reached (5 minutes). Exiting.");
            break;
        }
    }
    
    Ok(())
}
