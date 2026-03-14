use neuromorphic_engine::network::graph::NetworkGraph;
use neuromorphic_engine::engine::ffi::{Environment, NeuronType, Synapse};
use neuromorphic_engine::monitor::streamer;
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

// 20. Test "Linear Scaling (Neurons)"
#[test]
fn test_linear_scaling_neurons() {
    let sizes = [1_000, 5_000, 10_000, 50_000];
    let ticks = 50;
    
    println!("--- Linear Scaling (Neurons) TPS ---");
    for &size in &sizes {
        let mut graph = NetworkGraph::new();
        for _ in 0..size {
            graph.add_neuron(NeuronType::Pyramidal);
        }
        // Add exactly 10 synapses per neuron
        for i in 0..size {
            for j in 1..=10 {
                let target = (i + j) % size;
                graph.synapses.push(Synapse {
                    source_id: i as u32,
                    target_id: target as u32,
                    weight: 1.0,
                    neurotransmitter_buffer: 0.0,
                    last_activation_tick: 0,
                    ticks_below_threshold: 0,
                    ticks_until_delivery: 0,
                    flags: 0,
                    _padding: [0; 2],
                });
            }
        }
        
        let env = default_env();
        let start = Instant::now();
        for t in 1..=ticks {
            graph.tick(t, env.clone());
        }
        let elapsed = start.elapsed().as_secs_f64();
        let tps = ticks as f64 / elapsed;
        
        println!("Neurons: {:>6}, Synapses: {:>7} | TPS: {:.2}", size, size * 10, tps);
        assert!(tps > 0.0);
    }
}

// 21. Test "Connectivity Explosion"
#[test]
fn test_connectivity_explosion() {
    let size = 1000;
    let synapse_counts = [10_000, 100_000, 1_000_000];
    let ticks = 50;
    
    println!("--- Connectivity Explosion TPS ---");
    for &syn_count in &synapse_counts {
        let mut graph = NetworkGraph::new();
        for _ in 0..size {
            graph.add_neuron(NeuronType::Pyramidal);
        }
        
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
        
        let env = default_env();
        let start = Instant::now();
        for t in 1..=ticks {
            graph.tick(t, env.clone());
        }
        let elapsed = start.elapsed().as_secs_f64();
        let tps = ticks as f64 / elapsed;
        
        println!("Neurons: 1000, Synapses: {:>9} | TPS: {:.2}", syn_count, tps);
        assert!(tps > 0.0);
    }
}

// 22. Test "FFI Overhead"
#[test]
fn test_ffi_overhead() {
    let size = 10_000;
    let mut graph = NetworkGraph::new();
    for _ in 0..size {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    let env = default_env();
    
    let ticks = 100;
    let start_actual = Instant::now();
    for t in 1..=ticks {
        graph.tick(t, env.clone());
    }
    let elapsed_actual = start_actual.elapsed().as_secs_f64();
    
    println!("FFI + Rayon overhead for 10k neurons over {} ticks: {:.4} seconds", ticks, elapsed_actual);
    assert!(elapsed_actual < 5.0, "FFI overhead is too high");
}

// 23. Test "JSON Saturation"
#[test]
fn test_json_saturation() {
    let sizes = [1_000, 10_000, 50_000];
    
    println!("--- JSON Saturation ---");
    for &size in &sizes {
        let mut graph = NetworkGraph::new();
        for _ in 0..size {
            graph.add_neuron(NeuronType::Pyramidal);
        }
        
        for i in 0..size {
            for j in 1..=10 {
                let target = (i + j) % size;
                graph.synapses.push(Synapse {
                    source_id: i as u32,
                    target_id: target as u32,
                    weight: 1.0,
                    neurotransmitter_buffer: 0.0,
                    last_activation_tick: 0,
                    ticks_below_threshold: 0,
                    ticks_until_delivery: 0,
                    flags: 0,
                    _padding: [0; 2],
                });
            }
        }
        
        let start = Instant::now();
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
        let build_time = start.elapsed();
        
        let serialize_start = Instant::now();
        let json = serde_json::to_string(&snapshot).unwrap();
        let serialize_time = serialize_start.elapsed();
        
        println!("Size: {} neurons | JSON Build: {:.2?} | JSON Serialize: {:.2?} | Payload Size: {} bytes", 
            size, build_time, serialize_time, json.len());
        
        assert!(!json.is_empty());
    }
}

// 24. Test "Memory Footprint"
#[test]
fn test_memory_footprint() {
    let size = 100_000;
    let mut graph = NetworkGraph::new();
    for _ in 0..size {
        graph.add_neuron(NeuronType::Pyramidal);
    }
    let env = default_env();
    graph.tick(1, env);
    
    assert_eq!(graph.neurons.len(), 100_000);
}

// 25. Test "Double Buffer Sync"
#[test]
fn test_double_buffer_sync() {
    let mut graph = NetworkGraph::new();
    let src = graph.add_neuron(NeuronType::Pyramidal);
    let tgt = graph.add_neuron(NeuronType::Pyramidal);
    let env = default_env();
    
    graph.synapses.push(Synapse {
        source_id: src,
        target_id: tgt,
        weight: 1.0,
        neurotransmitter_buffer: 0.0,
        last_activation_tick: 0,
        ticks_below_threshold: 0,
        ticks_until_delivery: 0, 
        flags: 0,
        _padding: [0; 2],
    });
    
    // Force spike on source
    graph.neurons[src as usize].current_voltage = 40.0;
    graph.neurons[src as usize].last_spike_tick = 1;
    
    // Tick 1
    graph.tick(1, env.clone());
    
    // Since tick_synapses runs after tick_neurons, it should see the spike and add neurotransmitters
    assert!(graph.synapses[0].neurotransmitter_buffer > 0.0, "Synapse should pick up the spike in the same tick cycle");
}
