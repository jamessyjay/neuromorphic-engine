use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType, Synapse};
use std::time::Instant;

fn default_env() -> Environment {
    Environment {
        global_oxygen_level: 1.0,
        global_glucose_level: 1.0,
        dopamine_multiplier: 1.0,
        toxin_levels: 0.0,
        light_level: 0.0, stdp_learning_rate: 0.1,
    }
}

// 36. Test "The Pure Grid"
#[test]
fn test_pure_grid_neurogenesis() {
    let sizes = [1000, 2000, 4000, 8000];
    let mut prev_time = 0.0;
    
    println!("--- The Pure Grid (Neurogenesis O(n^2) Check) ---");
    for &size in &sizes {
        let mut graph = NetworkGraph::new();
        for _ in 0..size {
            graph.add_neuron(NeuronType::Pyramidal);
        }
        
        // Warm up and average multiple runs to avoid OS scheduling noise
        let iters = 5;
        let start = Instant::now();
        for _ in 0..iters {
            graph.run_neurogenesis_service(500); 
        }
        let elapsed = (start.elapsed().as_secs_f64() * 1000.0) / iters as f64; // in ms
        
        println!("Neurons: {:>4} | Time: {:.4} ms", size, elapsed);
        
        if prev_time > 0.0 {
            let factor = elapsed / prev_time;
            // Since we scale size by 2x, expected time scale should be ~2x (linear).
            // O(n^2) would be ~4x. We use 3.5 to account for cache overhead or small fluctuations,
            // but anything near 4.0 is bad. 100->500 had 5x size change, so factor 5x was actually linear,
            // which caused the panic (4.97 factor on 5x size). Now we strictly do 2x steps.
            assert!(factor < 3.5, "Neurogenesis is scaling non-linearly! Factor: {:.2}x for 2x scale", factor);
        }
        prev_time = elapsed;
    }
}

// 37. Test "Synaptic Saturation"
#[test]
fn test_synaptic_saturation() {
    let size = 500;
    let syn_counts = [1_000, 10_000, 50_000, 100_000];
    let env = default_env();
    
    println!("--- Synaptic Saturation (Memory Bandwidth Check) ---");
    let mut prev_tps = 0.0;
    for &synapses in &syn_counts {
        let mut graph = NetworkGraph::new();
        for _ in 0..size {
            graph.add_neuron(NeuronType::Pyramidal);
        }
        
        for i in 0..synapses {
            graph.synapses.push(Synapse {
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
        
        let ticks = 50;
        let start = Instant::now();
        for t in 1..=ticks {
            graph.tick(t, env.clone());
        }
        let elapsed = start.elapsed().as_secs_f64();
        let tps = ticks as f64 / elapsed;
        
        println!("Synapses: {:>6} | TPS: {:.2}", synapses, tps);
        assert!(tps > 0.0);
        
        if prev_tps > 0.0 {
            let drop = prev_tps / tps;
            if drop > 5.0 {
                println!("  [!] CACHE MISS CLIFF DETECTED! Drop factor: {:.2}", drop);
            }
        }
        prev_tps = tps;
    }
}

// 38. Test "Dendritic Branching Stress"
#[test]
fn test_dendritic_branching_stress() {
    let size = 1000;
    let synapses = 10_000;
    let branch_counts = [1, 4, 8];
    let env = default_env();
    
    println!("--- Dendritic Branching Stress ---");
    let mut times = vec![];
    
    for &b_count in &branch_counts {
        let mut graph = NetworkGraph::new();
        for _ in 0..size {
            graph.add_neuron(NeuronType::Pyramidal);
        }
        
        for i in 0..synapses {
            let branch_id = (i % b_count) as u8;
            graph.synapses.push(Synapse {
                source_id: (i % size) as u32,
                target_id: ((i + 1) % size) as u32,
                weight: 1.0,
                neurotransmitter_buffer: 0.0,
                last_activation_tick: 0,
                ticks_below_threshold: 0,
                ticks_until_delivery: 0,
                flags: branch_id,
                _padding: [0; 2],
            });
        }
        
        let ticks = 50;
        let start = Instant::now();
        for t in 1..=ticks {
            graph.tick(t, env.clone());
        }
        let elapsed = start.elapsed().as_secs_f64();
        
        println!("Active Branches per Neuron: {} | TPS: {:.2}", b_count, ticks as f64 / elapsed);
        times.push(elapsed);
    }
    
    let overhead = times[2] / times[0];
    println!("Overhead of 8 branches vs 1: {:.2}x", overhead);
    assert!(overhead < 3.0, "Dendritic computation overhead is too high!");
}

// 39. Test "Serialization Backpressure"
#[test]
fn test_serialization_backpressure() {
    use neuromorphic_engine::monitor::streamer;
    let size = 1000;
    let syn_count = 5000;
    let intervals = [100, 50, 10, 1];
    
    let mut graph = NetworkGraph::new();
    for _ in 0..size { graph.add_neuron(NeuronType::Pyramidal); }
    for i in 0..syn_count {
        graph.synapses.push(Synapse {
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
    
    println!("--- Serialization Backpressure (JSON Killer) ---");
    for &interval in &intervals {
        let ticks = 100;
        let start = Instant::now();
        
        for t in 1..=ticks {
            if t % interval == 0 {
                let mut snapshot = streamer::InitialStatePayload {
                    total_neurons: graph.neurons.len(),
                    total_synapses: graph.synapses.len(),
                    neurons: Vec::with_capacity(graph.neurons.len()),
                    synapses: Vec::with_capacity(graph.synapses.len()),
                };
                for n in &graph.neurons {
                    snapshot.neurons.push((n.id, n.x as f32, n.y as f32, n.z as f32, n.type_ as u8));
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
        
        println!("Interval: {:>3} | TPS: {:.2}", interval, tps);
    }
}

// 40. Test "Idle Drift"
#[test]
fn test_idle_drift() {
    let size = 10_000;
    let mut graph = NetworkGraph::new();
    for _ in 0..size {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    let env = default_env();
    
    let ticks = 100;
    let start = Instant::now();
    for t in 1..=ticks {
        graph.tick(t, env.clone());
    }
    let elapsed = start.elapsed().as_secs_f64();
    let tps = ticks as f64 / elapsed;
    
    println!("--- Idle Drift (Baseline Cost) ---");
    println!("10k empty neurons TPS: {:.2}", tps);
    assert!(tps > 1000.0, "Baseline overhead without synapses is too high!");
}
